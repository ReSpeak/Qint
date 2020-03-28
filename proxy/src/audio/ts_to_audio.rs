use std::cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd, Reverse};
use std::collections::hash_map::Entry;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, Instant};

use actix::*;
use anyhow::{bail, Result};
use audiopus::coder::Decoder;
use futures::prelude::*;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};
use tokio::sync::mpsc;
use tsclientlib::ClientId;
use tsproto_packets::packets::{AudioData, CodecType, InAudioBuf};

use super::*;
use crate::websocket::{SetTalkersMsg, Ws};
use crate::ConnectionId;

/// After this amount of seconds, a decoder will be removed.
const VOICE_TIMEOUT_SECS: u64 = 1;

pub struct PlayMsg(pub ConnectionId, pub InAudioBuf);
pub struct TalkersChangedMsg(pub ConnectionId);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct Id {
	con: ConnectionId,
	client: ClientId,
}

/// A decoded audio packet
struct AudioPacket {
	id: u16,
	data: Vec<f32>,
}

pub(crate) struct TsToAudio {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	spawn_send: mpsc::UnboundedSender<SendAudioEvent>,
	device: Option<AudioDevice<SdlCallback>>,
	/// For each client, store the opus decoder and the instant when it was last
	/// used.
	decoders: HashMap<Id, (Decoder, Instant)>,
	/// The audio queue, we always play the packet with the smallest id.
	data: Arc<Mutex<HashMap<Id, BinaryHeap<Reverse<AudioPacket>>>>>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
}

struct SdlCallback {
	logger: Logger,
	data: Arc<Mutex<HashMap<Id, BinaryHeap<Reverse<AudioPacket>>>>>,
	spawn_send: mpsc::UnboundedSender<SendAudioEvent>,
}

impl Message for PlayMsg {
	type Result = Result<()>;
}

impl Message for TalkersChangedMsg {
	type Result = ();
}

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}-{}", self.con.0, self.client.0)
	}
}

impl PartialEq for AudioPacket {
	fn eq(&self, other: &Self) -> bool { self.id == other.id }
}
impl Eq for AudioPacket {}

impl PartialOrd for AudioPacket {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl Ord for AudioPacket {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.id == other.id {
			Ordering::Equal
		} else if self.id.wrapping_sub(other.id) < std::u16::MAX / 2 {
			Ordering::Greater
		} else {
			Ordering::Less
		}
	}
}

impl Actor for TsToAudio {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.open_playback();

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			if !t2a.decoders.is_empty() {
				// Check for inactive connections
				let now = Instant::now();
				let dur = Duration::from_secs(VOICE_TIMEOUT_SECS);
				let logger = &t2a.logger;
				t2a.decoders.retain(|id, (_, last)| {
					if now.duration_since(*last) > dur {
						trace!(logger, "Removing stale connection"; "id" => %id);
						false
					} else {
						true
					}
				});

				if t2a.decoders.is_empty() {
					if let Some(device) = &t2a.device {
						debug!(logger, "Pausing playback");
						device.pause();
					}
				}
			}

			if t2a
				.device
				.as_ref()
				.map(|d| d.status() == AudioStatus::Stopped)
				.unwrap_or(true)
			{
				// Try to reconnect to audio
				t2a.open_playback();
			}
		});
	}
}

impl TsToAudio {
	pub(crate) fn new(
		logger: Logger, audio_subsystem: AudioSubsystem,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
		spawn_send: mpsc::UnboundedSender<SendAudioEvent>,
	) -> Result<Self>
	{
		let logger = logger.new(o!("pipeline" => "ts-to-audio"));
		let data = Arc::new(Mutex::new(Default::default()));

		Ok(Self {
			logger,
			audio_subsystem,
			device: None,
			decoders: Default::default(),
			data,
			connections,
			spawn_send,
		})
	}

	fn open_playback(&mut self) {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(2),
			samples: Some(USUAL_SAMPLE_COUNT as u16),
		};

