use std::time::Duration;

use actix::*;
use anyhow::{format_err, Result};
use audiopus::coder::Encoder;
use futures::prelude::*;
use rnnoise_c::DenoiseState;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};
use tokio::sync::mpsc;
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutPacket};

use super::*;
use crate::websocket::{SendPacketMsg, SetSelfTalkingMsg, Ws};

pub(crate) struct SetListenerMsg {
	pub connection: Addr<Ws>,
}

pub struct RemoveListenerMsg;
pub struct SetPlayingMsg(pub bool);
/// An audio packet and `true` if this is the last packet.
pub(crate) struct PlayPacketMsg(OutPacket, bool);

/// Threshold for voice activation detection.
const VAD_THRESHOLD: f32 = 0.2;

/// How many packets should still be sent after the voice detection is under the
/// threshold.
const TALKING_TIME: u8 = 5;

pub struct AudioToTs {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	spawn_send: mpsc::Sender<PlayPacketMsg>,
	connection: Option<Addr<Ws>>,

	device: Option<AudioDevice<SdlCallback>>,
	/// If we are muted or not
	is_playing: bool,
	/// If we are actually talking and sending audio
	is_talking: bool,
}

struct SdlCallback {
	logger: Logger,
	channels: audiopus::Channels,
	encoder: Option<Encoder>,
	denoise: DenoiseState,
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
			if a2t
				.device
				.as_ref()
				.map(|d| d.status() == AudioStatus::Stopped)
				.unwrap_or(true)
			{
				// Try to reconnect to audio
				a2t.open_device();
			}
		});
	}
}

impl Message for SetListenerMsg {
	type Result = ();
}

impl Message for RemoveListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for SetPlayingMsg {
	type Result = ();
}
impl Message for PlayPacketMsg {
	type Result = ();
}

impl Handler<SetListenerMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, msg: SetListenerMsg, _: &mut Self::Context,
	) -> Self::Result {
		// Remove from previous connection
		let is_playing = self.is_playing;
		self.is_playing = false;
		self.update_talking();
		self.is_playing = is_playing;

		self.connection = Some(msg.connection);
		self.update_talking();
	}
}

impl Handler<RemoveListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(
		&mut self, _: RemoveListenerMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.is_playing = false;
		self.update_talking();
		if let Some(device) = &self.device {
			device.pause();
		}
		self.connection.take().is_some()
	}
}

impl Handler<SetPlayingMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, SetPlayingMsg(play): SetPlayingMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(device) = &self.device {
			if play {
				device.resume();
			} else {
				device.pause();
			}
		}
		self.is_playing = play;
		self.update_talking();
	}
}

impl Handler<PlayPacketMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self, PlayPacketMsg(packet, is_end): PlayPacketMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		// Write into packet sink
		if let Some(con) = &mut self.connection {
			if !con.connected() {
				self.connection = None;
				if let Some(d) = &self.device {
					d.pause();
				}
				return;
			}
			let talking = self.is_talking;
			self.is_talking = !is_end;

			let logger = self.logger.clone();
			tokio::spawn(con.send(SendPacketMsg(packet)).map(move |r| {
				if let Err(e) = r {
					error!(logger, "Failed to send audio packet";
						"error" => ?e);
				}
			}));

			if talking != self.is_talking {
				self.update_talking();
			}
		}
	}
}

impl AudioToTs {
	pub(crate) fn new(
		logger: Logger, audio_subsystem: AudioSubsystem,
		spawn_send: mpsc::Sender<PlayPacketMsg>,
	) -> Result<Self>
	{
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));

		Ok(Self {
			logger,
			audio_subsystem,
			spawn_send,
			connection: None,
			device: None,

			is_playing: false,
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
				if self.is_playing {
					device.resume();
				}
				self.device = Some(device);
			}
			Err(e) => {
				error!(self.logger, "Failed to open capture device";
					"error" => ?e);
			}
		}
	}

	fn update_talking(&self) {
		if let Some(con) = &self.connection {
			tokio::spawn(
				con.send(SetSelfTalkingMsg(self.is_playing && self.is_talking)),
			);
		}
	}
}

impl SdlCallback {
	fn new(
		logger: Logger, channels: audiopus::Channels,
		spawn_send: mpsc::Sender<PlayPacketMsg>,
	) -> Self
	{
		Self {
			logger,
			channels,
			encoder: None,
			denoise: DenoiseState::new(),
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
		if let Err(e) = self.spawn_send.try_send(PlayPacketMsg(packet, is_end))
		{
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
		if buffer.len() % rnnoise_c::FRAME_SIZE != 0 {
			warn!(self.logger, "Size not fitting for denoising");
			should_talk = true;
		} else {
			// Scale to the expected range
			for d in &mut *buffer {
				*d *= i16::max_value() as f32;
			}

			let mut vad_probe = 0.0;
			for i in buffer.chunks_mut(rnnoise_c::FRAME_SIZE) {
				vad_probe += self.denoise.process_frame_in_place(i);
			}
			vad_probe /= (buffer.len() / rnnoise_c::FRAME_SIZE) as f32;

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
				let packet =
					OutAudio::new(&AudioData::C2S { id: 0, codec, data: &[] });
				self.send_packet(packet, true);
			}
			self.last_buffer.resize(buffer.len(), 0.0);
			self.last_buffer.copy_from_slice(buffer);
			self.encoder = None;
			return;
		}

		if let Err(e) = self.create_encoder() {
			error!(self.logger, "Failed to create opus encoder"; "error" => ?e);
			return;
		}

		if !did_talk {
			// Send cached last buffer if there was one
			if !self.last_buffer.is_empty() {
				trace!(
					self.logger,
					"Start to talk: Sending cached last buffer"
				);
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
						warn!(self.logger, "Failed to encode opus";
							"error" => ?e);
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

		match self
			.encoder
			.as_ref()
			.unwrap()
			.encode_float(buffer, &mut self.opus_output[..])
		{
			Err(e) => {
				warn!(self.logger, "Failed to encode opus"; "error" => ?e);
			}
			Ok(len) => {
				trace!(self.logger, "Sending packet");
				// Create packet
				let packet = OutAudio::new(&AudioData::C2S {
					id: 0,
					codec,
					data: &self.opus_output[..len],
				});
				self.send_packet(packet, false);
			}
		}
	}
}
