use actix_web::actix::*;
use failure::Error;
use futures::prelude::*;
use futures::executor::{ThreadPoolBuilder};
use futures::task::SpawnExt;
use gstreamer as gst;
use gstreamer_audio as gst_audio;
use gstreamer_app as gst_app;
use gst::prelude::*;
use slog::{debug, error, Logger};

use audio_to_ts::AudioToTs;
use ts_to_audio::TsToAudio;

pub mod ts_to_audio;
pub mod audio_to_ts;

const VOICE_TIMEOUT_SECS: u64 = 1;

pub fn start(logger: Logger) -> Result<(Addr<AudioToTs>, Addr<TsToAudio>), Error> {
	gst::init().expect("gstreamer failed to initialize");

	let pool = ThreadPoolBuilder::new()
		.pool_size(2)
		.name_prefix("audio")
		.create()?;

	let a2ts = AudioToTs::new(logger.clone(), pool.clone(), None)?;
	let ts2a = TsToAudio::new(logger, pool.clone())?;
	Ok((a2ts.start(), ts2a.start()))
}

fn main_loop(
	pipeline: &gst::Pipeline,
	logger: Logger,
) -> impl Future<Output = ()>
{
	// TODO Not automatically
	/*pipeline
		.set_state(gst::State::Playing)
		.expect("Unable to set the pipeline to the `Playing` state");
	debug!(logger, "Pipeline is playing");*/

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