		let logger = self.logger.clone();
		let data = self.data.clone();
		match self.audio_subsystem.open_playback(None, &desired_spec, |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			debug!(logger, "Got playback spec"; "spec" => ?spec, "driver" => self.audio_subsystem.current_audio_driver());
			SdlCallback {
				logger,
				data,
				spawn_send: self.spawn_send.clone(),
			}
		}) {
			Ok(device) => self.device = Some(device),
			Err(e) => {
				self.device = None;
				error!(self.logger, "Failed to open playback device"; "error" => ?e);
			}
		}
	}

	fn talkers_changed(&self, con_id: ConnectionId) {
		let con = {
			let cons = self.connections.lock().unwrap();
			if let Some(con) = cons.get(&con_id) {
				con.clone()
			} else {
				warn!(self.logger, "Failed to get connection for changed talkers"; "connection" => ?con_id);
				return;
			}
		};

		let talkers = self
			.data
			.lock()
			.unwrap()
			.keys()
			.filter(|id| id.con == con_id)
			.map(|id| id.client)
			.collect();
		let logger = self.logger.clone();
		tokio::spawn(con.send(SetTalkersMsg(talkers))
			.map(move |r| if let Err(e) = r {
				error!(logger, "Failed to notify connection about changed talkers"; "error" => ?e);
			}));
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<()>;
	fn handle(&mut self, msg: PlayMsg, _: &mut Self::Context) -> Self::Result {
		if self.device.is_none() {
			warn!(
				self.logger,
				"Unable to play audio packet, device is not initialized"
			);
			return Ok(());
		}

		if let AudioData::S2C { id: packet_id, from, codec, data }
		| AudioData::S2CWhisper { id: packet_id, from, codec, data } =
			msg.1.data().data()
		{
			if *codec != CodecType::OpusVoice && *codec != CodecType::OpusMusic
			{
				bail!("Got unsupported audio codec, only opus is supported");
			}

			let id = Id { con: msg.0, client: ClientId(*from) };
			if data.len() <= 5 {
				debug!(self.logger, "Got small audio packet"; "id" => %id);
				//decoder.reset_state()?;
				return Ok(());
			}

			let channels = self.device.as_ref().unwrap().spec().channels;
			let was_empty = self.decoders.is_empty();

			let mut tmp_entry;
			let decoder = match self.decoders.entry(id) {
				Entry::Occupied(o) => {
					tmp_entry = o;
					let entry = tmp_entry.get_mut();
					entry.1 = Instant::now();
					&mut entry.0
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
					let decoder = Decoder::new(
						audiopus::SampleRate::Hz48000,
						opus_channels,
					)?;
					&mut v.insert((decoder, Instant::now())).0
				}
			};

			let mut opus_output = vec![0f32; USUAL_FRAME_SIZE];
			let len = loop {
				match decoder.decode_float(Some(*data), &mut opus_output, false)
				{
					Ok(len) => break len,
					Err(audiopus::error::Error::Opus(
						audiopus::error::ErrorCode::BufferTooSmall,
					)) => {
						// Enlarge the buffer
						if opus_output.len() == MAX_FRAME_SIZE {
							bail!(
								"Bad opus packet, maximum buffer size exceeded"
							);
						} else if opus_output.len() * 2 > MAX_FRAME_SIZE {
							opus_output.resize(MAX_FRAME_SIZE, 0f32);
						} else {
							opus_output.resize(opus_output.len() * 2, 0f32);
						}
					}
					Err(e) => bail!("Error: {:?}, data: {:?}", e, data),
				}
			};

			// Shrink the buffer
			let size = len * usize::from(channels);
			if size <= opus_output.len() / 2 {
				opus_output.truncate(len);
			}
			trace!(self.logger, "Decoded opus packet"; "id" => %id, "len" => len);

			// Put into queue
			let changed;
			{
				let mut data = self.data.lock().unwrap();
				let entry = data.entry(id);
				// If new, fire change talkers event
				changed =
					if let Entry::Vacant(_) = &entry { true } else { false };

				let queue = entry.or_insert_with(Default::default);
				if queue.len() > 4 {
					debug!(self.logger, "Removing packets from playback queue"; "id" => %id, "count" => queue.len() - 2);
					while queue.len() > 4 {
						queue.pop();
					}
				}
				queue.push(Reverse(AudioPacket {
					id: *packet_id,
					data: opus_output,
				}))
			}

			if changed {
				self.talkers_changed(id.con);
			}

			if was_empty {
				debug!(self.logger, "Resuming playback");
				self.device.as_ref().unwrap().resume();
			}
		}
		Ok(())
	}
}

impl Handler<TalkersChangedMsg> for TsToAudio {
	type Result = ();
	fn handle(
		&mut self, msg: TalkersChangedMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.talkers_changed(msg.0);
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

		// All connections where the talkers changed
		let mut changed = HashSet::new();
		// Mix data
		{
			let mut data = self.data.lock().unwrap();
			data.retain(|id, queue| {
				if queue.is_empty() {
					changed.insert(id.con);
					debug!(self.logger, "Remove playback queue buffer"; "id" => %id);
					return false;
				}

				let mut i = 0;
				while i < buffer.len() {
					let mut q_packet = if let Some(p) = queue.peek_mut() {
						p
					} else {
						break;
					};
					let packet = &mut q_packet.0;

					let len = std::cmp::min(packet.data.len(), buffer.len() - i);
					for j in 0..len {
						buffer[i + j] += packet.data[j];
					}
					trace!(self.logger, "Add from buffer"; "id" => %id, "from" => i, "to" => i + len);
					i += len;

					if packet.data.len() > len {
						packet.data = packet.data.split_off(len);
					} else {
						drop(q_packet);
						queue.pop();
					}
				}

				trace!(self.logger, "Left playback queue buffer"; "id" => %id, "packets" => queue.len());
				true
			});
		}

		for con in changed {
			self.spawn_send.send(SendAudioEvent::TalkersChanged(con)).unwrap();
		}
	}
}
