//! A nice and simple explanation of how you create a WebRTC connection:
//! [https://shanetully.com/2014/09/a-dead-simple-webrtc-example/](https://shanetully.com/2014/09/a-dead-simple-webrtc-example/)
//!
//! Examples for WebRTC with gstreamer in Rust can be found here:
//! [https://github.com/centricular/gstwebrtc-demos](https://github.com/centricular/gstwebrtc-demos)

use actix_web::actix::*;
use failure::{format_err, Error};
use futures01::{future, Future};
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use gstreamer as gst;
use gstreamer_webrtc as gst_webrtc;
use gstreamer_sdp as gst_sdp;
use gst::prelude::*;
use lazy_static::lazy_static;
use qint_shared::*;
use slog::{debug, error, Logger};

const STUN_SERVER: &str = "stun://stun.services.mozilla.com";

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

pub struct WebrtcHandler {
	logger: Logger,
	executor: ThreadPool,
	ws_addr: Addr<crate::Ws>,
	pub webrtc: gst::Element,
}

/// The 'signalling' channel is the websocket which is also used for the rest of
/// the communication.
#[derive(Clone, Debug)]
pub struct SignallingMsg(pub WebrtcMsg);

impl Actor for WebrtcHandler {
	type Context = Context<Self>;
}

impl Message for SignallingMsg { type Result = (); }

impl Handler<SignallingMsg> for WebrtcHandler {
	type Result = ();
	fn handle(&mut self, msg: SignallingMsg, _: &mut Self::Context) -> Self::Result {
		match msg.0 {
			WebrtcMsg::Ice { candidate, sdp_mline_index } =>
				self.handle_ice(&candidate, sdp_mline_index),
			WebrtcMsg::Sdp { typ, sdp } => self.handle_sdp(&typ, &sdp),
		}
	}
}

impl WebrtcHandler {
	pub(crate) fn new(logger: Logger, executor: ThreadPool, pipeline: gst::Pipeline, ws_addr: Addr<crate::Ws>) -> Result<Self, Error> {
		check_plugins()?;
		let webrtc = gst::ElementFactory::make("webrtcbin", Some("webrtc")).unwrap();
		webrtc.set_property_from_str("stun-server", STUN_SERVER);
		webrtc.set_property_from_str("bundle-policy", "max-bundle");

		pipeline.add(&webrtc)?;

		let webrtc2 = webrtc.clone();
		let executor2 = executor.clone();
		let logger2 = logger.clone();
		let ws_addr2 = ws_addr.clone();
		webrtc.connect("on-negotiation-needed", false, move |_| {
			debug!(logger2, "Needs negotiation, sending offer");
			let webrtc = webrtc2.clone();
			let executor = executor2.clone();
			let ws_addr = ws_addr2.clone();
			let logger = logger2.clone();
			let promise = gst::Promise::new_with_change_func(move |promise| {
				let reply = promise.get_reply().unwrap();

				let offer = reply
					.get_value("offer")
					.unwrap()
					.get::<gst_webrtc::WebRTCSessionDescription>()
					.expect("Invalid argument");
				webrtc.emit("set-local-description", &[&offer, &None::<gst::Promise>]).unwrap();

				debug!(logger, "Send sdp"; "sdp" => offer.get_sdp().as_text().unwrap());
				let logger = logger.clone();
				let typ = match offer.get_type() {
					gst_webrtc::WebRTCSDPType::Offer => "offer",
					gst_webrtc::WebRTCSDPType::Pranswer => "pranswer",
					gst_webrtc::WebRTCSDPType::Answer => "answer",
					gst_webrtc::WebRTCSDPType::Rollback => "rollback",
					t => {
						error!(logger, "Unknown webrtc sdp type"; "type" => ?t);
						return;
					}
				};
				executor.spawn(future::lazy(move || {
					ws_addr.send(crate::WsMessage::Message(
						MessageP2F::Webrtc(WebrtcMsg::Sdp {
							typ: typ.into(),
							sdp: offer.get_sdp().as_text().unwrap(),
						}))).map_err(move |e| {
							error!(logger, "Failed to send webrtc message"; "error" => ?e);
						})
				})).detach();
			});

			webrtc2.emit("create-offer", &[&None::<gst::Structure>, &promise]).unwrap();
			None
		})?;

		let executor2 = executor.clone();
		let logger2 = logger.clone();
		let ws_addr2 = ws_addr.clone();
		webrtc.connect("on-ice-candidate", false, move |values| {
			let mlineindex = values[1].get::<u32>().expect("Invalid argument");
			let candidate = values[2].get::<String>().expect("Invalid argument");

			let logger = logger2.clone();
			debug!(logger, "Send ice"; "candidate" => &candidate, "mlineindex" => mlineindex);
			// Ignore failure when the websocket connection is gone
			let ws_addr = ws_addr2.clone();
			executor2.spawn(future::lazy(move || {
				ws_addr.send(crate::WsMessage::Message(
					MessageP2F::Webrtc(WebrtcMsg::Ice {
						candidate,
						sdp_mline_index: mlineindex,
					}))).map_err(move |e| {
						error!(logger, "Failed to send webrtc message"; "error" => ?e);
					})
			})).detach();
			None
		})?;

		Ok(Self {
			logger,
			executor,
			ws_addr,
			webrtc,
		})
	}

