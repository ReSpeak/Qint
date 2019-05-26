use std::fmt::Debug;
use std::sync::{Arc, Weak};

use actix_web::actix::*;
use failure::{format_err, Error};
use futures::compat::*;
use futures::executor::ThreadPool;
use gst::{gst_element_error, gst_element_warning};
use gst_audio::StreamVolumeExt;
use parking_lot::Mutex;
use slog::{debug, error, o, Logger};
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutPacket};

use super::*;

pub struct SetListenerMsg {
	connection: tsclientlib::Connection,
}

pub struct RemoveListenerMsg;
pub struct SetVolumeMsg(f64);
pub struct SetPlayingMsg(bool);

pub struct AudioToTs {
	listeners: Arc<Mutex<Vec<ConnectionSinkCreator>>>,

	logger: Logger,
	executor: ThreadPool,
	pipeline: gst::Pipeline,
	volume: gst_audio::StreamVolume,
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
	fn handle(&mut self, msg: SetListenerMsg, ctx: &mut Self::Context) -> Self::Result {
		let mut listeners = self.listeners.lock();
		*listeners = vec![ConnectionSinkCreator {
			con: msg.connection.get_tsproto_connection(),
		}];
	}
}

impl Handler<RemoveListenerMsg> for AudioToTs {
	type Result = bool;
	fn handle(&mut self, msg: RemoveListenerMsg, ctx: &mut Self::Context) -> Self::Result {
		let mut ls = self.listeners.lock();
		let res = !ls.is_empty();
		ls.clear();
		if let Err(e) = self.set_playing(false) {
			error!(self.logger, "Failed to stop playing"; "error" => ?e);
		}
		res
	}
}

impl Handler<SetVolumeMsg> for AudioToTs {
	type Result = ();
	fn handle(&mut self, msg: SetVolumeMsg, ctx: &mut Self::Context) -> Self::Result {
		self.set_volume(msg.0);
	}
}

impl Handler<SetPlayingMsg> for AudioToTs {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: SetPlayingMsg, ctx: &mut Self::Context) -> Self::Result {
		self.set_playing(msg.0)
	}
}

