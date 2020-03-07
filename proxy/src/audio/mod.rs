use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use actix::*;
use failure::Error;
use futures_threadpool::ThreadPool;
use futures::FutureExt;
use slog::{error, Logger};
use tokio::stream::StreamExt;
use tokio::sync::mpsc;
use tokio::task;

use crate::websocket::TsConnection;
use crate::ConnectionId;

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
pub(crate) struct AudioData {
	pub pool: ThreadPool,
	pub a2ts: Addr<AudioToTs>,
	pub ts2a: Addr<TsToAudio>,
}

#[derive(Clone, Debug)]
pub(crate) enum SendAudioEvent {
	TalkersChanged(ConnectionId),
	PlayPacket(Vec<f32>),
}

pub(crate) fn start(
	logger: Logger,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<TsConnection>>>>,
) -> Result<AudioData, Error>
{
	let sdl_context = sdl2::init().unwrap();

	let audio_subsystem = sdl_context.audio().unwrap();
	// SDL automatically disables the screensaver, enable it again
	if let Ok(video_subsystem) = sdl_context.video() {
		video_subsystem.enable_screen_saver();
	}

	let pool = futures_threadpool::Builder::new()
		.pool_size(2)
		.name_prefix("audio")
		.create();

	// Create thread local runtime for non-send tasks
	let (spawn_send, mut spawn_recv) = mpsc::unbounded_channel();
	let ts2a = TsToAudio::new(logger.clone(), audio_subsystem.clone(), connections, spawn_send.clone())?.start();
	let a2ts = AudioToTs::new(logger.clone(), audio_subsystem, pool.clone(), spawn_send)?.start();

	let ts2a2 = ts2a.clone();
	let a2ts2 = a2ts.clone();
	thread::spawn(move || {
		let mut rt = tokio::runtime::Runtime::new().unwrap();
		let local = tokio::task::LocalSet::new();

		// Run the local task set.
		local.block_on(&mut rt, async move {
			while let Some(e) = spawn_recv.next().await {
				let logger = logger.clone();
				match e {
					SendAudioEvent::TalkersChanged(con) => task::spawn_local(
						ts2a2.send(ts_to_audio::TalkersChangedMsg(con))
							.map(move |r| {match r {
								Ok(()) => {}
								Err(e) => {
									error!(logger, "Failed to notify TS2Audio pipeline about talker change"; "error" => ?e)
								}
							}
							}),
						),
					SendAudioEvent::PlayPacket(buffer) => task::spawn_local(
						a2ts2.send(audio_to_ts::PlayPacketMsg(buffer))
							.map(move |r| {match r {
								Ok(()) => {}
								Err(e) => {
									error!(logger, "Failed to send audio data to Audio2TS pipeline"; "error" => ?e)
								}
							}})
					),
				};
			}
		});
	});

	Ok(AudioData { pool, a2ts, ts2a })
}
