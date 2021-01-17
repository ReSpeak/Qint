use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::Duration;

use actix::*;
use anyhow::{format_err, Result};
use audiopus::coder::Encoder;
use ebur128::EbuR128;
use futures::prelude::*;
use nnnoiseless::DenoiseState;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};
use tokio::sync::mpsc;
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutPacket};

use super::*;
use crate::loudness_ws::LoudnessService;
use crate::websocket::{CaptureLoudnessMsg, SendPacketMsg, SetSelfTalkingMsg, Ws};

pub(crate) struct AddListenerMsg(pub Addr<Ws>);
pub(crate) struct RemoveListenerMsg(pub Addr<Ws>);
pub(crate) struct AddLoudnessListenerMsg(pub Addr<LoudnessService>);
pub(crate) struct RemoveLoudnessListenerMsg(pub Addr<LoudnessService>);
pub(crate) struct SetPacketlossMsg(pub f32);
/// An audio packet and `true` if this is the last packet.
pub(crate) struct PlayPacketMsg(Option<(OutPacket, bool)>, Option<f64>);
pub(crate) struct SetLoudnessThresholdMsg(pub f64);
pub(crate) struct ResetMsg;

/// Threshold for voice activation detection.
const VAD_THRESHOLD: f32 = 0.3;
/// The default minimum loudness for voice activation detection.
const DEFAULT_LOUDNESS_THRESHOLD: f64 = -50.0;

/// How many packets should still be sent after the voice detection is under the
/// threshold.
const TALKING_TIME: u8 = 5;

/// Magic value sent if the client stopped talking.
const LOUDNESS_END_MAGIC: f64 = -1000.0;

pub struct AudioToTs {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	spawn_send: mpsc::Sender<PlayPacketMsg>,
	connections: HashSet<Addr<Ws>>,
	loudness_cons: HashSet<Addr<LoudnessService>>,
	/// The loudness threshold in LUFS (Loudness Unit Full Scale).
	///
	/// This is actually a `f64`, there is no `AtomicF64` though.
	loudness_threshold: Arc<AtomicU64>,
	/// Packet loss in percent, 0-100.
	packet_loss: Arc<AtomicU8>,

	device: Option<AudioDevice<SdlCallback>>,
	/// If we are actually talking and sending audio
	is_talking: bool,
}

struct SdlCallback {
	logger: Logger,
	channels: audiopus::Channels,
	encoder: Option<Encoder>,
	denoise: Box<DenoiseState<'static>>,
	denoise_buffer: [f32; DenoiseState::FRAME_SIZE],
	loudness: Option<EbuR128>,
	loudness_threshold: Arc<AtomicU64>,
	packet_loss: Arc<AtomicU8>,
	opus_output: [u8; MAX_OPUS_FRAME_SIZE],
	/// The last captured buffer if we are not talking.
	///
	/// We keep one and if we start talking, we encode and send this first. This
	/// ensures a smoother start.
	/// Empty if we are currently sending.
	last_buffer: Vec<f32>,
	/// If we are actually talking and sending audio.
	///
	/// This is `TALKING_TIME + 1` if voice activation triggers and greater 0 if
	/// packets should be sent.
	is_talking: u8,

	spawn_send: mpsc::Sender<PlayPacketMsg>,
}

impl Actor for AudioToTs {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.open_device();

		ctx.run_interval(Duration::from_secs(1), |a2t, _| {
			if a2t.device.as_ref().map(|d| d.status() == AudioStatus::Stopped).unwrap_or(true) {
				// Try to reconnect to audio
				a2t.open_device();
			}
		});
	}
}

impl Message for AddListenerMsg {
	type Result = ();
}
impl Message for RemoveListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for AddLoudnessListenerMsg {
	type Result = ();
}
impl Message for RemoveLoudnessListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for SetPacketlossMsg {
	type Result = ();
}
impl Message for PlayPacketMsg {
	type Result = ();
}
impl Message for SetLoudnessThresholdMsg {
	type Result = ();
}
impl Message for ResetMsg {
	type Result = ();
}

