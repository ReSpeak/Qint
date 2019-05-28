use std::collections::HashMap;
use std::fmt;
use std::io::Write;
use std::sync::Arc;
use std::time::{Duration, Instant};

use actix_web::actix::*;
use actix_web::actix::fut::wrap_future;
use async_timer::oneshot::{Oneshot, Timer};
use failure::{format_err, Error};
use futures::prelude::*;
use futures::executor::ThreadPool;
use futures01::Future as _;
use parking_lot::Mutex;
use slog::{o, Logger};
use tsclientlib::ClientId;
use tsproto_packets::packets::{AudioData, CodecType, InAudio};

use crate::ConnectionId;
use super::*;

pub struct TsToAudio {
	logger: Logger,
	pipeline: gst::Pipeline,
	mixer: gst::Element,
	queue: gst::Element,
	sink: gst::Element,

	voices: HashMap<Id, Voice>,
}

pub struct PlayMsg(pub ConnectionId, pub InAudio);

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Id {
	con: ConnectionId,
	client: ClientId,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct Voice {
	source: gst_app::AppSrc,
	decode: gst::Element,
}

impl Actor for TsToAudio {
	type Context = Context<Self>;
}

impl Message for PlayMsg { type Result = Result<(), Error>; }

impl fmt::Display for Id {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}-{}", self.con.0, self.client.0)
	}
}

impl Handler<PlayMsg> for TsToAudio {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: PlayMsg, ctx: &mut Self::Context) -> Self::Result {
		// TODO Whisper packets
		if let AudioData::S2C { id: _, from, codec, data } = msg.1.data() {
			if *codec != CodecType::OpusVoice && *codec != CodecType::OpusMusic {
				return Err(format_err!("Got unsupported audio codec, only opus is supported"));
			}

			let id = Id { con: msg.0, client: ClientId(*from) };
			if !self.voices.contains_key(&id) {
				self.add_voice(id.clone(), ctx)?;
			}

			let mut buffer = gst::Buffer::with_size(data.len()).unwrap();
			{
				let buffer = buffer.get_mut().unwrap();
				let mut bdata = buffer.map_writable().unwrap();
				let mut bdata = &mut *bdata;
				bdata.write_all(data).unwrap();
			}
			//let clock = self.appsrc.get_clock().unwrap();
			//println!("Push buffer {} at {}", id, clock.get_time() - self.appsrc.get_base_time());

			let voice = &self.voices[&id];
			voice.source.push_buffer(buffer)?;
		}
		Ok(())
	}
}

