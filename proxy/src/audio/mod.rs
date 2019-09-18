use actix_web::actix::*;
use failure::Error;
use futures::prelude::*;
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use slog::{debug, error, Logger};

use audio_to_ts::AudioToTsSdl;
use ts_to_audio::TsToAudioSdl;

pub mod audio_to_ts;
pub mod ts_to_audio;

const VOICE_TIMEOUT_SECS: u64 = 1;

#[derive(Clone)]
pub struct AudioData {
	pub pool: ThreadPool,
	pub a2ts: Addr<AudioToTsSdl>,
	pub ts2a: Addr<TsToAudioSdl>,
}

pub(crate) fn start(logger: Logger, webrtc: Option<Addr<crate::Ws>>)
	-> Result<AudioData, Error> {

	let sdl_context = sdl2::init().unwrap();
	let audio_subsystem = sdl_context.audio().unwrap();

	let pool = futures_threadpool::Builder::new()
		.pool_size(2)
		.name_prefix("audio")
		.create();

	let ts2a = TsToAudioSdl::new(logger.clone(), &audio_subsystem)?;
	let a2ts = AudioToTsSdl::new(logger.clone(), pool.clone(), None)?;

	Ok(AudioData {
		pool,
		a2ts: a2ts.start(),
		ts2a: ts2a.start(),
	})
}