impl Handler<AddListenerMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: AddListenerMsg, _: &mut Self::Context) -> Self::Result {
		self.connections.insert(msg.0.clone());
		self.update_device_state();
		if self.is_talking {
			// Update is_talking for this connection
			tokio::spawn(msg.0.send(SetSelfTalkingMsg(self.is_talking)));
		}
		debug!(self.logger, "Add listener");
	}
}

impl Handler<RemoveListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(&mut self, msg: RemoveListenerMsg, _: &mut Self::Context) -> Self::Result {
		debug!(self.logger, "Removing listener");
		if self.is_talking {
			// Update is_talking for this connection
			tokio::spawn(msg.0.send(SetSelfTalkingMsg(false)));
		}
		let r = self.connections.remove(&msg.0);
		self.update_device_state();
		r
	}
}

impl Handler<AddLoudnessListenerMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: AddLoudnessListenerMsg, _: &mut Self::Context) -> Self::Result {
		self.loudness_cons.insert(msg.0);
		self.update_device_state();
	}
}

impl Handler<RemoveLoudnessListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(&mut self, msg: RemoveLoudnessListenerMsg, _: &mut Self::Context) -> Self::Result {
		let r = self.loudness_cons.remove(&msg.0);
		self.update_device_state();
		r
	}
}

impl Handler<SetPacketlossMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: SetPacketlossMsg, _: &mut Self::Context) -> Self::Result {
		self.packet_loss.store((msg.0 * 100.0) as u8, Ordering::Relaxed);
	}
}

impl Handler<PlayPacketMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, PlayPacketMsg(packet_end, loudness): PlayPacketMsg, _: &mut Self::Context,
	) -> Self::Result {
		// Write into packet sink
		let logger = self.logger.clone();
		if let Some((packet, is_end)) = packet_end {
			self.connections.retain(|con| {
				if !con.connected() {
					false
				} else {
					let logger2 = logger.clone();
					tokio::spawn(con.send(SendPacketMsg(packet.clone())).map(move |r| {
						if let Err(e) = r {
							warn!(logger2, "Failed to send audio packet"; "error" => %e);
						}
					}));

					if let Some(loudness) = loudness {
						let logger2 = logger.clone();
						tokio::spawn(con.send(CaptureLoudnessMsg(loudness)).map(move |r| {
							if let Err(e) = r {
								warn!(logger2, "Failed to send loudness"; "error" => %e);
							}
						}));
					}

					true
				}
			});

			if self.is_talking != !is_end {
				self.is_talking = !is_end;
				self.update_talking();
			}
		}

		if let Some(loudness) = loudness {
			self.loudness_cons.retain(|con| {
				if !con.connected() {
					false
				} else {
					let logger = logger.clone();
					tokio::spawn(con.send(CaptureLoudnessMsg(loudness)).map(move |r| {
						if let Err(e) = r {
							warn!(logger, "Failed to send loudness"; "error" => %e);
						}
					}));

					true
				}
			});
		}

		self.update_device_state();
	}
}

impl Handler<SetLoudnessThresholdMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, SetLoudnessThresholdMsg(thres): SetLoudnessThresholdMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.loudness_threshold.store(thres.to_bits(), Ordering::Relaxed);
	}
}

impl Handler<ResetMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, _: ResetMsg, _: &mut Self::Context) -> Self::Result { self.open_device(); }
}

