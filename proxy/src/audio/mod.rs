use actix_web::actix::*;
use failure::Error;
use futures_threadpool::ThreadPool;
use slog::Logger;

use audio_to_ts::AudioToTs;
use ts_to_audio::TsToAudio;

pub mod audio_to_ts;
pub mod ts_to_audio;

/// The maximum supported size of a decoded audio packet.
///
/// Use 48 kHz, maximum of 120 ms frames (3 times 40 ms frames of which there
/// are 25 per second) and stereo data (2 channels).
/// This is a maximum of 11520 samples and 45 kiB.
const MAX_FRAME_SIZE: usize = 48000 / 25 * 3 * 2;

/// The usual frame size.
///
/// Use 48 kHz, 20 ms frames (50 per second) and mono data (1 channel).
/// This means 1920 samples and 7.5 kiB.
const USUAL_FRAME_SIZE: usize = 48000 / 50;

/// The number of samples to use for SDL output.
///
/// This is the [`USUAL_FRAME_SIZE`] divided by the number of channels.
///
/// [`USUAL_FRAME_SIZE`]: constant.USUAL_FRAME_SIZE.html
const USUAL_SAMPLE_COUNT: usize = USUAL_FRAME_SIZE;

/// The maximum size of an opus frame is 1275 as from RFC6716.
const MAX_OPUS_FRAME_SIZE: usize = 1275;

#[derive(Clone)]
pub struct AudioData {
	pub pool: ThreadPool,
	pub a2ts: Addr<AudioToTs>,
	pub ts2a: Addr<TsToAudio>,
}

pub(crate) fn start(logger: Logger)
	-> Result<AudioData, Error> {

	let sdl_context = sdl2::init().unwrap();
	let audio_subsystem = sdl_context.audio().unwrap();

	let pool = futures_threadpool::Builder::new()
		.pool_size(2)
		.name_prefix("audio")
		.create();

	let ts2a = TsToAudio::new(logger.clone(), &audio_subsystem)?;
	let a2ts = AudioToTs::new(logger.clone(), &audio_subsystem, pool.clone())?;

	Ok(AudioData {
		pool,
		a2ts: a2ts.start(),
		ts2a: ts2a.start(),
	})
}
