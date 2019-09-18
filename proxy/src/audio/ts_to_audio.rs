use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::io::Write;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use actix_web::actix::*;
use actix_web::actix::fut::wrap_future;
use async_timer::oneshot::{Oneshot, Timer};
use audiopus::coder::{Decoder, GenericCtl};
use failure::{format_err, Error};
use futures::prelude::*;
use futures01::Future as _;
use parking_lot::Mutex;
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpec, AudioSpecDesired};
use slog::{info, o, Logger};
use tsclientlib::ClientId;
use tsproto_packets::packets::{AudioData, CodecType, InAudio};

use crate::ConnectionId;
use super::*;

pub struct PlayMsg(pub ConnectionId, pub InAudio);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Id {
	con: ConnectionId,
	client: ClientId,
}

impl Message for PlayMsg { type Result = Result<(), Error>; }

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}-{}", self.con.0, self.client.0)
	}
}fn voice_timeout(last_sent: Arc<Mutex<Instant>>) -> Box<dyn futures01::Future<Item=(), Error=()>> {
	let timeout = Timer::new(Duration::from_secs(VOICE_TIMEOUT_SECS)).unit_error().compat();
	Box::new(timeout.and_then(move |_| -> Box::<dyn futures01::Future<Item=_, Error=_>> {
		let last = *last_sent.lock();
		if Instant::now().duration_since(last).as_secs() >= VOICE_TIMEOUT_SECS {
			Box::new(futures01::future::ok(()))
		} else {
			voice_timeout(last_sent)
		}
	}))
}

struct SdlVoice {
	decoder: Decoder,
}

pub struct TsToAudioSdl {
	logger: Logger,
	device: AudioDevice<SdlCallback>,
	voices: HashMap<Id, SdlVoice>,
	data: Arc<Mutex<HashMap<Id, Vec<f32>>>>,
}

impl Actor for TsToAudioSdl {
	type Context = Context<Self>;
}

impl TsToAudioSdl {
	pub fn new(logger: Logger, audio_subsystem: &AudioSubsystem) -> Result<Self, Error> {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(1),
			// Default sample size, 20 ms per packet
			samples: Some(48000 / 50),
		};

		let data = Arc::new(Mutex::new(Default::default()));

		let logger2 = logger.clone();
		let data2 = data.clone();
		let device = audio_subsystem.open_playback(None, &desired_spec, move |spec| {
			info!(logger2, "Got playback spec"; "spec" => ?spec);
			SdlCallback {
				logger: logger2.clone(),
				spec,
				data: data2.clone(),
			}
		}).map_err(|e| format_err!("SDL error: {}", e))?;


		Ok(Self {
			logger,
			device,
			voices: Default::default(),
			data,
		})
	}
}

impl Handler<PlayMsg> for TsToAudioSdl {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: PlayMsg, _: &mut Self::Context) -> Self::Result {
		// TODO Whisper packets
		if let AudioData::S2C { id: _, from, codec, data } = msg.1.data() {
			if *codec != CodecType::OpusVoice && *codec != CodecType::OpusMusic {
				return Err(format_err!("Got unsupported audio codec, only opus is supported"));
			}

			let id = Id { con: msg.0, client: ClientId(*from) };

			info!(self.logger, "Getting voice"; "id" => %id);
			let mut tmp_entry;
			let voice = match self.voices.entry(id) {
				Entry::Occupied(o) => {
					tmp_entry = o;
					tmp_entry.get_mut()
				}
				Entry::Vacant(v) => {
					let channels = if *codec == CodecType::OpusVoice {
						audiopus::Channels::Mono
					} else {
						audiopus::Channels::Stereo
					};
					let decoder = Decoder::new(audiopus::SampleRate::Hz48000, channels)?;
					v.insert(SdlVoice {
						decoder,
					})
				}
			};

			if data.len() == 0 {
				info!(self.logger, "Resetting decoder");
				voice.decoder.reset_state()?;
				return Ok(());
			}

			let mut output = vec![0f32; 48000 / 50];
			let len = voice.decoder.decode_float(*data, &mut output, false)?;
			output.truncate(len);
			info!(self.logger, "Decoded bytes"; "len" => len);

			// Put into queue
			{
				let mut data = self.data.lock();
				let queue = data.entry(id).or_insert_with(|| Default::default());
				queue.append(&mut output);
			}
			self.device.resume();
		}
		Ok(())
	}
}

struct SdlCallback {
	logger: Logger,
	spec: AudioSpec,
	data: Arc<Mutex<HashMap<Id, Vec<f32>>>>,
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		let mut data = self.data.lock();
		info!(self.logger, "Filling buffer"; "buffer len" => buffer.len());
		// Fill the buffer with silence
		for d in &mut *buffer {
			*d = 0.0;
		}

		// Mix data
		data.retain(|_, d| {
			let len = std::cmp::min(buffer.len(), d.len());
			buffer.copy_from_slice(&d[..len]);
			if d.len() == len {
				false
			} else {
				*d = d.split_off(len);
				true
			}
		});
	}
}