impl AudioToTs {
	pub(crate) fn new(
		logger: Logger, audio_subsystem: AudioSubsystem, spawn_send: mpsc::Sender<PlayPacketMsg>,
	) -> Result<Self> {
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));

		Ok(Self {
			logger,
			audio_subsystem,
			spawn_send,
			connections: Default::default(),
			loudness_cons: Default::default(),
			loudness_threshold: Arc::new(AtomicU64::new(DEFAULT_LOUDNESS_THRESHOLD.to_bits())),
			packet_loss: Arc::new(AtomicU8::new(0)),

			device: None,
			is_talking: false,
		})
	}

	fn open_device(&mut self) {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(1),
			// Default sample size, 20 ms per packet
			samples: Some(48000 / 50),
		};

		let spawn_send = self.spawn_send.clone();
		match self
			.audio_subsystem
			.open_capture(None, &desired_spec, |spec| {
				// This spec will always be the desired spec, the sdl wrapper
				// passes zero as `allowed_changes`.
				debug!(self.logger, "Got capture spec"; "spec" => ?spec,
					"driver" => self.audio_subsystem.current_audio_driver());
				let channels = if spec.channels == 1 {
					audiopus::Channels::Mono
				} else {
					audiopus::Channels::Stereo
				};

				SdlCallback::new(
					self.logger.clone(),
					channels,
					spawn_send,
					self.loudness_threshold.clone(),
					self.packet_loss.clone(),
				)
			})
			.map_err(|e| format_err!("SDL error: {}", e))
		{
			Ok(device) => {
				self.device = Some(device);
				self.update_device_state();
			}
			Err(e) => {
				error!(self.logger, "Failed to open capture device";
					"error" => %e);
			}
		}
	}

	fn update_talking(&self) {
		for con in &self.connections {
			tokio::spawn(con.send(SetSelfTalkingMsg(self.is_talking)));
		}
	}

	fn update_device_state(&self) {
		if let Some(device) = &self.device {
			if self.connections.is_empty() && self.loudness_cons.is_empty() {
				device.pause();
			} else {
				device.resume();
			}
		}
	}
}

impl SdlCallback {
	fn new(
		logger: Logger, channels: audiopus::Channels, spawn_send: mpsc::Sender<PlayPacketMsg>,
		loudness_threshold: Arc<AtomicU64>, packet_loss: Arc<AtomicU8>,
	) -> Self {
		let loudness = match EbuR128::new(1, super::SAMPLE_RATE as u32, ebur128::Mode::M) {
			Ok(r) => Some(r),
			Err(e) => {
				warn!(logger, "Failed to create loudness measurement"; "error" => %e);
				None
			}
		};
		Self {
			logger,
			channels,
			encoder: None,
			denoise: DenoiseState::new(),
			denoise_buffer: [0.0; DenoiseState::FRAME_SIZE],
			loudness,
			loudness_threshold,
			packet_loss,
			opus_output: [0; MAX_OPUS_FRAME_SIZE],
			last_buffer: Default::default(),
			is_talking: 0,
			spawn_send,
		}
	}

	fn create_encoder(&mut self) -> Result<()> {
		if self.encoder.is_none() {
			self.encoder = Some(Encoder::new(
				audiopus::SampleRate::Hz48000,
				self.channels,
				audiopus::Application::Voip,
			)?);
		}
		Ok(())
	}

	fn send_packet(&mut self, packet: Option<(OutPacket, bool)>, loudness: Option<f64>) {
		if let Err(e) = self.spawn_send.try_send(PlayPacketMsg(packet, loudness)) {
			warn!(self.logger, "Failed to send audio packet"; "error" => %e);
		}
	}

