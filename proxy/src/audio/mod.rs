use actix_web::actix::*;
use failure::Error;
use futures_threadpool::ThreadPool;
use slog::Logger;

use audio_to_ts::AudioToTs;
use ts_to_audio::TsToAudio;

pub mod audio_to_ts;
pub mod ts_to_audio;

const VOICE_TIMEOUT_SECS: u64 = 1;

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
