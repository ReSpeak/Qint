use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use anyhow::{Result, format_err};
use ebur128::EbuR128;
use tokio::runtime::Handle;
use tracing::{Span, info_span, trace, warn};
use tsclientlib::ClientId;
use tsproto_packets::packets::InAudioBuf;

use super::*;
use crate::ConnectionId;
use crate::connection::{GetClientVolumeMsg, LoudnessesMsg, QintConnection, TalkersChangedMsg};

type Id = (ConnectionId, ClientId);
type AudioHandler = tsclientlib::audio::AudioHandler<Id>;

pub struct PlayMsg(pub Id, pub InAudioBuf);
pub struct SetGlobalVolumeMsg(pub f32);
pub struct SetVolumeMsg(pub Id, pub f32);

pub trait TsToAudioImpl: Unpin {
	fn started(ts_to_audio: &mut TsToAudio<Self>, ctx: &mut Context<TsToAudio<Self>>)
	where Self: Sized + 'static;

	/// Used to start playback if it was paused.
	fn got_play_msg(ts_to_audio: &mut TsToAudio<Self>)
	where Self: Sized;

	/// Re-open the playback device.
	fn reset(ts_to_audio: &mut TsToAudio<Self>)
	where Self: Sized;

	fn get_audio_devices(ts_to_audio: &mut TsToAudio<Self>) -> Vec<String>
	where Self: Sized;
}

pub struct TsToAudio<Impl> {
	pub preferred_device: Option<String>,
	pub data: Arc<Mutex<AudioHandler>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
	/// The global volume to multiply all output with.
	///
	/// This is actually a `f32`, there is no `AtomicF32` though.
	global_volume: Arc<AtomicU32>,
	pub real_impl: Impl,
}

pub struct TsToAudioCallback {
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

impl<Impl: TsToAudioImpl + 'static> Actor for TsToAudio<Impl> {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) { Impl::started(self, ctx); }
}

impl<Impl: TsToAudioImpl> TsToAudio<Impl> {
	pub(crate) fn new(
		real_impl: Impl, preferred_device: Option<String>,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>, global_volume: f32,
	) -> Self {
		let data = Arc::new(Mutex::new(AudioHandler::new()));

		Self {
			preferred_device,
			data,
			connections,
			global_volume: Arc::new(AtomicU32::new(global_volume.to_bits())),
			real_impl,
		}
	}

	pub fn get_callback(&self) -> TsToAudioCallback {
		TsToAudioCallback {
			span: info_span!("ts-to-audio"),
			data: self.data.clone(),
			connections: self.connections.clone(),
			loudness: Default::default(),
			handle: Handle::current(),
			global_volume: self.global_volume.clone(),
		}
	}
}

impl<Impl: TsToAudioImpl + 'static> Handler<PlayMsg> for TsToAudio<Impl> {
	type Result = Result<()>;
	fn handle(&mut self, PlayMsg(id, packet): PlayMsg, ctx: &mut Self::Context) -> Self::Result {
		{
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
		}
		Impl::got_play_msg(self);
		Ok(())
	}
}

impl<Impl: TsToAudioImpl + 'static> Handler<SetGlobalVolumeMsg> for TsToAudio<Impl> {
	type Result = ();
	fn handle(
		&mut self, SetGlobalVolumeMsg(volume): SetGlobalVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.global_volume.store(volume.to_bits(), Ordering::Relaxed);
	}
}

impl<Impl: TsToAudioImpl + 'static> Handler<SetVolumeMsg> for TsToAudio<Impl> {
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

impl<Impl: TsToAudioImpl + 'static> Handler<ResetMsg> for TsToAudio<Impl> {
	type Result = ();
	fn handle(&mut self, _: ResetMsg, _: &mut Self::Context) -> Self::Result { Impl::reset(self); }
}

impl<Impl: TsToAudioImpl + 'static> Handler<GetAudioDevices> for TsToAudio<Impl> {
	type Result = Vec<String>;
	fn handle(&mut self, _: GetAudioDevices, _: &mut Self::Context) -> Self::Result {
		Impl::get_audio_devices(self)
	}
}

impl<Impl: TsToAudioImpl + 'static> Handler<SetAudioDevice> for TsToAudio<Impl> {
	type Result = ();
	fn handle(&mut self, set: SetAudioDevice, _: &mut Self::Context) -> Self::Result {
		if self.preferred_device != set.0 {
			self.preferred_device = set.0;
			Impl::reset(self);
		}
	}
}

impl TsToAudioCallback {
	pub fn callback(&mut self, buffer: &mut [f32]) {
		let _span = self.span.enter();
		trace!(len = buffer.len(), "Filling playback buffer");
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
	}
}
