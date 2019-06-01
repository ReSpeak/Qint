//! A nice and simple explanation of how you create a WebRTC connection:
//! [https://shanetully.com/2014/09/a-dead-simple-webrtc-example/](https://shanetully.com/2014/09/a-dead-simple-webrtc-example/)
//!
//! Examples for WebRTC with gstreamer in Rust can be found here:
//! [https://github.com/centricular/gstwebrtc-demos](https://github.com/centricular/gstwebrtc-demos)

use actix_web::actix::*;
use failure::{format_err, Error};
use futures::executor::ThreadPool;
use futures::prelude::*;
use futures::task::SpawnExt;
use futures::compat::Future01CompatExt;
use gstreamer as gst;
use gstreamer_webrtc as gst_webrtc;
use gstreamer_sdp as gst_sdp;
use gst::prelude::*;
use lazy_static::lazy_static;
use qint_shared::*;
use slog::{error, Logger};

// TODO Add mozilla
//  {'iceServers': [{'url': 'stun:stun.services.mozilla.com'}, {'url': 'stun:stun.l.google.com:19302'}]};
const STUN_SERVER: &str = "stun://stun.l.google.com:19302";
lazy_static! {
	static ref RTP_CAPS_OPUS: gst::Caps = {
		gst::Caps::new_simple(
			"application/x-rtp",
			&[
				("media", &"audio"),
				("encoding-name", &"OPUS"),
				("payload", &(97i32)),
			],
		)
	};
	static ref RTP_CAPS_VP8: gst::Caps = {
		gst::Caps::new_simple(
			"application/x-rtp",
			&[
				("media", &"video"),
				("encoding-name", &"VP8"),
				("payload", &(96i32)),
			],
		)
	};
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MediaType {
	Audio,
	Video,
}

pub struct WebrtcHandler {
	logger: Logger,
	executor: ThreadPool,
	pipeline: gst::Pipeline,
	webrtc: gst::Element,
}

/// The 'signalling' channel is the websocket which is also used for the rest of
/// the communication.
#[derive(Clone, Debug)]
pub struct SignallingMsg(pub WebrtcMsg);

/// This message has to be sent to setup the webrtc element.
pub(crate) struct SetupMsg(pub Addr<crate::Ws>);

impl Actor for WebrtcHandler {
	type Context = Context<Self>;
}

impl Message for SignallingMsg { type Result = (); }
impl Message for SetupMsg { type Result = Result<(), Error>; }

impl Handler<SignallingMsg> for WebrtcHandler {
	type Result = ();
	fn handle(&mut self, msg: SignallingMsg, _: &mut Self::Context) -> Self::Result {
		error!(self.logger, "Got {:?}", msg);
		match msg.0 {
			WebrtcMsg::Ice { candidate, sdp_mline_index } =>
				self.handle_ice(&candidate, sdp_mline_index),
			WebrtcMsg::Sdp { typ, sdp } => self.handle_sdp(&typ, &sdp),
		}
	}
}

impl Handler<SetupMsg> for WebrtcHandler {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: SetupMsg, ctx: &mut Self::Context) -> Self::Result {
		let executor = self.executor.clone();
		let ws_addr = msg.0.clone();
		let webrtc = self.webrtc.clone();
		let logger = self.logger.clone();
		self.webrtc.connect("on-negotiation-needed", false, move |_| {
			println!("Needs negotiation");
			let webrtc2 = webrtc.clone();
			let mut executor = executor.clone();
			let ws_addr = ws_addr.clone();
			let logger = logger.clone();
			let promise = gst::Promise::new_with_change_func(move |promise| {
				let reply = promise.get_reply().unwrap();

				let offer = reply
					.get_value("offer")
					.unwrap()
					.get::<gst_webrtc::WebRTCSessionDescription>()
					.expect("Invalid argument");
				webrtc2.emit("set-local-description", &[&offer, &None::<gst::Promise>]).unwrap();

				let logger = logger.clone();
				println!("Send {}", offer.get_sdp().as_text().unwrap());
				/*executor.spawn(*/ws_addr.do_send(crate::WsMessage::Message(
					MessageP2F::Webrtc(WebrtcMsg::Sdp {
						typ: "offer".to_string(),
						sdp: offer.get_sdp().as_text().unwrap(),
					})));/*.compat().map(move |r| {
						if let Err(e) = r {
							error!(logger, "Failed to send webrtc message"; "error" => ?e);
						}
					})).unwrap();*/
			});

			webrtc.emit("create-offer", &[&None::<gst::Structure>, &promise]).unwrap();
			None
		})?;

		let executor = self.executor.clone();
		let ws_addr = msg.0.clone();
		let logger = self.logger.clone();
		self.webrtc.connect("on-ice-candidate", false, move |values| {
			println!("Got ice");
			let _webrtc = values[0].get::<gst::Element>().expect("Invalid argument");
			let mlineindex = values[1].get::<u32>().expect("Invalid argument");
			let candidate = values[2].get::<String>().expect("Invalid argument");

			let logger2 = logger.clone();
			let mut executor = executor.clone();
			// Ignore failure when the websocket connection is gone
			/*executor.spawn(*/ws_addr.do_send(crate::WsMessage::Message(
				MessageP2F::Webrtc(WebrtcMsg::Ice {
					candidate,
					sdp_mline_index: mlineindex,
				})));/*.compat().map(move |r| {
					if let Err(e) = r {
						error!(logger2, "Failed to send webrtc message"; "error" => ?e);
					}
				})).unwrap();*/
			None
		})?;

		let pipeline = self.pipeline.clone();
		let webrtc = self.webrtc.clone();
		let logger = self.logger.clone();
		self.webrtc.connect("pad-added", false, move |_| {
			println!("Webrtc pad added");
			let decodebin = gst::ElementFactory::make("decodebin", None).unwrap();
			let pipeline2 = pipeline.clone();
			let logger = logger.clone();
			decodebin
				.connect("pad-added", false, move |values| {
					let pad = values[1].get::<gst::Pad>().expect("Invalid argument");
					if !pad.has_current_caps() {
						println!("Pad {:?} has no caps, can't do anything, ignoring", pad);
						return None;
					}

					let caps = pad.get_current_caps().unwrap();
					let name = caps.get_structure(0).unwrap().get_name();

					let handled = if name.starts_with("video") {
						handle_media_stream(&pad, &pipeline2, MediaType::Video)
					} else if name.starts_with("audio") {
						handle_media_stream(&pad, &pipeline2, MediaType::Audio)
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

			pipeline.add(&decodebin).unwrap();

			decodebin.sync_state_with_parent().unwrap();
			webrtc.link(&decodebin).unwrap();
			None
		})?;

		// TODO
		let webrtc = self.webrtc.clone();
			let webrtc2 = webrtc.clone();
			let mut executor = self.executor.clone();
			let ws_addr = msg.0.clone();
			let logger = self.logger.clone();
			let promise = gst::Promise::new_with_change_func(move |promise| {
				let reply = promise.get_reply().unwrap();

				let offer = reply
					.get_value("offer")
					.unwrap()
					.get::<gst_webrtc::WebRTCSessionDescription>()
					.expect("Invalid argument");
				webrtc2.emit("set-local-description", &[&offer, &None::<gst::Promise>]).unwrap();

				println!("Send {:?}", offer.get_sdp().as_text().unwrap());
				// TODO This is not json
				let logger = logger.clone();
				/*executor.spawn(*/ws_addr.do_send(crate::WsMessage::Message(
					MessageP2F::Webrtc(WebrtcMsg::Sdp {
						typ: "offer".to_string(),
						sdp: offer.get_sdp().as_text().unwrap(),
					})));/*.compat().map(move |r| {
						if let Err(e) = r {
							error!(logger, "Failed to send webrtc message"; "error" => ?e);
						}
					})).unwrap();*/
			});

			webrtc.emit("create-offer", &[&None::<gst::Structure>, &promise]).unwrap();

		self.pipeline.set_state(gst::State::Playing)?;
		error!(self.logger, "Finished webrtc setup");
		Ok(())
	}
}

impl WebrtcHandler {
	pub fn new(logger: Logger, executor: ThreadPool, pipeline: gst::Pipeline) -> Result<Self, Error> {
		check_plugins()?;
		let webrtc = gst::ElementFactory::make("webrtcbin", Some("webrtc")).unwrap();
		pipeline.add(&webrtc)?;
		webrtc.connect("on-negotiation-needed", false, move |values| {
			println!("Neeedeeed");
			None
		});

		webrtc.set_property_from_str("stun-server", STUN_SERVER);
		webrtc.set_property_from_str("bundle-policy", "max-bundle");

		//add_video_source(&pipeline, &webrtc)?;
		add_audio_source(&pipeline, &webrtc)?;

		Ok(Self {
			logger,
			executor,
			pipeline,
			webrtc,
		})
	}

	fn handle_ice(&mut self, candidate: &str, sdp_mline_index: u32) {
		self.webrtc
			.emit("add-ice-candidate", &[&sdp_mline_index, &candidate])
			.unwrap();
	}

	fn handle_sdp(&mut self, typ: &str, sdp: &str) {
		if typ != "answer" {
			error!(self.logger, "Sdp type is not \"answer\""; "type" => typ);
			return;
		}

		println!("{}", sdp);
		let ret = gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes()).unwrap();
		// TODO Report: If this is None, we get a SEGFAULT in set-remote-description
		//println!("Media: {:?}", ret.get_media(0));
		let answer =
			gst_webrtc::WebRTCSessionDescription::new(gst_webrtc::WebRTCSDPType::Answer, ret);
		let promise = gst::Promise::new();
		//promise.interrupt();
		self.webrtc.emit("set-remote-description", &[&answer, &promise])
			.unwrap();
	}
}

fn check_plugins() -> Result<(), Error> {
	let needed = [
		"opus",
		"vpx",
		"nice",
		"webrtc",
		"dtls",
		"srtp",
		"rtpmanager",
		"videotestsrc",
		"audiotestsrc",
	];

	let registry = gst::Registry::get();
	let missing = needed
		.iter()
		.filter(|n| registry.find_plugin(n).is_none())
		.map(|n| *n)
		.collect::<Vec<_>>();

	if !missing.is_empty() {
		Err(format_err!("Missing gstreamer elements: {:?}", missing))?
	} else {
		Ok(())
	}
}

fn handle_media_stream(
	pad: &gst::Pad,
	pipe: &gst::Pipeline,
	media_type: MediaType,
) -> Result<(), Error> {
	println!("Trying to handle stream {:?}", media_type);

	let (q, conv, sink) = match media_type {
		MediaType::Audio => {
			let q = gst::ElementFactory::make("queue", None).unwrap();
			let conv = gst::ElementFactory::make("audioconvert", None).unwrap();
			let sink = gst::ElementFactory::make("autoaudiosink", None).unwrap();
			let resample = gst::ElementFactory::make("audioresample", None).unwrap();

			pipe.add_many(&[&q, &conv, &resample, &sink])?;
			gst::Element::link_many(&[&q, &conv, &resample, &sink])?;

			resample.sync_state_with_parent()?;

			(q, conv, sink)
		}
		MediaType::Video => {
			/*let q = gst::ElementFactory::make("queue", None).unwrap();
			let conv = gst::ElementFactory::make("videoconvert", None).unwrap();
			let sink = gst::ElementFactory::make("autovideosink", None).unwrap();

			pipe.add_many(&[&q, &conv, &sink])?;
			gst::Element::link_many(&[&q, &conv, &sink])?;

			(q, conv, sink)*/
			return Err(format_err!("Video is not yet implemented"));
		}
	};
	q.sync_state_with_parent()?;
	conv.sync_state_with_parent()?;
	sink.sync_state_with_parent()?;

	let qpad = q.get_static_pad("sink").unwrap();
	pad.link(&qpad)?;

	Ok(())
}

/*fn add_video_source(pipeline: &gst::Pipeline, webrtcbin: &gst::Element) -> Result<(), Error> {
	let videotestsrc = gst::ElementFactory::make("videotestsrc", None).unwrap();
	let videoconvert = gst::ElementFactory::make("videoconvert", None).unwrap();
	let queue = gst::ElementFactory::make("queue", None).unwrap();
	let vp8enc = gst::ElementFactory::make("vp8enc", None).unwrap();

	videotestsrc.set_property_from_str("pattern", "ball");
	videotestsrc.set_property("is-live", &true).unwrap();
	vp8enc.set_property("deadline", &1i64).unwrap();

	let rtpvp8pay = gst::ElementFactory::make("rtpvp8pay", None).unwrap();
	let queue2 = gst::ElementFactory::make("queue", None).unwrap();

	pipeline.add_many(&[
		&videotestsrc,
		&videoconvert,
		&queue,
		&vp8enc,
		&rtpvp8pay,
		&queue2,
	])?;

	gst::Element::link_many(&[
		&videotestsrc,
		&videoconvert,
		&queue,
		&vp8enc,
		&rtpvp8pay,
		&queue2,
	])?;

	queue2.link_filtered(webrtcbin, &*RTP_CAPS_VP8)?;

	Ok(())
}*/

fn add_audio_source(pipeline: &gst::Pipeline, webrtcbin: &gst::Element) -> Result<(), Error> {
	let audiotestsrc = gst::ElementFactory::make("audiotestsrc", None).unwrap();
	let queue = gst::ElementFactory::make("queue", None).unwrap();
	let audioconvert = gst::ElementFactory::make("audioconvert", None).unwrap();
	let audioresample = gst::ElementFactory::make("audioresample", None).unwrap();
	let queue2 = gst::ElementFactory::make("queue", None).unwrap();
	let opusenc = gst::ElementFactory::make("opusenc", None).unwrap();
	let rtpopuspay = gst::ElementFactory::make("rtpopuspay", None).unwrap();
	let queue3 = gst::ElementFactory::make("queue", None).unwrap();

	audiotestsrc.set_property_from_str("wave", "red-noise");
	audiotestsrc.set_property("is-live", &true).unwrap();

	pipeline.add_many(&[
		&audiotestsrc,
		&queue,
		&audioconvert,
		&audioresample,
		&queue2,
		&opusenc,
		&rtpopuspay,
		&queue3,
	])?;

	gst::Element::link_many(&[
		&audiotestsrc,
		&queue,
		&audioconvert,
		&audioresample,
		&queue2,
		&opusenc,
		&rtpopuspay,
		&queue3,
	])?;

	queue3.link_filtered(webrtcbin, Some(&*RTP_CAPS_OPUS))?;
	println!("Added audio source");

	Ok(())
}
