use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use actix::*;
use anyhow::{format_err, Result};
use ebur128::EbuR128;
use oboe::{
	AudioOutputCallback, AudioOutputStream, AudioOutputStreamSafe, AudioStream, AudioStreamAsync,
	AudioStreamBase, AudioStreamBuilder, AudioStreamSafe, DataCallbackResult, DefaultStreamValues,
	Mono, Output, PerformanceMode, SharingMode, Stereo, StreamState,
};
use tokio::runtime::Handle;
use tracing::{debug, error, info_span, warn, Span};
use tsclientlib::ClientId;
use tsproto_packets::packets::InAudioBuf;

use super::*;
use crate::connection::{GetClientVolumeMsg, LoudnessesMsg, QintConnection, TalkersChangedMsg};
use crate::ConnectionId;

type Id = (ConnectionId, ClientId);
type AudioHandler = tsclientlib::audio::AudioHandler<Id>;

pub struct PlayMsg(pub Id, pub InAudioBuf);
pub struct SetGlobalVolumeMsg(pub f32);
pub struct SetVolumeMsg(pub Id, pub f32);

pub struct TsToAudio {
	preferred_device: Option<String>,
	stream: Option<AudioStreamAsync<Output, OboeCallback>>,
	data: Arc<Mutex<AudioHandler>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
	/// The global volume to multiply all output with.
	///
	/// This is actually a `f32`, there is no `AtomicF32` though.
	global_volume: Arc<AtomicU32>,
}

struct OboeCallback {
	span: Span,
	data: Arc<Mutex<AudioHandler>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
	loudness: HashMap<Id, EbuR128>,
	handle: Handle,
	global_volume: Arc<AtomicU32>,
}

impl Message for PlayMsg {
	type Result = Result<()>;
}
impl Message for SetGlobalVolumeMsg {
	type Result = ();
}
impl Message for SetVolumeMsg {
	type Result = Result<()>;
}

impl Actor for TsToAudio {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.open_playback();

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			// Restart on errors
			/*if t2a.stream.as_ref().map(|d| d.get_state() == AudioStatus::Stopped).unwrap_or(true) {
				// Try to reconnect to audio
				t2a.open_playback();
			}*/

			if let Some(stream) = &mut t2a.stream {
				let data_empty = t2a.data.lock().unwrap().get_queues().is_empty();
				let state = stream.get_state();
				if state != StreamState::Starting && state != StreamState::Started && !data_empty {
					debug!("Resuming playback");
					stream.request_start();
				} else if (state == StreamState::Starting || state == StreamState::Started)
					&& data_empty
				{
					debug!("Pausing playback");
					stream.request_pause();
				}
			}
		});
	}
}

impl TsToAudio {
	pub(crate) fn new(
		preferred_device: Option<String>,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>, global_volume: f32,
	) -> Result<Self> {
		let data = Arc::new(Mutex::new(AudioHandler::new()));

		Ok(Self {
			preferred_device,
			stream: None,
			data,
			connections,
			global_volume: Arc::new(AtomicU32::new(global_volume.to_bits())),
		})
	}

	fn open_playback(&mut self) {
		let callback = OboeCallback {
			span: info_span!("ts-to-audio"),
			data: self.data.clone(),
			connections: self.connections.clone(),
			loudness: Default::default(),
			handle: Handle::current(),
			global_volume: self.global_volume.clone(),
		};

		match AudioStreamBuilder::default()
			.set_performance_mode(PerformanceMode::LowLatency)
			.set_sharing_mode(SharingMode::Shared)
			.set_format::<f32>()
			.set_channel_count::<Stereo>()
			.set_sample_rate(48000)
			.set_buffer_capacity_in_frames(USUAL_SAMPLE_COUNT as i32)
			.set_callback(callback)
			.open_stream()
		{
			Ok(mut stream) => {
				if let Err(error) = stream.start() {
					error!(%error, "Failed to start playback stream");
				}
				self.stream = Some(stream);
			}
			Err(error) => {
				self.stream = None;
				error!(%error, "Failed to open playback stream");
			}
		}
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<()>;
	fn handle(&mut self, PlayMsg(id, packet): PlayMsg, ctx: &mut Self::Context) -> Self::Result {
		if let Some(stream) = &mut self.stream {
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
							Ok(Err(error)) => {
								warn!(%error, "Failed to get volume for client");
							}
							Err(error) => {
								warn!(%error, "Failed to get volume for client");
							}
						},
					));
				}
			}

			let state = stream.get_state();
			if state != StreamState::Starting && state != StreamState::Started {
				debug!("Resuming playback");
				stream.request_start();
			}
		}
		Ok(())
	}
}

