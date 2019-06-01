use std::sync::Arc;

use actix_web::actix::*;
use failure::{format_err, Error};
use futures01::{Future, Sink};
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use gst::{gst_element_error, gst_element_warning};
use gst_audio::StreamVolumeExt;
use parking_lot::Mutex;
use slog::{debug, error, o, Logger};
use tsproto_packets::packets::{AudioData, CodecType, OutAudio};

use super::*;
use super::webrtc::WebrtcHandler;

pub struct SetListenerMsg {
	pub connection: tsclientlib::Connection,
}

pub struct RemoveListenerMsg;
pub struct SetVolumeMsg(pub f64);
pub struct SetPlayingMsg(pub bool);

pub struct AudioToTs {
	listeners: Arc<Mutex<Vec<ConnectionSinkCreator>>>,

	logger: Logger,
	bin: gst::Bin,
	volume: Option<gst_audio::StreamVolume>,
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
		if let Err(e) = self.set_playing(false) {
			error!(self.logger, "Failed to stop playing"; "error" => ?e);
		}
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
		self.set_playing(msg.0)
	}
}

impl AudioToTs {
	/// We need an explicit executor because we want to spawn new tasks in
	/// callbacks from gstreamer threads.
	pub fn new(
		logger: Logger,
		pipeline: gst::Pipeline,
		executor: ThreadPool,
		rtc: Option<&WebrtcHandler>,
		uri: Option<&str>,
	) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));
		// TODO Put everything into ts-to-audio bin and play/pause that
		let bin = gst::Bin::new(Some("ts-to-audio"));

		let sink = gst::ElementFactory::make("appsink", Some("appsink"))
			.ok_or_else(|| format_err!("Missing appsink"))?;
		bin.add(&sink)?;
		let volume;

		if let Some(rtc) = rtc {
			volume = None;
			let logger2 = logger.clone();
			let pipeline2 = pipeline.clone();
			let sink2 = sink.clone();
			let webrtc2 = rtc.webrtc.clone();
			rtc.webrtc.connect("pad-added", false, move |_| {
				debug!(logger2, "Webrtc pad added");
				let decodebin = gst::ElementFactory::make("decodebin", None).unwrap();
				let pipeline = pipeline2.clone();
				let logger = logger2.clone();
				let sink = sink2.clone();
				decodebin.connect("pad-added", false, move |values| {
					let pad = values[1].get::<gst::Pad>().expect("Invalid argument");
					if !pad.has_current_caps() {
						println!("Pad {:?} has no caps, can't do anything, ignoring", pad);
						return None;
					}

					let caps = pad.get_current_caps().unwrap();
					let name = caps.get_structure(0).unwrap().get_name();

					let handled = if name.starts_with("video") {
						Err(format_err!("Cannot handle video streams"))
					} else if name.starts_with("audio") {
						handle_audio_stream(&pad, &pipeline, &sink)
					} else {
						println!("Unknown pad {:?}, ignoring", pad);
						Ok(())
					};

					if let Err(err) = handled {
						error!(logger, "Error adding pad with caps"; "name" => name, "error" => ?err);
					}
					None
				})
				.unwrap();

				pipeline2.add(&decodebin).unwrap();

				decodebin.sync_state_with_parent().unwrap();
				webrtc2.link(&decodebin).unwrap();
				None
			})?;

		} else {
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

			opusenc.set_property_from_str("bitrate-type", "vbr");
			opusenc.set_property_from_str("audio-type", "voice"); // or generic
			// Discontinuous transmission: Reduce bandwidth of silence
			// Unfortunately creates artifacts
			//opusenc.set_property("dtx", &glib::Value::from(&true))?;
			// Inband forward error correction
			opusenc.set_property("inband-fec", &true)?;
			// Packetloss between 0 - 100
			opusenc.set_property("packet-loss-percentage", &0)?;

			bin.add_many(&[&decode, &resampler, &vol, &opusenc])?;
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

			volume = Some(vol.dynamic_cast::<gst_audio::StreamVolume>().unwrap());
		}

		pipeline.add(&bin)?;

		let appsink = sink.dynamic_cast::<gst_app::AppSink>().unwrap();
		appsink.set_caps(Some(&gst::Caps::new_simple("audio/x-opus",
			&[("channel-mapping-family", &0i32)])));

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
					let mut listeners = listeners2.lock();
					listeners.retain(|l| {
						if l.con.upgrade().is_none() {
							return false;
						}

						let sink = l.con.as_packet_sink();
						let logger = logger.clone();
						let packet = packet.clone();
						executor2.spawn(sink.send(packet).map(|_| ()).map_err(move |e| {
							error!(logger, "Failed to send packet"; "error" => ?e);
						})).detach();
						true
					});
					Ok(gst::FlowSuccess::Ok)
				})
				.build(),
		);

		Ok(Self {
			listeners,

			logger: logger2,
			bin,
			volume,
		})
	}

	pub fn set_volume(&self, volume: f64) {
		if let Some(v) = &self.volume {
			v.set_volume(gst_audio::StreamVolumeFormat::Linear, volume);
		}
		// TODO
	}

	/*pub fn is_playing(&self) -> Result<bool, failure::Error> {
		// Returns (success, current state, pending state)
		let state = self.bin.get_state(
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
	}*/

	pub fn set_playing(&self, playing: bool) -> Result<(), Error> {
		if playing {
			debug!(self.logger, "Change to playing");
			self.bin.set_state(gst::State::Playing)?;
		} else {
			debug!(self.logger, "Change to paused");
			self.bin.set_state(gst::State::Paused)?;
		}
		Ok(())
	}
}

fn handle_audio_stream(
	pad: &gst::Pad,
	pipe: &gst::Pipeline,
	appsink: &gst::Element,
) -> Result<(), Error> {
	let q = gst::ElementFactory::make("queue", None).unwrap();

	let conv = gst::ElementFactory::make("audioconvert", None).unwrap();
	let resample = gst::ElementFactory::make("audioresample", None).unwrap();
	let opusenc = gst::ElementFactory::make("opusenc", None).unwrap();
	opusenc.set_property_from_str("bitrate-type", "vbr");
	opusenc.set_property_from_str("audio-type", "voice"); // or generic
	// Discontinuous transmission: Reduce bandwidth of silence
	// Unfortunately creates artifacts
	//opusenc.set_property("dtx", &glib::Value::from(&true))?;
	// Inband forward error correction
	opusenc.set_property("inband-fec", &true)?;
	// Packetloss between 0 - 100
	opusenc.set_property("packet-loss-percentage", &0)?;


	pipe.add_many(&[&q, &conv, &resample, &opusenc])?;
	gst::Element::link_many(&[&q, &conv, &resample, &opusenc, appsink])?;

	resample.sync_state_with_parent()?;

	q.sync_state_with_parent()?;
	conv.sync_state_with_parent()?;
	opusenc.sync_state_with_parent()?;

	let qpad = q.get_static_pad("sink").unwrap();
	pad.link(&qpad)?;

	Ok(())
}
