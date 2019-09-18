use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fmt;
use std::sync::Arc;

use actix_web::actix::*;
use audiopus::coder::{Decoder, GenericCtl};
use failure::{format_err, Error};
use parking_lot::Mutex;
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpec, AudioSpecDesired};
use slog::{info, o, Logger};
use tsclientlib::ClientId;
use tsproto_packets::packets::{AudioData, CodecType, InAudio};

use crate::ConnectionId;

/// The maximum supported size of a decoded audio packet.
///
/// Use 48 kHz, 20 ms frames (50 times per second) and stereo data.
const MAX_FRAME_SIZE: usize = 48000 / 50 * 2;

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
}

struct Voice {
	decoder: Decoder,
}

struct VoiceData {
	queue: Vec<f32>,
	channels: u8,
}

pub struct TsToAudio {
	logger: Logger,
	device: AudioDevice<SdlCallback>,
	voices: HashMap<Id, Voice>,
	data: Arc<Mutex<HashMap<Id, VoiceData>>>,

	/// Decoded opus data
	// TODO Support bigger packets?
	opus_output: [f32; MAX_FRAME_SIZE],
}

impl VoiceData {
	fn new(channels: u8) -> Self {
		Self {
			queue: Default::default(),
			channels,
		}
	}
}

impl Actor for TsToAudio {
	type Context = Context<Self>;
}

impl TsToAudio {
	pub fn new(logger: Logger, audio_subsystem: &AudioSubsystem) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "ts-to-audio"));
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(2),
			// Default sample size, 20 ms per packet
			samples: Some(48000 / 50),
		};

		let data = Arc::new(Mutex::new(Default::default()));

		let logger2 = logger.clone();
		let data2 = data.clone();
		let device = audio_subsystem.open_playback(None, &desired_spec, move |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			info!(logger2, "Got playback spec"; "spec" => ?spec);
			SdlCallback {
				spec,
				data: data2.clone(),
			}
		}).map_err(|e| format_err!("SDL error: {}", e))?;


		Ok(Self {
			logger,
			device,
			voices: Default::default(),
			data,

			opus_output: [0f32; MAX_FRAME_SIZE],
		})
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: PlayMsg, _: &mut Self::Context) -> Self::Result {
		// TODO Whisper packets
		if let AudioData::S2C { id: _, from, codec, data } = msg.1.data() {
			if *codec != CodecType::OpusVoice && *codec != CodecType::OpusMusic {
				return Err(format_err!("Got unsupported audio codec, only opus is supported"));
			}

			let id = Id { con: msg.0, client: ClientId(*from) };
			let channels;
			let opus_channels;
			if *codec == CodecType::OpusMusic {
				channels = 2;
				opus_channels = audiopus::Channels::Stereo;
			} else {
				channels = 1;
				opus_channels = audiopus::Channels::Mono;
			}

			//info!(self.logger, "Getting voice"; "id" => %id);
			let mut tmp_entry;
			let voice = match self.voices.entry(id) {
				Entry::Occupied(o) => {
					tmp_entry = o;
					tmp_entry.get_mut()
				}
				Entry::Vacant(v) => {
					info!(self.logger, "Creating opus decoder"; "channels" => channels);

					// TODO Always use the channel count of SDL, opus automatically
					// averages or duplicates samples for each channel.
					let decoder = Decoder::new(audiopus::SampleRate::Hz48000, opus_channels)?;
					v.insert(Voice { decoder })
				}
			};

			if data.len() == 0 {
				info!(self.logger, "Resetting decoder");
				voice.decoder.reset_state()?;
				return Ok(());
			}

			//info!(self.logger, "Decode opus packets"; "len" => data.len());
			let len = voice.decoder.decode_float(*data, &mut self.opus_output[..], false)?;
			//info!(self.logger, "Decoded frames"; "len" => len);

			// Put into queue
			{
				let mut data = self.data.lock();
				let d = data.entry(id).or_insert_with(|| VoiceData::new(channels as u8));
				let queue = &mut d.queue;
				if queue.len() > len * channels * 2 {
					info!(self.logger, "Removing samples"; "count" => queue.len());
					*queue = queue.split_off(queue.len() - len * channels);
					queue.clear();
				}
				queue.extend_from_slice(&self.opus_output[..len * channels]);
			}
			self.device.resume();
		}
		Ok(())
	}
}

struct SdlCallback {
	spec: AudioSpec,
	data: Arc<Mutex<HashMap<Id, VoiceData>>>,
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		//info!(self.logger, "Filling buffer"; "buffer len" => buffer.len());
		// Fill the buffer with silence
		for d in &mut *buffer {
			*d = 0.0;
		}

		// Mix data
		let mut data = self.data.lock();
		data.retain(|_, d| {
			let queue = &mut d.queue;

			// TODO Dynamically check channel count
			let len;
			if d.channels == 2 {
				len = std::cmp::min(buffer.len(), queue.len());
				buffer[..len].copy_from_slice(&queue[..len]);
			} else {
				// Convert mono to stereo
				len = std::cmp::min(buffer.len() / 2, queue.len());
				for i in 0..len {
					buffer[i * 2] = queue[i];
					buffer[i * 2 + 1] = queue[i];
				}
			}

			if queue.len() == len {
				false
			} else {
				*queue = queue.split_off(len);
				//info!(self.logger, "Left buffer"; "len" => queue.len());
				true
			}
		});
	}
}
