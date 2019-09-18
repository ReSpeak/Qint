use std::sync::Arc;

use actix_web::actix::*;
use audiopus::coder::Encoder;
use failure::{format_err, Error};
use futures01::{Future, Sink};
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use parking_lot::Mutex;
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpec, AudioSpecDesired};
use slog::{error, info, o, Logger};
use tsproto_packets::packets::{AudioData, CodecType, OutAudio};

const MAX_OPUS_FRAME_SIZE: usize = 1275;

pub struct SetListenerMsg {
	pub connection: tsclientlib::Connection,
}

pub struct RemoveListenerMsg;
pub struct SetVolumeMsg(pub f32);
pub struct SetPlayingMsg(pub bool);

pub struct AudioToTs {
	listeners: Arc<Mutex<Vec<ConnectionSinkCreator>>>,
	device: AudioDevice<SdlCallback>,

	volume: Arc<Mutex<f32>>,
}

struct ConnectionSinkCreator {
	con: tsproto::client::ClientConVal,
}

impl Actor for AudioToTs {
	type Context = Context<Self>;
}

impl Message for SetListenerMsg { type Result = (); }

impl Message for RemoveListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for SetVolumeMsg { type Result = (); }
impl Message for SetPlayingMsg { type Result = Result<(), Error>; }

impl Handler<SetListenerMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: SetListenerMsg, _: &mut Self::Context) -> Self::Result {
		let mut listeners = self.listeners.lock();
		*listeners = vec![ConnectionSinkCreator {
			con: msg.connection.get_tsproto_connection(),
		}];
	}
}

impl Handler<RemoveListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(&mut self, _: RemoveListenerMsg, _: &mut Self::Context) -> Self::Result {
		let mut ls = self.listeners.lock();
		let res = !ls.is_empty();
		ls.clear();
		self.set_playing(false);
		res
	}
}

impl Handler<SetVolumeMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: SetVolumeMsg, _: &mut Self::Context) -> Self::Result {
		self.set_volume(msg.0);
	}
}

impl Handler<SetPlayingMsg> for AudioToTs {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: SetPlayingMsg, _: &mut Self::Context) -> Self::Result {
		self.set_playing(msg.0);
		Ok(())
	}
}

impl AudioToTs {
	pub fn new(
		logger: Logger,
		audio_subsystem: &AudioSubsystem,
		executor: ThreadPool,
	) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));

		let listeners = Arc::new(Mutex::new(Vec::<ConnectionSinkCreator>::new()));

		let listeners2 = listeners.clone();
		let volume = Arc::new(Mutex::new(1.0));
		let volume2 = volume.clone();

		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(1),
			// Default sample size, 20 ms per packet
			samples: Some(48000 / 50),
		};

		let device = audio_subsystem.open_capture(None, &desired_spec, move |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			info!(logger, "Got capture spec"; "spec" => ?spec);
			let opus_channels = audiopus::Channels::Mono;
			let encoder = Encoder::new(audiopus::SampleRate::Hz48000,
				opus_channels, audiopus::Application::Voip).expect("Could not create encoder");

			SdlCallback {
				logger: logger.clone(),
				spec,
				encoder,
				executor: executor.clone(),
				listeners: listeners2.clone(),
				volume: volume2.clone(),

				opus_output: [0; MAX_OPUS_FRAME_SIZE],
			}
		}).map_err(|e| format_err!("SDL error: {}", e))?;

		Ok(Self {
			listeners,
			device,
			volume,
		})
	}

	pub fn set_volume(&mut self, volume: f32) {
		let mut vol = self.volume.lock();
		*vol = volume;
	}

	pub fn set_playing(&self, playing: bool) {
		if playing {
			self.device.resume();
		} else {
			self.device.pause();
		}
	}
}

struct SdlCallback {
	logger: Logger,
	spec: AudioSpec,
	encoder: Encoder,
	executor: ThreadPool,
	listeners: Arc<Mutex<Vec<ConnectionSinkCreator>>>,
	volume: Arc<Mutex<f32>>,

	/// Encoded opus data, maximum opus frame size is 1275 as from RFC6716.
	opus_output: [u8; MAX_OPUS_FRAME_SIZE],
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		// Handle volume
		let volume = *self.volume.lock();
		if volume != 1.0 {
			for d in &mut *buffer {
				*d *= volume;
			}
		}

		// TODO Support stereo
		match self.encoder.encode_float(buffer, &mut self.opus_output[..]) {
			Err(e) => {
				error!(self.logger, "Failed to encode opus"; "error" => ?e);
			}
			Ok(len) => {
				// Create packet
				let packet = OutAudio::new(&AudioData::C2S {
					id: 0,
					codec: CodecType::OpusVoice,
					data: &self.opus_output[..len],
				});

				// Write into packet sink
				let mut listeners = self.listeners.lock();
				listeners.retain(|l| {
					if l.con.upgrade().is_none() {
						return false;
					}

					let sink = l.con.as_packet_sink();
					let logger = self.logger.clone();
					let packet = packet.clone();
					self.executor.spawn(sink.send(packet).map(|_| ()).map_err(move |e| {
						error!(logger, "Failed to send packet"; "error" => ?e);
					})).detach();
					true
				});
			}
		}
	}
}
