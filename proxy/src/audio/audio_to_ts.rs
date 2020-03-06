use std::time::Duration;

use actix::*;
use audiopus::coder::Encoder;
use failure::{format_err, Error};
use futures::prelude::*;
use futures01::{Future, Sink};
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use rnnoise_c::DenoiseState;
use sdl2::audio::{
	AudioCallback, AudioDevice, AudioSpec, AudioSpecDesired, AudioStatus,
};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, warn, Logger};
use tokio::runtime::Handle;
use tsproto::client::ClientConVal;
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutPacket};

use crate::websocket::SetSelfTalkingMsg;
use super::*;

pub(crate) struct SetListenerMsg {
	pub connection: tsclientlib::Connection,
	pub ts_connection: Addr<TsConnection>,
}

pub struct RemoveListenerMsg;
pub struct SetVolumeMsg(pub f32);
pub struct SetPlayingMsg(pub bool);
struct PlayPacketMsg(Vec<f32>);

/// Threshold for voice activation detection.
const VAD_THRESHOLD: f32 = 0.1;

pub struct AudioToTs {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	executor: ThreadPool,
	handle: Handle,
	listener: Option<ClientConVal>,
	connection: Option<Addr<TsConnection>>,
	encoder: Option<AudioEncoder>,

	is_playing: bool,
	is_talking: bool,
	volume: f32,
}

struct AudioEncoder {
	logger: Logger,
	device: AudioDevice<SdlCallback>,
	spec: AudioSpec,

	encoder: Encoder,
	denoise: DenoiseState,
	opus_output: [u8; MAX_OPUS_FRAME_SIZE],
}

struct SdlCallback {
	logger: Logger,
	handle: Handle,
	a2t: Addr<AudioToTs>,
}

impl Actor for AudioToTs {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		ctx.run_interval(Duration::from_secs(1), |a2t, ctx| {
			if a2t.encoder.as_ref().map(|e| e.device.status() == AudioStatus::Stopped).unwrap_or(true) {
				// Try to reconnect to audio
				match AudioEncoder::new(
					a2t.logger.clone(),
					&a2t.audio_subsystem,
					a2t.handle.clone(),
					ctx.address(),
				) {
					Ok(e) => {
						debug!(a2t.logger, "Reconnected to capture device");
						if a2t.is_playing {
							e.device.resume();
						}
						a2t.encoder = Some(e);
					}
					Err(e) => {
						error!(a2t.logger, "Failed to open capture device"; "error" => ?e);
					}
				};
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
impl Message for SetVolumeMsg {
	type Result = ();
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
		&mut self,
		msg: SetListenerMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		// Remove from previous connection
		let is_playing = self.is_playing;
		self.is_playing = false;
		self.update_talking();
		self.is_playing = is_playing;

		self.connection = Some(msg.ts_connection);
		self.listener = Some(msg.connection.get_tsproto_connection());
		self.update_talking();
	}
}

impl Handler<RemoveListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(
		&mut self,
		_: RemoveListenerMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		self.is_playing = false;
		self.update_talking();
		self.connection = None;
		let res = self.listener.is_some();
		self.listener = None;
		if let Some(e) = &self.encoder {
			e.device.pause();
		}
		res
	}
}

impl Handler<SetVolumeMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self,
		msg: SetVolumeMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		self.volume = msg.0;
	}
}

impl Handler<SetPlayingMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self,
		msg: SetPlayingMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		if let Some(e) = &self.encoder {
			if msg.0 {
				e.device.resume();
			} else {
				e.device.pause();
			}
		}
		self.is_playing = msg.0;
		self.update_talking();
	}
}

impl Handler<PlayPacketMsg> for AudioToTs {
	type Result = ();
	fn handle(
		&mut self,
		PlayPacketMsg(mut buffer): PlayPacketMsg,
		_: &mut Self::Context,
	) -> Self::Result
	{
		// Write into packet sink
		if let Some(con) = &mut self.listener {
			if con.upgrade().is_none() {
				self.listener = None;
				return;
			}
			drop(con);

			let vol = self.volume;
			if let Some(e) = &mut self.encoder {
				let talking;
				if let Some(packet) = e.handle_audio_buffer(&mut buffer, vol) {
					talking = true;
					let sink = self.listener.as_mut().unwrap().as_packet_sink();
					let logger = self.logger.clone();
					self.executor
						.spawn(sink.send(packet).map(|_| ()).map_err(
							move |e| {
								error!(logger, "Failed to send packet"; "error" => ?e);
							},
						))
						.detach();
				} else {
					talking = false;
				}

				if talking != self.is_talking {
					self.is_talking = talking;
					self.update_talking();
				}
			}
		}
	}
}