impl Handler<SetGlobalVolumeMsg> for TsToAudio {
	type Result = ();
	fn handle(
		&mut self, SetGlobalVolumeMsg(volume): SetGlobalVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.global_volume.store(volume.to_bits(), Ordering::Relaxed);
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

impl Handler<GetAudioDevices> for TsToAudio {
	type Result = Vec<String>;
	fn handle(&mut self, _: GetAudioDevices, _: &mut Self::Context) -> Self::Result {
		let mut devices = Vec::new();
		/*if let Some(dev_cnt) = self.audio_subsystem.num_audio_playback_devices() {
			for dev_index in 0..dev_cnt {
				if let Ok(dev_name) = self.audio_subsystem.audio_playback_device_name(dev_index) {
					devices.push(dev_name);
				}
			}
		}*/
		devices
	}
}

impl Handler<SetAudioDevice> for TsToAudio {
	type Result = ();
	fn handle(&mut self, set: SetAudioDevice, _: &mut Self::Context) -> Self::Result {
		if self.preferred_device != set.0 {
			self.preferred_device = set.0;
			self.open_playback();
		}
	}
}

impl AudioOutputCallback for OboeCallback {
	type FrameType = (f32, Stereo);
	fn on_audio_ready(
		&mut self, stream: &mut dyn AudioOutputStreamSafe, buffer: &mut [(f32, f32)],
	) -> DataCallbackResult {
		let buffer: &mut [f32] = unsafe {
			std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut f32, buffer.len() * 2)
		};
		let _span = self.span.enter();
		// Clear buffer
		for d in &mut *buffer {
			*d = 0.0;
		}

		let mut data = self.data.lock().unwrap();
		let has_connections = !data.get_queues().is_empty();
		let loudness = &mut self.loudness;
		// Collect loudness per client per connection
		let mut loudnesses = HashMap::new();
		let removed = data.fill_buffer_with_proc(buffer, |id, buf| {
			let ebur128 = match loudness.entry(id.clone()) {
				Entry::Occupied(e) => e.into_mut(),
				Entry::Vacant(e) => {
					match EbuR128::new(2, super::SAMPLE_RATE as u32, ebur128::Mode::M) {
						Err(error) => {
							warn!(%error, "Failed to create loudness measurement");
							return;
						}
						Ok(r) => e.insert(r),
					}
				}
			};
			if let Err(error) = ebur128.add_frames_f32(buf) {
				warn!(%error, "Failed to measure loudness of client");
			}
			match ebur128.loudness_momentary() {
				Err(error) => warn!(%error, "Failed to measure loudness"),
				Ok(lufs) => {
					let ls = loudnesses.entry(id.0).or_insert_with(HashMap::new);
					ls.insert(id.1, lufs);
				}
			}
		});
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

		// Send loudnesses
		if !loudnesses.is_empty() {
			let cons = self.connections.lock().unwrap();
			for (c, ls) in loudnesses {
				if let Some(con) = cons.get(&c) {
					self.handle.spawn(con.send(LoudnessesMsg(ls)).map(|_| ()));
				}
			}
		}

		for id in &removed {
			self.loudness.remove(&id);
		}

		// Adjust with global volume
		let global_volume = f32::from_bits(self.global_volume.load(Ordering::Relaxed));
		if global_volume != 1.0 && has_connections {
			for d in &mut *buffer {
				*d *= global_volume;
			}
		}

		DataCallbackResult::Continue
	}
}