	fn handle_ice(&mut self, candidate: &str, sdp_mline_index: u32) {
		debug!(self.logger, "Received ice"; "candidate" => candidate,
			"sdpMLineIndex" => sdp_mline_index);
		self.webrtc
			.emit("add-ice-candidate", &[&sdp_mline_index, &candidate])
			.unwrap();
	}

	fn handle_sdp(&mut self, typ: &str, sdp: &str) {
		debug!(self.logger, "Received sdp"; "type" => typ, "sdp" => sdp);

		let typ = if typ == "answer" {
			gst_webrtc::WebRTCSDPType::Answer
		} else if typ == "offer" {
			gst_webrtc::WebRTCSDPType::Offer
		} else {
			error!(self.logger, "Unknown sdp type"; "type" => typ);
			return;
		};

		let ret = gst_sdp::SDPMessage::parse_buffer(sdp.as_bytes()).unwrap();
		if ret.get_media(0).is_none() {
			// TODO Report bug: If this is None, we get a SEGFAULT in
			// set-remote-description because of a null pointer.
			error!(self.logger, "Media of sdp is None");
			return;
		}
		let sdp = gst_webrtc::WebRTCSessionDescription::new(typ, ret);
		let promise;
		if typ == gst_webrtc::WebRTCSDPType::Offer {
			// Send answer
			let logger = self.logger.clone();
			let webrtc = self.webrtc.clone();
			let executor = self.executor.clone();
			let ws_addr = self.ws_addr.clone();
			promise = gst::Promise::new_with_change_func(move |_| {
				let webrtc2 = webrtc.clone();
				let promise = gst::Promise::new_with_change_func(move |promise| {
					let reply = promise.get_reply().unwrap();

					let offer = reply
						.get_value("answer")
						.unwrap()
						.get::<gst_webrtc::WebRTCSessionDescription>()
						.expect("Invalid argument");
					webrtc2.emit("set-local-description", &[&offer, &None::<gst::Promise>]).unwrap();

					debug!(logger, "Send sdp"; "sdp" => offer.get_sdp().as_text().unwrap());
					let logger = logger.clone();
					let typ = match offer.get_type() {
						gst_webrtc::WebRTCSDPType::Offer => "offer",
						gst_webrtc::WebRTCSDPType::Pranswer => "pranswer",
						gst_webrtc::WebRTCSDPType::Answer => "answer",
						gst_webrtc::WebRTCSDPType::Rollback => "rollback",
						t => {
							error!(logger, "Unknown webrtc sdp type"; "type" => ?t);
							return;
						}
					};
					executor.spawn(future::lazy(move || {
						ws_addr.send(crate::WsMessage::Message(
							MessageP2F::Webrtc(WebrtcMsg::Sdp {
								typ: typ.into(),
								sdp: offer.get_sdp().as_text().unwrap(),
							}))).map_err(move |e| {
								error!(logger, "Failed to send webrtc message"; "error" => ?e);
							})
					})).detach();
				});

				webrtc.emit("create-answer", &[&None::<gst::Structure>, &promise]).unwrap();
			});
		} else {
			promise = gst::Promise::new();
		}

		self.webrtc.emit("set-remote-description", &[&sdp, &promise])
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