impl AudioToTs {
	/// We need an explicit executor because we want to spawn new tasks in callbacks
	/// from gstreamer threads.
	pub fn new(
		logger: Logger,
		mut executor: ThreadPool,
		uri: Option<&str>,
	) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));
		let pipeline = gst::Pipeline::new(Some("audio-to-ts-pipeline"));

		let decode;
		if let Some(uri) = &uri {
			decode = gst::ElementFactory::make("uridecodebin", Some("decode"))
				.ok_or_else(|| format_err!("Missing uridecodebin"))?;
			decode.set_property("uri", uri)?;
		} else {
			decode = gst::ElementFactory::make("autoaudiosrc", Some("audiosrc"))
				.ok_or_else(|| format_err!("Missing autoaudiosrc"))?;
		}

		let resampler = gst::ElementFactory::make("audioresample", Some("resample"))
			.ok_or_else(|| format_err!("Missing audioresample"))?;

		let vol = gst::ElementFactory::make("volume", Some("vol"))
			.ok_or_else(|| format_err!("Missing volume"))?;

		let opusenc = gst::ElementFactory::make("opusenc", Some("opusenc"))
			.ok_or_else(|| format_err!("Missing opusenc"))?;
		let sink = gst::ElementFactory::make("appsink", Some("appsink"))
			.ok_or_else(|| format_err!("Missing appsink"))?;

		opusenc.set_property_from_str("bitrate-type", "vbr");
		opusenc.set_property_from_str("audio-type", "voice"); // or generic
		// Discontinuous transmission: Reduce bandwidth of silence
		// Unfortunately creates artifacts
		//opusenc.set_property("dtx", &glib::Value::from(&true))?;
		// Inband forward error correction
		opusenc.set_property("inband-fec", &true)?;
		// Packetloss between 0 - 100
		opusenc.set_property("packet-loss-percentage", &0)?;

		pipeline.add_many(&[&decode, &resampler, &vol, &opusenc, &sink])?;
		gst::Element::link_many(&[&resampler, &vol, &opusenc, &sink])?;
		if uri.is_none() {
			decode.link(&resampler)?;
		}

		// Link decode to next element if a pad gets available
		let next = resampler;
		let logger2 = logger.clone();
		decode.connect_pad_added(move |dbin, src_pad| {
			debug!(logger2, "Got new pad"; "name" => src_pad.get_name().as_str());
			let is_audio = src_pad.get_current_caps().and_then(|caps| {
				caps.get_structure(0).map(|s| {
					debug!(logger2, "Capabilities"; "name" => src_pad.get_name().as_str(),
						"caps" => ?s);
					s.get_name().starts_with("audio/")
				})
			});

			let is_audio = if let Some(is_audio) = is_audio {
				is_audio
			} else {
				gst_element_warning!(
					dbin,
					gst::CoreError::Negotiation,
					("Failed to get media type from pad {}", src_pad.get_name())
				);
				return;
			};
			if !is_audio {
				return;
			}

			// Link to sink pad of next element
			let sink_pad = next
				.get_static_pad("sink")
				.expect("Next element has no sink pad");
			if let Err(error) = src_pad.link(&sink_pad) {
				error!(logger2, "Cannot link pads"; "error" => ?error);
				gst_element_error!(
					dbin,
					gst::ResourceError::Failed,
					("Failed to link decoder")
				);
			}
		});

		let streamvolume = vol.dynamic_cast::<gst_audio::StreamVolume>().unwrap();

		let appsink = sink.dynamic_cast::<gst_app::AppSink>().unwrap();

		let listeners = Arc::new(Mutex::new(Vec::<ConnectionSinkCreator>::new()));

		let logger2 = logger.clone();
		let executor2 = executor.clone();
		let listeners2 = listeners.clone();
		appsink.set_callbacks(
			gst_app::AppSinkCallbacks::new()
				.new_sample(move |appsink| {
					let sample = match appsink.pull_sample() {
						None => return Err(gst::FlowError::Eos),
						Some(sample) => sample,
					};

					let buffer = if let Some(buffer) = sample.get_buffer() {
						buffer
					} else {
						gst_element_error!(
							appsink,
							gst::ResourceError::Failed,
							("Failed to get buffer from appsink")
						);

						return Err(gst::FlowError::Error);
					};

					let map = if let Some(map) = buffer.map_readable() {
						map
					} else {
						gst_element_error!(
							appsink,
							gst::ResourceError::Failed,
							("Failed to map buffer readable")
						);

						return Err(gst::FlowError::Error);
					};

					// Create packet
					let packet = OutAudio::new(&AudioData::C2S {
						id: 0,
						codec: CodecType::OpusMusic,
						data: map.as_slice(),
					});

					// Write into packet sink
					let logger2 = logger.clone();
					let listeners = listeners2.lock();
					for l in &*listeners {
						let sink = l.con.as_packet_sink().sink_compat();
						let logger = logger.clone();
						let packet = packet.clone();
						executor2.clone().spawn(async {
							let logger = logger;
							let mut sink = sink;
							let r = sink.send(packet).await;
							if let Err(e) = r {
								error!(logger, "Failed to send packet"; "error" => ?e);
							}
						}).expect("Failed to start");
					}
					Ok(gst::FlowSuccess::Ok)
				})
				.build(),
		);

		// Run event handler in background
		executor.spawn(main_loop(&pipeline, logger2.clone())).unwrap();

		Ok(Self {
			listeners,

			logger: logger2,
			executor,
			pipeline,
			volume: streamvolume,
		})
	}

	pub fn set_volume(&self, volume: f64) {
		self.volume.set_volume(gst_audio::StreamVolumeFormat::Linear, volume);
	}

	pub fn is_playing(&self) -> Result<bool, failure::Error> {
		// Returns (success, current state, pending state)
		let state = self.pipeline.get_state(
			gst::ClockTime::from_mseconds(10),
		);

		if state.0.is_ok() {
			if state.1 == gst::State::Playing {
				Ok(true)
			} else if state.1 == gst::State::Paused {
				Ok(false)
			} else {
				Err(format_err!("State is neither playing nor paused ({:?})", state.1))
			}
		} else {
			Err(format_err!("Failed to get current state ({:?})", state))
		}
	}

	pub fn set_playing(&self, playing: bool) -> Result<(), Error> {
		if playing {
			debug!(self.logger, "Change to playing");
			self.pipeline.set_state(gst::State::Playing)?;
		} else {
			debug!(self.logger, "Change to paused");
			self.pipeline.set_state(gst::State::Paused)?;
		}
		Ok(())
	}
}

impl Drop for AudioToTs {
	fn drop(&mut self) {
		// Cleanup gstreamer
		self.pipeline.set_state(gst::State::Null).expect(
			"Failed to shutdown gstreamer pipeline");
	}
}
