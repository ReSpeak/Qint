use std::collections::HashSet;
use std::time::Duration;

use actix::*;
use anyhow::{format_err, Result};
use audiopus::coder::Encoder;
use futures::prelude::*;
use nnnoiseless::DenoiseState;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};
use tokio::sync::mpsc;
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutPacket};

use super::*;
use crate::websocket::{SendPacketMsg, SetSelfTalkingMsg, Ws};

pub(crate) struct AddListenerMsg(pub Addr<Ws>);
pub(crate) struct RemoveListenerMsg(pub Addr<Ws>);
/// An audio packet and `true` if this is the last packet.
pub(crate) struct PlayPacketMsg(OutPacket, bool);
pub(crate) struct ResetMsg;

/// Threshold for voice activation detection.
const VAD_THRESHOLD: f32 = 0.2;

/// How many packets should still be sent after the voice detection is under the
/// threshold.
const TALKING_TIME: u8 = 5;

pub struct AudioToTs {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	spawn_send: mpsc::Sender<PlayPacketMsg>,
	connections: HashSet<Addr<Ws>>,

	device: Option<AudioDevice<SdlCallback>>,
	/// If we are actually talking and sending audio
	is_talking: bool,
}

struct SdlCallback {
	logger: Logger,
	channels: audiopus::Channels,
	encoder: Option<Encoder>,
	denoise: Box<DenoiseState>,
	denoise_buffer: [f32; DenoiseState::FRAME_SIZE],
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
impl Message for PlayPacketMsg {
	type Result = ();
}
impl Message for ResetMsg {
	type Result = ();
}

impl Handler<AddListenerMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: AddListenerMsg, _: &mut Self::Context) -> Self::Result {
		if self.connections.is_empty() {
			if let Some(device) = &self.device {
				device.resume();
			}
		}
		self.connections.insert(msg.0.clone());
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
		if self.connections.is_empty() {
			if let Some(device) = &self.device {
				device.pause();
			}
		}
		r
	}
}

impl Handler<PlayPacketMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, PlayPacketMsg(packet, is_end): PlayPacketMsg, _: &mut Self::Context,
	) -> Self::Result {
		// Write into packet sink
		let is_talking = self.is_talking;
		let logger = self.logger.clone();
		self.connections.retain(|con| {
			if !con.connected() {
				if is_talking {
					// Update is_talking for this connection
					tokio::spawn(con.send(SetSelfTalkingMsg(false)));
				}
				false
			} else {
				let logger = logger.clone();
				tokio::spawn(con.send(SendPacketMsg(packet.clone())).map(move |r| {
					if let Err(e) = r {
						warn!(logger, "Failed to send audio packet";
							"error" => %e);
					}
				}));

				true
			}
		});

		if self.is_talking != !is_end {
			self.is_talking = !is_end;
			self.update_talking();
		}

		if self.connections.is_empty() {
			if let Some(d) = &self.device {
				d.pause();
			}
		}
	}
}

impl Handler<ResetMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, _: ResetMsg, _: &mut Self::Context) -> Self::Result {
		self.open_device();
	}
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

				SdlCallback::new(self.logger.clone(), channels, spawn_send)
			})
			.map_err(|e| format_err!("SDL error: {}", e))
		{
			Ok(device) => {
				if !self.connections.is_empty() {
					device.resume();
				}
				self.device = Some(device);
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
}

impl SdlCallback {
	fn new(
		logger: Logger, channels: audiopus::Channels, spawn_send: mpsc::Sender<PlayPacketMsg>,
	) -> Self {
		Self {
			logger,
			channels,
			encoder: None,
			denoise: DenoiseState::new(),
			denoise_buffer: [0.0; DenoiseState::FRAME_SIZE],
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

	fn send_packet(&mut self, packet: OutPacket, is_end: bool) {
		if let Err(e) = self.spawn_send.try_send(PlayPacketMsg(packet, is_end)) {
			warn!(self.logger, "Failed to send audio packet";
				"error" => %e);
		}
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		let did_talk = self.is_talking != 0;
		let should_talk;
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
			self.is_talking = TALKING_TIME + 1;
		}

		if !should_talk {
			self.is_talking = self.is_talking.saturating_sub(1);
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
				self.send_packet(packet, true);
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

		if !did_talk {
			// Send cached last buffer if there was one
			if !self.last_buffer.is_empty() {
				trace!(self.logger, "Start to talk: Sending cached last buffer");
				for d in &mut self.last_buffer {
					*d /= i16::max_value() as f32;
				}
				match self
					.encoder
					.as_ref()
					.unwrap()
					.encode_float(&self.last_buffer, &mut self.opus_output[..])
				{
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
						self.send_packet(packet, false);
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
				self.send_packet(packet, false);
			}
		}
	}
}
