use actix_web::actix::*;
use failure::Error;
use futures::prelude::*;
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use gstreamer as gst;
use gstreamer_audio as gst_audio;
use gstreamer_app as gst_app;
use gst::prelude::*;
use slog::{debug, error, Logger};

use audio_to_ts::AudioToTs;
use ts_to_audio::TsToAudio;

pub mod audio_to_ts;
pub mod ts_to_audio;
pub mod webrtc;

const VOICE_TIMEOUT_SECS: u64 = 1;

#[derive(Clone)]
pub struct AudioData {
	pub pool: ThreadPool,
	pub pipeline: gst::Pipeline,
	pub a2ts: Addr<AudioToTs>,
	pub ts2a: Addr<TsToAudio>,
}

pub(crate) fn start(logger: Logger, webrtc: Option<Addr<crate::Ws>>)
	-> Result<(AudioData, Option<Addr<webrtc::WebrtcHandler>>), Error> {
	gst::init().expect("gstreamer failed to initialize");

	let pool = futures_threadpool::Builder::new()
		.pool_size(2)
		.name_prefix("audio")
		.create();
	let pipeline = gst::Pipeline::new(Some("ts-pipeline"));

	let rtc = if let Some(addr) = webrtc {
		Some(webrtc::WebrtcHandler::new(
			logger.clone(),
			pool.clone(),
			pipeline.clone(),
			addr,
		)?)
	} else {
		None
	};

	let ts2a = TsToAudio::new(logger.clone(), pipeline.clone(), rtc.as_ref())?;
	let a2ts = AudioToTs::new(logger.clone(), pipeline.clone(), pool.clone(), rtc.as_ref(), None)?;

	let rtc = rtc.map(|r| r.start());

	// Run event handler in background
	pool.spawn(main_loop(&pipeline, logger.clone()).unit_error().compat()).detach();

	Ok((AudioData {
		pool,
		pipeline,
		a2ts: a2ts.start(),
		ts2a: ts2a.start(),
	}, rtc))
}

fn main_loop(
	pipeline: &gst::Pipeline,
	logger: Logger,
) -> impl Future<Output = ()>
{
	// We use an AbortHandle for having a Future that runs forever
	// until we call handle.abort() to quit our event loop
	let (quit_handle, quit_registration) = future::AbortHandle::new_pair();

	// BusStream implements the Stream trait. Stream::for_each is calling a closure for each item
	// and returns a Future that resolves when the stream is done
	let bus = pipeline.get_bus().unwrap();
	let messages = gst::BusStream::new(&bus).for_each(move |msg| {
		use gst::MessageView;

		// Determine whether we want to quit: on EOS or error message
		// we quit, otherwise simply continue.
		match msg.view() {
			MessageView::Eos(..) => {
				debug!(logger, "Got end of playing stream");
				quit_handle.abort();
			}
			MessageView::Error(err) => {
				error!(logger,
					"gstreamer pipeline error";
					"src" => ?err.get_src().map(|s| s.get_path_string().as_str().to_string()),
					"error" => %err.get_error(),
					"debug" => ?err.get_debug()
				);
				quit_handle.abort();
			}
			MessageView::StateChanged(change) => {
				if change.get_current() == gst::State::Null {
					quit_handle.abort();
				}
			}
			_ => {}
		}

		future::ready(())
	});
	future::Abortable::new(messages, quit_registration).map(|_| ())
}
