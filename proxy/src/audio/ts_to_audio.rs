use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;

use actix_web::actix::*;
use audiopus::coder::{Decoder, GenericCtl};
use failure::{format_err, Error};
use parking_lot::Mutex;
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired};
use slog::{debug, o, trace, Logger};
use tsclientlib::ClientId;
use tsproto_packets::packets::{AudioData, CodecType, InAudio};

use crate::ConnectionId;
use super::*;

/// After this amount of seconds, a decoder will be removed.
const VOICE_TIMEOUT_SECS: u64 = 1;

pub struct PlayMsg(pub ConnectionId, pub InAudio);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Id {
	con: ConnectionId,
	client: ClientId,
}

pub struct TsToAudio {
	logger: Logger,
	device: AudioDevice<SdlCallback>,
	// TODO Remove inactive decoders
	decoders: HashMap<Id, Decoder>,
	/// The audio queue, new data is appended at the end and data is loaded
	/// from the beginning.
	data: Arc<Mutex<HashMap<Id, VecDeque<f32>>>>,

	/// Decoded opus data
	opus_output: Vec<f32>,
}

struct SdlCallback {
	logger: Logger,
	data: Arc<Mutex<HashMap<Id, VecDeque<f32>>>>,
}

impl Message for PlayMsg { type Result = Result<(), Error>; }

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}-{}", self.con.0, self.client.0)
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
			samples: Some(USUAL_SAMPLE_COUNT as u16),
		};

		let data = Arc::new(Mutex::new(Default::default()));

		let logger2 = logger.clone();
		let data2 = data.clone();
		let device = audio_subsystem.open_playback(None, &desired_spec, move |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			debug!(logger2, "Got playback spec"; "spec" => ?spec);
			SdlCallback {
				logger: logger2,
				data: data2.clone(),
			}
		}).map_err(|e| format_err!("SDL error: {}", e))?;


		Ok(Self {
			logger,
			device,
			decoders: Default::default(),
			data,

			opus_output: vec![0f32; USUAL_FRAME_SIZE],
		})
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: PlayMsg, _: &mut Self::Context) -> Self::Result {
		if let AudioData::S2C { id: _, from, codec, data } |
			AudioData::S2CWhisper { id: _, from, codec, data } = msg.1.data() {
			if *codec != CodecType::OpusVoice && *codec != CodecType::OpusMusic {
				return Err(format_err!("Got unsupported audio codec, only opus is supported"));
			}

			let id = Id { con: msg.0, client: ClientId(*from) };
			let channels = self.device.spec().channels;

			let mut tmp_entry;
			let decoder = match self.decoders.entry(id) {
				Entry::Occupied(o) => {
					tmp_entry = o;
					tmp_entry.get_mut()
				}
				Entry::Vacant(v) => {
					debug!(self.logger, "Creating opus decoder"; "id" => %id);
					let opus_channels = if channels == 1 {
						audiopus::Channels::Mono
					} else {
						audiopus::Channels::Stereo
					};

					// Always use the channel count of SDL, opus automatically
					// averages or duplicates samples for each channel.
					let decoder = Decoder::new(audiopus::SampleRate::Hz48000, opus_channels)?;
					v.insert(decoder)
				}
			};

			if data.len() == 0 {
				debug!(self.logger, "Resetting decoder"; "id" => %id);
				decoder.reset_state()?;
				return Ok(());
			}

			let len = loop {
				match decoder.decode_float(*data, &mut self.opus_output[..], false) {
					Ok(len) => break len,
					Err(audiopus::error::Error::Opus(audiopus::error::ErrorCode::BufferTooSmall)) => {
						// Enlarge the buffer
						if self.opus_output.len() == MAX_FRAME_SIZE {
							return Err(format_err!("Bad opus packet, maximum buffer size exceeded").into());
						} else if self.opus_output.len() * 2 > MAX_FRAME_SIZE {
							self.opus_output.resize(MAX_FRAME_SIZE, 0f32);
						} else {
							self.opus_output.resize(self.opus_output.len() * 2, 0f32);
						}
					}
					Err(e) => return Err(e.into()),
				}
			};

			// Shrink the buffer
			let size = len * usize::from(channels);
			if size <= self.opus_output.len() / 2 {
				self.opus_output.truncate(len);
			}
			trace!(self.logger, "Decoded opus packet"; "id" => %id, "len" => len);

			// Put into queue
			{
				let mut data = self.data.lock();
				let queue = data.entry(id).or_insert_with(|| Default::default());
				if queue.len() > size * 2 {
					debug!(self.logger, "Removing samples from playback queue"; "id" => %id, "count" => queue.len() - size);
					*queue = queue.split_off(queue.len() - size);
					queue.clear();
				}
				queue.extend(self.opus_output[..size].iter());
			}
			self.device.resume();
		}
		Ok(())
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		trace!(self.logger, "Filling audio playback buffer"; "len" => buffer.len());
		// Fill the buffer with silence
		for d in &mut *buffer {
			*d = 0.0;
		}

		// Mix data
		let mut data = self.data.lock();
		data.retain(|id, queue| {

			let len = std::cmp::min(buffer.len(), queue.len());
			let (a, b) = queue.as_slices();
			let alen = std::cmp::min(a.len(), len);
			buffer[..alen].copy_from_slice(&a[..alen]);
			if alen < len {
				buffer[alen..len].copy_from_slice(&b[..len - alen]);
			}

			if queue.len() == len {
				trace!(self.logger, "Remove playback queue buffer"; "id" => %id);
				false
			} else {
				*queue = queue.split_off(len);
				trace!(self.logger, "Left playback queue buffer"; "id" => %id, "len" => queue.len());
				true
			}
		});
	}
}