	fn measure_loudness(&mut self, buffer: &[f32]) -> Option<f64> {
		if let Some(ebur128) = &mut self.loudness {
			if let Err(e) = ebur128.add_frames_f32(buffer) {
				warn!(self.logger, "Failed to measure loudness with new data"; "error" => %e);
			} else {
				match ebur128.loudness_momentary() {
					Err(e) => {
						warn!(self.logger, "Failed to measure loudness"; "error" => %e);
					}
					Ok(lufs) => return Some(lufs),
				}
			}
		}
		None
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		let did_talk = self.is_talking != 0;
		let mut should_talk;
		let mut loudness = None;
		let mut packet_end = None;
		// Denoise
		if buffer.len() % DenoiseState::FRAME_SIZE != 0 {
			warn!(self.logger, "Size not fitting for denoising");
			should_talk = true;
		} else {
			// Scale to the expected range
			for d in &mut *buffer {
				*d *= i16::max_value() as f32;
			}

			let mut vad_probe = 0.0;
			for i in buffer.chunks_mut(DenoiseState::FRAME_SIZE) {
				vad_probe += self.denoise.process_frame(&mut self.denoise_buffer, i);
				i.copy_from_slice(&self.denoise_buffer);
			}
			vad_probe /= (buffer.len() / DenoiseState::FRAME_SIZE) as f32;

			trace!(self.logger, "Vad probe"; "value" => vad_probe);

			should_talk = vad_probe >= VAD_THRESHOLD;
			if should_talk || self.is_talking > 1 {
				for d in &mut *buffer {
					*d /= i16::max_value() as f32;
				}
			}
		}

		if should_talk {
			// Additionally measure loudness, it has to be over the threshold
			if let Some(lufs) = self.measure_loudness(buffer) {
				loudness = Some(lufs);
				if lufs < f64::from_bits(self.loudness_threshold.load(Ordering::Relaxed)) {
					should_talk = false;
				}
			}
		}

		if should_talk {
			self.is_talking = TALKING_TIME + 1;
		}

		if !should_talk {
			self.is_talking = self.is_talking.saturating_sub(1);

			if self.is_talking != 0 {
				// Measure loudness so the frontend can display it
				loudness = self.measure_loudness(buffer);
			}
		}

		let codec = if self.channels == audiopus::Channels::Mono {
			CodecType::OpusVoice
		} else {
			CodecType::OpusMusic
		};
		if self.is_talking == 0 {
			if did_talk {
				// Send empty packet to signal end
				trace!(self.logger, "Sending last empty packet");
				let packet = OutAudio::new(&AudioData::C2S { id: 0, codec, data: &[] });
				self.send_packet(Some((packet, true)), Some(LOUDNESS_END_MAGIC));
			} else if loudness.is_some() {
				self.send_packet(None, loudness);
			}
			self.last_buffer.resize(buffer.len(), 0.0);
			self.last_buffer.copy_from_slice(buffer);
			self.encoder = None;
			return;
		}

		if let Err(e) = self.create_encoder() {
			error!(self.logger, "Failed to create opus encoder"; "error" => %e);
			return;
		}

		// Update packet loss
		let loss = self.packet_loss.load(Ordering::Relaxed);
		let encoder = self.encoder.as_mut().unwrap();
		if loss == 0 {
			if let Err(e) = encoder.set_inband_fec(false) {
				warn!(self.logger, "Failed to disable opus inband fec"; "error" => %e);
			}
		} else {
			if let Err(e) = encoder.set_packet_loss_perc(loss) {
				warn!(self.logger, "Failed to set opus packet loss"; "error" => %e, "loss" => loss);
			}
			if let Err(e) = encoder.set_inband_fec(true) {
				warn!(self.logger, "Failed to enable opus inband fec"; "error" => %e);
			}
		}

		if !did_talk {
			// Send cached last buffer if there was one
			if !self.last_buffer.is_empty() {
				trace!(self.logger, "Start to talk: Sending cached last buffer");
				for d in &mut self.last_buffer {
					*d /= i16::max_value() as f32;
				}
				match encoder.encode_float(&self.last_buffer, &mut self.opus_output[..]) {
					Err(e) => {
						warn!(self.logger, "Failed to encode opus"; "error" => %e);
					}
					Ok(len) => {
						// Create packet
						let packet = OutAudio::new(&AudioData::C2S {
							id: 0,
							codec,
							data: &self.opus_output[..len],
						});
						self.send_packet(Some((packet, false)), loudness);
					}
				}
				self.last_buffer.clear();
			}
		}

		match self.encoder.as_ref().unwrap().encode_float(buffer, &mut self.opus_output[..]) {
			Err(e) => {
				warn!(self.logger, "Failed to encode opus"; "error" => %e);
			}
			Ok(len) => {
				trace!(self.logger, "Sending packet");
				// Create packet
				let packet =
					OutAudio::new(&AudioData::C2S { id: 0, codec, data: &self.opus_output[..len] });
				packet_end = Some((packet, false));
			}
		}

		if packet_end.is_some() || loudness.is_some() {
			self.send_packet(packet_end, loudness);
		}
	}
}