impl AudioToTs {
	pub fn new(
		logger: Logger,
		audio_subsystem: AudioSubsystem,
		executor: ThreadPool,
		handle: Handle,
	) -> Result<Self, Error>
	{
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));

		Ok(Self {
			logger,
			audio_subsystem,
			executor,
			handle,
			listener: None,
			connection: None,
			encoder: None,

			is_playing: false,
			is_talking: false,
			volume: 1.0,
		})
	}

	fn update_talking(&self) {
		if let Some(con) = &self.connection {
			tokio::spawn(con.send(SetSelfTalkingMsg(self.is_playing && self.is_talking)));
		}
	}
}

impl AudioEncoder {
	fn new(
		logger: Logger,
		audio_subsystem: &AudioSubsystem,
		handle: Handle,
		a2t: Addr<AudioToTs>,
	) -> Result<Self, Error> {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(1),
			// Default sample size, 20 ms per packet
			samples: Some(48000 / 50),
		};

		let logger2 = logger.clone();
		let mut audio_spec = None;
		let mut opus_channels = None;
		let device = audio_subsystem.open_capture(None, &desired_spec, |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			debug!(logger, "Got capture spec"; "spec" => ?spec,
				"driver" => audio_subsystem.current_audio_driver());
			opus_channels = Some(if spec.channels == 1 {
				audiopus::Channels::Mono
			} else {
				audiopus::Channels::Stereo
			});

			audio_spec = Some(spec);
			SdlCallback {
				logger,
				handle,
				a2t,
			}
		}).map_err(|e| format_err!("SDL error: {}", e))?;

		Ok(Self {
			logger: logger2,
			device,
			spec: audio_spec.unwrap(),

			encoder: Encoder::new(audiopus::SampleRate::Hz48000,
				opus_channels.unwrap(), audiopus::Application::Voip)
				.expect("Could not create opus encoder"),
			denoise: DenoiseState::new(),
			opus_output: [0; MAX_OPUS_FRAME_SIZE],
		})
	}

	fn handle_audio_buffer(&mut self, buffer: &mut [f32], volume: f32) -> Option<OutPacket> {
		// Denoise
		if buffer.len() % rnnoise_c::FRAME_SIZE != 0 {
			warn!(self.logger, "Size not fitting for denoising");
		} else {
			// Scale to the expected range
			for d in &mut *buffer {
				*d *= u16::max_value() as f32;
			}

			let mut vad_probe = 0.0;
			for i in buffer.chunks_mut(rnnoise_c::FRAME_SIZE) {
				vad_probe += self.denoise.process_frame_in_place(i);
			}
			vad_probe /= (buffer.len() / rnnoise_c::FRAME_SIZE) as f32;

			//debug!(self.logger, "Vad probe"; "value" => vad_probe);
			if vad_probe < VAD_THRESHOLD {
				return None;
			}

			for d in &mut *buffer {
				*d /= u16::max_value() as f32;
			}
		}

		// Handle volume
		if volume != 1.0 {
			for d in &mut *buffer {
				*d *= volume;
			}
		}

		match self.encoder.encode_float(buffer, &mut self.opus_output[..]) {
			Err(e) => {
				error!(self.logger, "Failed to encode opus"; "error" => ?e);
				None
			}
			Ok(len) => {
				// Create packet
				let codec = if self.spec.channels == 1 {
					CodecType::OpusVoice
				} else {
					CodecType::OpusMusic
				};
				Some(OutAudio::new(&AudioData::C2S {
					id: 0,
					codec,
					data: &self.opus_output[..len],
				}))
			}
		}
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		let logger = self.logger.clone();
		let a2t = self.a2t.clone();
		let buffer = buffer.to_vec();
		/*self.handle.enter(move ||
			tokio::spawn(a2t.send(PlayPacketMsg(buffer))
				.map(move |r| {match r {
					Ok(()) => {}
					Err(e) => {
						error!(logger, "Failed to send audio data to Audio2TS pipeline"; "error" => ?e)
					}
				}
		})));*/
		std::thread::spawn(move || {
			let mut rt = tokio::runtime::Runtime::new().unwrap();

			rt.block_on(async {
				let local = tokio::task::LocalSet::new();

				// Run the local task set.
				local.run_until(async move {
					tokio::task::spawn_local(
						a2t.send(PlayPacketMsg(buffer))
							.map(move |r| {match r {
								Ok(()) => {}
								Err(e) => {
									error!(logger, "Failed to send audio data to Audio2TS pipeline"; "error" => ?e)
								}
							}
							}),
					).await.unwrap();
				}).await;

				local.await;
			});
		});
	}
}
