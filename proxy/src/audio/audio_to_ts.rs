use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, AtomicU64, Ordering};

use anyhow::Result;
use audiopus::coder::Encoder;
use ebur128::EbuR128;
use nnnoiseless::DenoiseState;
use tokio::sync::mpsc;
use tracing::{Span, debug, error, info_span, trace, warn};
use tsproto_packets::packets::CodecType;

use super::*;
use crate::connection::{CaptureLoudnessMsg, QintConnection, SendAudioMsg, SetSelfTalkingMsg};
use crate::with_log;

pub trait LoudnessTrait {
	fn send(&self, msg: CaptureLoudnessMsg);
	fn connected(&self) -> bool;
}
pub type LoudnessListener = Box<dyn LoudnessTrait + Send>;

pub struct AddListenerMsg(pub Addr<QintConnection>);
pub struct RemoveListenerMsg(pub Addr<QintConnection>);
pub struct AddLoudnessListenerMsg(pub LoudnessListener);
pub struct RemoveLoudnessListenerMsg(pub usize);
pub struct SetPacketlossMsg(pub f32);
/// An audio packet and `true` if this is the last packet.
#[derive(Debug, Default)]
pub struct PlayPacketMsg {
	codec: Option<CodecType>,
	data: Option<Vec<u8>>,
	is_end: bool,
	loudness: Option<f64>,
	vad: Option<f32>,
}
pub struct SetLoudnessThresholdMsg(pub f64);
pub struct SetVadThresholdMsg(pub f32);

/// Threshold for voice activation detection.
const DEFAULT_VAD_THRESHOLD: f32 = 0.3;
/// The default minimum loudness for voice activation detection.
const DEFAULT_LOUDNESS_THRESHOLD: f64 = -50.0;

/// How many packets should still be sent after the voice detection is under the
/// threshold.
const TALKING_TIME: u8 = 5;

/// Magic value sent if the client stopped talking.
const LOUDNESS_END_MAGIC: f64 = -1000.0;

pub trait AudioToTsImpl: Unpin {
	fn started(ts_to_audio: &mut AudioToTs<Self>, ctx: &mut Context<AudioToTs<Self>>)
	where Self: Sized + 'static;

	/// Re-open the playback device.
	fn reset(ts_to_audio: &mut AudioToTs<Self>)
	where Self: Sized;

	/// To pause or unpause capturing, i.e. on mute or unmute
	fn set_playing(ts_to_audio: &mut AudioToTs<Self>, playing: bool)
	where Self: Sized + 'static;

	fn get_audio_devices(ts_to_audio: &mut AudioToTs<Self>) -> Vec<String>
	where Self: Sized;
}

pub struct AudioToTs<Impl> {
	pub preferred_device: Option<String>,
	spawn_send: mpsc::Sender<PlayPacketMsg>,
	connections: HashSet<Addr<QintConnection>>,
	loudness_cons: HashMap<usize, LoudnessListener>,
	/// When enabled will send all loudness packets regardless of their activation
	/// values.
	loudness_listening: Arc<AtomicBool>,
	loudness_id_cnt: usize,
	/// The loudness threshold in LUFS (Loudness Unit Full Scale).
	///
	/// This is actually a `f64`, there is no `AtomicF64` though.
	loudness_threshold: Arc<AtomicU64>,
	/// The voice activation detection threshold [0.0 ≤ vad ≤ 1.0].
	///
	/// This is actually a `f32`, there is no `AtomicF32` though.
	vad_threshold: Arc<AtomicU32>,
	/// Packet loss in percent, 0-100.
	packet_loss: Arc<AtomicU8>,

	/// If we are actually talking and sending audio
	is_talking: bool,

	pub real_impl: Impl,
}

pub struct AudioToTsCallback {
	span: Span,
	channels: audiopus::Channels,
	encoder: Option<Encoder>,
	denoise: Box<DenoiseState<'static>>,
	denoise_buffer: [f32; DenoiseState::FRAME_SIZE],
	vad_threshold: Arc<AtomicU32>,
	loudness: Option<EbuR128>,
	loudness_threshold: Arc<AtomicU64>,
	loudness_listening: Arc<AtomicBool>,
	packet_loss: Arc<AtomicU8>,
	opus_output: [u8; MAX_OPUS_FRAME_SIZE],
	/// The last captured buffer if we are not talking.
	///
	/// We keep one and if we start talking, we encode and send this first. This
	/// ensures a smoother start.
	/// Empty if we are currently sending.
	last_buffer: Vec<f32>,
	last_buffer_is_upscaled: bool,
	/// If we are actually talking and sending audio.
	///
	/// This is `TALKING_TIME + 1` if voice activation triggers and greater 0 if
	/// packets should be sent.
	is_talking: u8,

