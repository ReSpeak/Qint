use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use actix::*;
use anyhow::Result;
use futures::FutureExt;
use slog::{error, Logger};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task;

use crate::connection::QintConnection;
use crate::{ConnectionId, Settings};
use audio_to_ts::AudioToTs;
use ts_to_audio::TsToAudio;

pub mod audio_to_ts;
pub mod ts_to_audio;

/// Sample rate is 48 kHz.
const SAMPLE_RATE: usize = 48000;

/// The usual frame size.
///
/// Use 48 kHz, 20 ms frames (50 per second) and mono data (1 channel).
/// This means 1920 samples and 7.5 kiB.
const USUAL_FRAME_SIZE: usize = SAMPLE_RATE / 50;

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
	pub a2ts: Addr<AudioToTs>,
	pub ts2a: Addr<TsToAudio>,
}

pub struct ResetMsg;
pub struct GetAudioDevices();
pub struct SetAudioDevice(pub Option<String>);

impl Message for ResetMsg {
	type Result = ();
}
impl Message for GetAudioDevices {
	type Result = Vec<String>;
}
impl Message for SetAudioDevice {
	type Result = ();
}

pub(crate) fn start(
	logger: Logger, connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>, settings: &Settings,
) -> Result<AudioData> {
	let global_volume = settings.get_global_volume().unwrap_or(1.0);
	let (capture, playback) = settings.get_preferred_audio_device();

	let sdl_context = sdl2::init().unwrap();

	let audio_subsystem = sdl_context.audio().unwrap();
	// SDL automatically disables the screensaver, enable it again
	if let Ok(video_subsystem) = sdl_context.video() {
		video_subsystem.enable_screen_saver();
	}

	let mut runtime = Runtime::new().unwrap();

	// Create thread local runtime for non-send tasks
	// A channel size of 1 leads to audio drops when cpu is fully used
	let (spawn_send, mut spawn_recv) = mpsc::channel(5);
	let ts2a = TsToAudio::new(
		logger.clone(),
		audio_subsystem.clone(),
		playback,
		connections,
		global_volume,
	)?
	.start();
	let a2ts = AudioToTs::new(logger.clone(), audio_subsystem, capture, spawn_send)?.start();

	let a2ts2 = a2ts.clone();
	thread::spawn(move || {
		let local = tokio::task::LocalSet::new();

		// Run the local task set.
		local.block_on(&mut runtime, async move {
			while let Some(msg) = spawn_recv.recv().await {
				let logger = logger.clone();
				task::spawn_local(a2ts2.send(msg).map(move |r| match r {
					Ok(()) => {}
					Err(e) => error!(logger, "Failed to send audio data to Audio2TS \
							pipeline"; "error" => %e),
				}));
			}
		});
	});

	Ok(AudioData { a2ts, ts2a })
}
