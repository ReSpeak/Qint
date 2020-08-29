use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actix::*;
use anyhow::{format_err, Result};
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, warn, Logger};
use tokio::runtime::Handle;
use tsclientlib::ClientId;
use tsproto_packets::packets::InAudioBuf;

use super::*;
use crate::websocket::{GetClientVolumeMsg, TalkersChangedMsg, Ws};
use crate::ConnectionId;

type Id = (ConnectionId, ClientId);
type AudioHandler = tsclientlib::audio::AudioHandler<Id>;

pub struct PlayMsg(pub Id, pub InAudioBuf);
pub struct SetVolumeMsg(pub Id, pub f32);
pub struct ResetMsg;

pub(crate) struct TsToAudio {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	device: Option<AudioDevice<SdlCallback>>,
	data: Arc<Mutex<AudioHandler>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
}

struct SdlCallback {
	data: Arc<Mutex<AudioHandler>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	handle: Handle,
}

impl Message for PlayMsg {
	type Result = Result<()>;
}
impl Message for SetVolumeMsg {
	type Result = Result<()>;
}
impl Message for ResetMsg {
	type Result = ();
}

impl Actor for TsToAudio {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.open_playback();

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			// Restart on errors
			if t2a.device.as_ref().map(|d| d.status() == AudioStatus::Stopped).unwrap_or(true) {
				// Try to reconnect to audio
				t2a.open_playback();
			}

			if let Some(device) = &t2a.device {
				let data_empty = t2a.data.lock().unwrap().get_queues().is_empty();
				if device.status() == AudioStatus::Paused && !data_empty {
					debug!(t2a.logger, "Resuming playback");
					device.resume();
				} else if device.status() == AudioStatus::Playing && data_empty {
					debug!(t2a.logger, "Pausing playback");
					device.pause();
				}
			}
		});
	}
}

impl TsToAudio {
	pub(crate) fn new(
		logger: Logger, audio_subsystem: AudioSubsystem,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	) -> Result<Self>
	{
		let logger = logger.new(o!("pipeline" => "ts-to-audio"));
		let data = Arc::new(Mutex::new(AudioHandler::new(logger.clone())));

		Ok(Self { logger, audio_subsystem, device: None, data, connections })
	}

	fn open_playback(&mut self) {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(2),
			samples: Some(USUAL_SAMPLE_COUNT as u16),
		};

		let logger = self.logger.clone();
		let data = self.data.clone();
		let connections = self.connections.clone();
		match self.audio_subsystem.open_playback(None, &desired_spec, |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			debug!(logger, "Got playback spec"; "spec" => ?spec, "driver" => self.audio_subsystem.current_audio_driver());
			SdlCallback { data, connections, handle: Handle::current() }
		}) {
			Ok(device) => self.device = Some(device),
			Err(e) => {
				self.device = None;
				error!(self.logger, "Failed to open playback device"; "error" => %e);
			}
		}
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<()>;
	fn handle(&mut self, PlayMsg(id, packet): PlayMsg, ctx: &mut Self::Context) -> Self::Result {
		if let Some(device) = &self.device {
			let mut data = self.data.lock().unwrap();
			if let Some(new_id) = data.handle_packet(id, packet)? {
				let cons = self.connections.lock().unwrap();
				let talkers = data
					.get_queues()
					.iter()
					.filter_map(|((con, client), queue)| {
						if *con == new_id.0 { Some((*client, queue.is_whispering())) } else { None }
					})
					.collect();
				if let Some(con) = cons.get(&new_id.0) {
					actix::spawn(con.send(TalkersChangedMsg(talkers)).map(|_| ()));

					// Get the volume of the new talker
					ctx.spawn(fut::wrap_future(con.send(GetClientVolumeMsg(new_id.1))).map(
						move |v, this: &mut Self, _| match v {
							Ok(Ok(v)) => {
								let mut data = this.data.lock().unwrap();
								if let Some(q) = data.get_mut_queues().get_mut(&new_id) {
									q.volume = v;
								}
							}
							Ok(Err(e)) => {
								warn!(this.logger, "Failed to get volume for \
									client"; "error" => %e);
							}
							Err(e) => {
								warn!(this.logger, "Failed to get volume for \
									client"; "error" => %e);
							}
						},
					));
				}
			}

			if device.status() == AudioStatus::Paused {
				debug!(self.logger, "Resuming playback");
				device.resume();
			}
		}
		Ok(())
	}
}

impl Handler<SetVolumeMsg> for TsToAudio {
	type Result = Result<()>;
	fn handle(
		&mut self, SetVolumeMsg(id, volume): SetVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		let mut data = self.data.lock().unwrap();
		if let Some(queue) = data.get_mut_queues().get_mut(&id) {
			queue.volume = volume;
			Ok(())
		} else {
			Err(format_err!("Client not found"))
		}
	}
}

impl Handler<ResetMsg> for TsToAudio {
	type Result = ();
	fn handle(&mut self, _: ResetMsg, _: &mut Self::Context) -> Self::Result {
		self.open_playback();
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		// Clear buffer
		for d in &mut *buffer {
			*d = 0.0;
		}

		let mut data = self.data.lock().unwrap();
		let removed = data.fill_buffer(buffer);
		let message_connections = removed.iter().map(|i| i.0).collect::<HashSet<_>>();
		if !message_connections.is_empty() {
			let cons = self.connections.lock().unwrap();
			for c in message_connections {
				let talkers = data
					.get_queues()
					.iter()
					.filter_map(|((con, client), queue)| {
						if *con == c { Some((*client, queue.is_whispering())) } else { None }
					})
					.collect();
				if let Some(con) = cons.get(&c) {
					self.handle.spawn(con.send(TalkersChangedMsg(talkers)).map(|_| ()));
				}
			}
		}
	}
}