	spawn_send: mpsc::Sender<PlayPacketMsg>,
}

impl<Impl: AudioToTsImpl + 'static> Actor for AudioToTs<Impl> {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) { Impl::started(self, ctx); }
}

impl Message for AddListenerMsg {
	type Result = ();
}
impl Message for RemoveListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for AddLoudnessListenerMsg {
	type Result = usize;
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
impl Message for SetVadThresholdMsg {
	type Result = ();
}

impl<Impl: AudioToTsImpl + 'static> Handler<AddListenerMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(&mut self, msg: AddListenerMsg, _: &mut Self::Context) -> Self::Result {
		self.connections.insert(msg.0.clone());
		self.update_device_state();
		if self.is_talking {
			// Update is_talking for this connection
			actix::spawn(with_log!(
				msg.0.send(SetSelfTalkingMsg(self.is_talking)),
				"Failed to set self talking status"
			));
		}
		debug!("Add listener");
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<RemoveListenerMsg> for AudioToTs<Impl> {
	type Result = bool;
	fn handle(&mut self, msg: RemoveListenerMsg, _: &mut Self::Context) -> Self::Result {
		debug!("Removing listener");
		if self.is_talking {
			// Update is_talking for this connection
			actix::spawn(with_log!(
				msg.0.send(SetSelfTalkingMsg(false)),
				"Failed to set self talking status"
			));
		}
		let r = self.connections.remove(&msg.0);
		self.update_device_state();
		r
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<AddLoudnessListenerMsg> for AudioToTs<Impl> {
	type Result = usize;
	fn handle(&mut self, msg: AddLoudnessListenerMsg, _: &mut Self::Context) -> Self::Result {
		self.loudness_id_cnt += 1;
		let id = self.loudness_id_cnt;
		self.loudness_cons.insert(id, msg.0);
		self.loudness_listening.store(true, Ordering::Relaxed);
		self.update_device_state();
		id
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<RemoveLoudnessListenerMsg> for AudioToTs<Impl> {
	type Result = bool;
	fn handle(&mut self, msg: RemoveLoudnessListenerMsg, _: &mut Self::Context) -> Self::Result {
		let r = self.loudness_cons.remove(&msg.0);
		self.loudness_listening.store(self.loudness_cons.len() > 0, Ordering::Relaxed);
		self.update_device_state();
		r.is_some()
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<SetPacketlossMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(&mut self, msg: SetPacketlossMsg, _: &mut Self::Context) -> Self::Result {
		self.packet_loss.store((msg.0 * 100.0) as u8, Ordering::Relaxed);
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<PlayPacketMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(&mut self, packet: PlayPacketMsg, _: &mut Self::Context) -> Self::Result {
		// Write into packet sink
		let loudness = packet.loudness;
		let vad = packet.vad.unwrap_or(0f32);

		if let (Some(data), Some(codec)) = (packet.data, packet.codec) {
			self.connections.retain(|con| {
				if !con.connected() {
					false
				} else {
					actix::spawn(with_log!(
						con.send(SendAudioMsg(codec, data.clone())),
						"Failed to send audio packet"
					));

					if let Some(loudness) = loudness {
						actix::spawn(with_log!(
							con.send(CaptureLoudnessMsg(loudness, vad)),
							"Failed to send loudness"
						));
					}

					true
				}
			});

			if self.is_talking != !packet.is_end {
				self.is_talking = !packet.is_end;
				self.update_talking();
			}
		}

		if let Some(loudness) = loudness {
			self.loudness_cons.retain(|_, con| {
				if !con.connected() {
					false
				} else {
					con.send(CaptureLoudnessMsg(loudness, vad));
					true
				}
			});
			self.loudness_listening.store(self.loudness_cons.len() > 0, Ordering::Relaxed);
		}

		self.update_device_state();
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<SetLoudnessThresholdMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(
		&mut self, SetLoudnessThresholdMsg(thres): SetLoudnessThresholdMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.loudness_threshold.store(thres.to_bits(), Ordering::Relaxed);
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<SetVadThresholdMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(
		&mut self, SetVadThresholdMsg(thres): SetVadThresholdMsg, _: &mut Self::Context,
	) -> Self::Result {
		self.vad_threshold.store(thres.to_bits(), Ordering::Relaxed);
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<ResetMsg> for AudioToTs<Impl> {
	type Result = ();
	fn handle(&mut self, _: ResetMsg, _: &mut Self::Context) -> Self::Result { Impl::reset(self); }
}

impl<Impl: AudioToTsImpl + 'static> Handler<GetAudioDevices> for AudioToTs<Impl> {
	type Result = Vec<String>;
	fn handle(&mut self, _: GetAudioDevices, _: &mut Self::Context) -> Self::Result {
		Impl::get_audio_devices(self)
	}
}

impl<Impl: AudioToTsImpl + 'static> Handler<SetAudioDevice> for AudioToTs<Impl> {
	type Result = ();
	fn handle(&mut self, set: SetAudioDevice, _: &mut Self::Context) -> Self::Result {
		if self.preferred_device != set.0 {
			self.preferred_device = set.0;
			Impl::reset(self);
		}
	}
}

impl<Impl: AudioToTsImpl + 'static> AudioToTs<Impl> {
	pub(crate) fn new(
		real_impl: Impl, preferred_device: Option<String>, spawn_send: mpsc::Sender<PlayPacketMsg>,
	) -> Self {
		Self {
			preferred_device,
			spawn_send,
			connections: Default::default(),
			loudness_cons: Default::default(),
			loudness_id_cnt: 0,
			loudness_threshold: Arc::new(AtomicU64::new(DEFAULT_LOUDNESS_THRESHOLD.to_bits())),
			vad_threshold: Arc::new(AtomicU32::new(DEFAULT_VAD_THRESHOLD.to_bits())),
			loudness_listening: Arc::new(AtomicBool::new(false)),
			packet_loss: Arc::new(AtomicU8::new(0)),
			is_talking: false,

			real_impl,
		}
	}

	pub fn get_callback(&self, channels: audiopus::Channels) -> AudioToTsCallback {
		AudioToTsCallback::new(
			channels,
			self.spawn_send.clone(),
			self.vad_threshold.clone(),
			self.loudness_threshold.clone(),
			self.loudness_listening.clone(),
			self.packet_loss.clone(),
		)
	}

	fn update_talking(&self) {
		for con in &self.connections {
			actix::spawn(with_log!(
				con.send(SetSelfTalkingMsg(self.is_talking)),
				"Failed to update self talking status"
			));
		}
	}

	pub fn update_device_state(&mut self) {
		Impl::set_playing(self, !self.connections.is_empty() || !self.loudness_cons.is_empty());
	}
}

impl AudioToTsCallback {
	fn new(
		channels: audiopus::Channels, spawn_send: mpsc::Sender<PlayPacketMsg>,
		vad_threshold: Arc<AtomicU32>, loudness_threshold: Arc<AtomicU64>,
		loudness_listening: Arc<AtomicBool>, packet_loss: Arc<AtomicU8>,
	) -> Self {
		let loudness = match EbuR128::new(1, super::SAMPLE_RATE as u32, ebur128::Mode::M) {
			Ok(r) => Some(r),
			Err(error) => {
				warn!(%error, "Failed to create loudness measurement");
				None
			}
		};
		Self {
			span: info_span!("audio-to-ts"),
			channels,
			encoder: None,
			denoise: DenoiseState::new(),
			denoise_buffer: [0.0; DenoiseState::FRAME_SIZE],
			vad_threshold,
			loudness,
			loudness_threshold,
			loudness_listening,
			packet_loss,
			opus_output: [0; MAX_OPUS_FRAME_SIZE],
			last_buffer: Default::default(),
			last_buffer_is_upscaled: false,
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

	fn send_audio(&mut self, packet: PlayPacketMsg) {
		if let Err(error) = self.spawn_send.try_send(packet) {
			warn!(%error, "Failed to send audio packet");
		}
	}

	fn measure_loudness(&mut self, buffer: &[f32]) -> Option<f64> {
		if let Some(ebur128) = &mut self.loudness {
			if let Err(error) = ebur128.add_frames_f32(buffer) {
				warn!(%error, "Failed to measure loudness with new data");
			} else {
				match ebur128.loudness_momentary() {
					Err(error) => {
						warn!(%error, "Failed to measure loudness");
					}
					Ok(lufs) => return Some(lufs),
				}
			}
		}
		None
	}

	pub fn callback(&mut self, buffer: &[f32]) {
		let mut data = buffer.to_vec();
		self.callback_mut_buffer(&mut data);
	}

	pub fn callback_mut_buffer(&mut self, buffer: &mut [f32]) {
		let _span = self.span.clone().entered();
		let did_talk = self.is_talking != 0;
		let is_loudness_listening = self.loudness_listening.load(Ordering::Relaxed);
		let mut loudness = None;
		let mut loudness_triggered = true;
		let mut vad = None;
		let vad_triggered;
		let mut is_upscaled = false;

		// Denoise
		if buffer.len() % DenoiseState::FRAME_SIZE != 0 {
			warn!("Size not fitting for denoising");
			vad_triggered = true;
		} else {
			// Scale to the expected range
			for d in &mut *buffer {
				*d *= i16::max_value() as f32;
			}
			is_upscaled = true;

			let mut vad_probe = 0.0;
			for i in buffer.chunks_mut(DenoiseState::FRAME_SIZE) {
				vad_probe += self.denoise.process_frame(&mut self.denoise_buffer, i);
				i.copy_from_slice(&self.denoise_buffer);
			}
			vad_probe /= (buffer.len() / DenoiseState::FRAME_SIZE) as f32;
			trace!(%vad_probe);
			vad = Some(vad_probe);
			vad_triggered = vad_probe >= f32::from_bits(self.vad_threshold.load(Ordering::Relaxed));
		}

		if vad_triggered || self.is_talking > 1 || is_loudness_listening {
			if is_upscaled {
				for d in &mut *buffer {
					*d /= i16::max_value() as f32;
				}
				is_upscaled = false;
			}

			// Additionally measure loudness, it has to be over the threshold
			if let Some(lufs) = self.measure_loudness(buffer) {
				loudness = Some(lufs);
				loudness_triggered =
					lufs >= f64::from_bits(self.loudness_threshold.load(Ordering::Relaxed));
			}
		}

		let should_talk = vad_triggered && loudness_triggered;
		if should_talk {
			self.is_talking = TALKING_TIME + 1;
		} else {
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
				trace!("Sending last empty packet");
				self.send_audio(PlayPacketMsg {
					codec: Some(codec),
					data: Some(Vec::new()),
					is_end: true,
					loudness: Some(LOUDNESS_END_MAGIC),
					vad,
				});
			} else if is_loudness_listening && (loudness.is_some() || vad.is_some()) {
				self.send_audio(PlayPacketMsg { loudness, vad, ..Default::default() });
			}
			self.last_buffer.resize(buffer.len(), 0.0);
			self.last_buffer.copy_from_slice(buffer);
			self.last_buffer_is_upscaled = is_upscaled;
			self.encoder = None;
			return;
		}

		if let Err(error) = self.create_encoder() {
			error!(%error, "Failed to create opus encoder");
			return;
		}

		// Update packet loss
		let loss = self.packet_loss.load(Ordering::Relaxed);
		let encoder = self.encoder.as_mut().unwrap();
		if loss == 0 {
			if let Err(error) = encoder.set_inband_fec(false) {
				warn!(%error, "Failed to disable opus inband fec");
			}
		} else {
			if let Err(error) = encoder.set_packet_loss_perc(loss) {
				warn!(%error, loss, "Failed to set opus packet loss");
			}
			if let Err(error) = encoder.set_inband_fec(true) {
				warn!(%error, "Failed to enable opus inband fec");
			}
		}

		if !did_talk {
			// Send cached last buffer if there was one
			if !self.last_buffer.is_empty() {
				trace!("Start to talk: Sending cached last buffer");
				if self.last_buffer_is_upscaled {
					for d in &mut self.last_buffer {
						*d /= i16::max_value() as f32;
					}
				}
				match encoder.encode_float(&self.last_buffer, &mut self.opus_output[..]) {
					Err(error) => {
						warn!(%error, "Failed to encode opus");
					}
					Ok(len) => {
						self.send_audio(PlayPacketMsg {
							codec: Some(codec),
							data: Some(self.opus_output[..len].to_vec()),
							is_end: false,
							loudness: None,
							vad: None,
						});
					}
				}
				self.last_buffer.clear();
			}
		}

		assert!(!is_upscaled);
		let packet =
			match self.encoder.as_ref().unwrap().encode_float(buffer, &mut self.opus_output[..]) {
				Err(error) => {
					warn!(%error, "Failed to encode opus");
					None
				}
				Ok(len) => {
					trace!("Sending packet");
					Some(self.opus_output[..len].to_vec())
				}
			};

		// TODO: consider not sending the packed at all if we coudn't encode the audio data
		// otherwise this could result in confusing visuals showing playback but not sending
		// anything
		if packet.is_some() || loudness.is_some() || vad.is_some() {
			self.send_audio(PlayPacketMsg {
				codec: Some(codec),
				data: packet,
				is_end: false,
				loudness,
				vad,
			});
		}
	}
}