impl TsToAudio {
	/// We need an explicit executor because we want to spawn new tasks in
	/// callbacks from gstreamer threads.
	pub fn new(logger: Logger, mut executor: ThreadPool) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "ts-to-audio"));
		let pipeline = gst::Pipeline::new(Some("ts-to-audio-pipeline"));

		let mixer = gst::ElementFactory::make("audiomixer", Some("mixer"))
			.ok_or_else(|| format_err!("Missing audiomixer"))?;
		let queue = gst::ElementFactory::make("queue", Some("queue"))
			.ok_or_else(|| format_err!("Missing queue"))?;

		// The latency with autoaudiosink is high
		// Linux: Try pulsesink, alsasink
		// Windows: Try directsoundsink
		// Else use autoaudiosink
		let mut autosink = None;
		#[cfg(target_os = "linux")]
		{
			if autosink.is_none() {
				if let Some(sink) =
					gst::ElementFactory::make("pulsesink", Some("autosink"))
				{
					autosink = Some(sink);
				}
			}
		}
		#[cfg(target_os = "linux")]
		{
			if autosink.is_none() {
				if let Some(sink) =
					gst::ElementFactory::make("alsasink", Some("autosink"))
				{
					autosink = Some(sink);
				}
			}
		}

		#[cfg(target_os = "windows")]
		{
			if autosink.is_none() {
				if let Some(sink) =
					gst::ElementFactory::make("directsoundsink", Some("autosink"))
				{
					autosink = Some(sink);
				}
			}
		}

		let autosink = if let Some(sink) = autosink {
			sink
		} else {
			gst::ElementFactory::make("pulsesink", Some("autosink"))
				.ok_or_else(|| format_err!("Missing autoaudiosink"))?
		};
		if autosink.has_property("buffer-time", None).is_ok() {
			autosink
				.set_property("buffer-time", &20_000i64)?;
		}
		if autosink.has_property("blocksize", None).is_ok() {
			autosink.set_property("blocksize", &960u32)?;
		}

		// The audiotestsrc just has to exist and not be eos.
		// Without the audiotestsrc, the audiomixer would send eos after the
		// last pad is removed and the pipeline would finish.
		let fakesrc = gst::ElementFactory::make("audiotestsrc", Some("fake"))
			.ok_or_else(|| format_err!("Missing audiotestsrc"))?;
		fakesrc.set_property("do-timestamp", &true)?;
		fakesrc.set_property("is-live", &true)?;
		fakesrc.set_property("samplesperbuffer", &960i32)?; // 20ms at 48 000 kHz
		fakesrc.set_property_from_str("wave", "Silence");

		pipeline.add_many(&[&fakesrc, &mixer, &queue, &autosink])?;
		gst::Element::link_many(&[&mixer, &queue, &autosink])?;
		// Additionally, we use the audiotestsrc to force the output to stereo and 48000 kHz
		fakesrc.link_filtered(
			&mixer,
			Some(&gst::Caps::new_simple(
				"audio/x-raw",
				&[("rate", &48000i32), ("channels", &2i32)],
			)),
		)?;

		// Set playing only when someone sends audio
		pipeline.set_state(gst::State::Playing)?;
		mixer.set_state(gst::State::Paused)?;
		queue.set_state(gst::State::Paused)?;
		autosink.set_state(gst::State::Paused)?;
		fakesrc.set_state(gst::State::Paused)?;

		// Run event handler in background
		executor.spawn(main_loop(&pipeline, logger.clone())).unwrap();

		Ok(Self {
			logger,
			pipeline,
			mixer,
			queue,
			sink: autosink,

			voices: Default::default(),
		})
	}

	fn add_voice(&mut self, id: Id, ctx: &mut Context<Self>) -> Result<(), Error> {
		debug!(self.logger, "Create voice"; "id" => %id);
		let appsrc = gst::ElementFactory::make("appsrc",
				Some(&format!("appsrc-{}", id)))
			.ok_or_else(|| format_err!("Missing appsrc"))?;

		let src = appsrc.clone().dynamic_cast::<gst_app::AppSrc>().unwrap();
		src.set_caps(Some(&gst::Caps::new_simple("audio/x-opus",
			&[("channel-mapping-family", &0i32)])));
		src.set_property_format(gst::Format::Time);
		src.set_property_min_latency((gst::SECOND_VAL / 50) as i64); // 20 ms in ns
		src.set_property_min_latency(0); // in ns
		// Important to reduce the playback latency
		src.set_property("do-timestamp", &true)?;
		// Set as live source, which means it does not produce data when paused
		src.set_property("is-live", &true)?;

		let decode = gst::ElementFactory::make(
			"opusdec",
			Some(&format!("decoder-{}", id)),
		).expect("Missing opusdec");

		let last_sent = Arc::new(Mutex::new(Instant::now()));
		let last = last_sent.clone();
		let src_pad = decode.get_static_pad("src").unwrap();
		src_pad.add_probe(
			gst::PadProbeType::DATA_DOWNSTREAM,
			move |_pad, _info| {
				let mut last_sent = last.lock();
				*last_sent = Instant::now();
				gst::PadProbeReturn::Ok
			},
		);

		// Check every second if the stream timed out
		let id2 = id.clone();
		let mixer = self.mixer.clone();
		let queue = self.queue.clone();
		let sink = self.sink.clone();
		let pipeline = self.pipeline.clone();
		let appsrc2 = appsrc.clone();
		let decode2 = decode.clone();
		let logger = self.logger.clone();
		ctx.spawn(wrap_future(voice_timeout(last_sent)).map(move |_, ts2a: &mut TsToAudio, _ctx| {
			let mut it = mixer.iterate_sink_pads();
			let last_pad = it.next().is_err() || it.next().is_err() || it.next().is_err();
			if last_pad {
				// Pause pipeline
				mixer.set_state(gst::State::Paused).unwrap();
				queue.set_state(gst::State::Paused).unwrap();
				sink.set_state(gst::State::Paused).unwrap();
			}

			// Unlink and remove decoder
			debug!(logger, "Remove voice"; "id" => %id2);

			let mixer_pad = src_pad.get_peer();

			gst::Element::unlink_many(&[&appsrc2, &decode2, &mixer]);
			pipeline.remove_many(&[&appsrc2, &decode2]).unwrap();

			// Remove pad from mixer
			if let Some(pad) = mixer_pad {
				if let Err(e) = mixer.remove_pad(&pad) {
					error!(logger, "Cannot remove mixer pad"; "error" => ?e);
				}
			} else {
				error!(logger, "Cannot find mixer pad");
			}

			// Cleanup
			appsrc2.set_state(gst::State::Null).unwrap();
			decode2.set_state(gst::State::Null).unwrap();
			if ts2a.voices.remove(&id2).is_none() {
				error!(logger, "Cannot find voice"; "id" => %id2);
			}
		}));

		let mut it = self.mixer.iterate_sink_pads();
		let first_pad = it.next().is_err() || it.next().is_err();
		self.pipeline.add_many(&[&appsrc, &decode])?;
		gst::Element::link_many(&[&appsrc, &decode, &self.mixer])?;
		/*let sink_pad = mixer
			.get_request_pad("sink_%u")
			.expect("Next element has no sink pad");
		if let Err(error) = src_pad.link(&sink_pad) {
			error!(logger, "Cannot link pads"; "error" => ?error);
			gst_element_error!(
				dbin,
				gst::ResourceError::Failed,
				("Failed to link decoder")
			);
		}*/

		if first_pad {
			// Start pipeline
			self.sink.set_state(gst::State::Playing).unwrap();
			self.queue.set_state(gst::State::Playing).unwrap();
			self.mixer.set_state(gst::State::Playing).unwrap();
		}
		decode.set_state(gst::State::Playing)?;
		appsrc.set_state(gst::State::Playing)?;

		self.voices.insert(id, Voice {
			source: src,
			decode,
		});

		Ok(())
	}
}

impl Drop for TsToAudio {
	fn drop(&mut self) {
		// Cleanup gstreamer
		// TODO Sometimes segfaults here when gstreamer posts onto bus
		// Is the bus still running or so?
		self.pipeline.set_state(gst::State::Null).expect(
			"Failed to shutdown gstreamer pipeline");
	}
}

fn voice_timeout(last_sent: Arc<Mutex<Instant>>) -> Box<futures01::Future<Item=(), Error=()>> {
	let timeout = Timer::new(Duration::from_secs(VOICE_TIMEOUT_SECS)).unit_error().compat();
	Box::new(timeout.and_then(move |_| -> Box::<futures01::Future<Item=_, Error=_>> {
		let last = *last_sent.lock();
		if Instant::now().duration_since(last).as_secs() >= VOICE_TIMEOUT_SECS {
			Box::new(futures01::future::ok(()))
		} else {
			voice_timeout(last_sent)
		}
	}))
}
