use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

use actix::*;
use anyhow::Result;
use futures::FutureExt;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio::task;
use tracing::error;

use crate::connection::QintConnection;
use crate::{ConnectionId, Settings};

pub mod audio_to_ts;
pub mod ts_to_audio;

#[cfg(feature = "oboe")]
pub mod oboe;
#[cfg(feature = "sdl2")]
pub mod sdl;

#[cfg(feature = "oboe")]
pub type AudioToTs = audio_to_ts::AudioToTs<oboe::AudioToTsOboe>;
#[cfg(feature = "sdl2")]
pub type AudioToTs = audio_to_ts::AudioToTs<sdl::AudioToTsSdl>;
#[cfg(feature = "oboe")]
pub type TsToAudio = ts_to_audio::TsToAudio<oboe::TsToAudioOboe>;
#[cfg(feature = "sdl2")]
pub type TsToAudio = ts_to_audio::TsToAudio<sdl::TsToAudioSdl>;

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
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>, settings: &Settings,
) -> Result<AudioData> {
	let global_volume = settings.get_global_volume().unwrap_or(1.0);
	let (capture, playback) = settings.get_preferred_audio_device();

	#[cfg(feature = "sdl2")]
	let sdl_context = sdl2::init().unwrap();

	#[cfg(feature = "sdl2")]
	let audio_subsystem = sdl_context.audio().unwrap();
	#[cfg(feature = "sdl2")]
	{
		// SDL automatically disables the screensaver, enable it again
		if let Ok(video_subsystem) = sdl_context.video() {
			video_subsystem.enable_screen_saver();
		}
	}

	#[cfg(feature = "oboe")]
	{
		/*if let Err(error) = oboe::DefaultStreamValues::init() {
			// TODO log
		}*/
	}

	let mut runtime = Runtime::new().unwrap();

	// Create thread local runtime for non-send tasks
	// A channel size of 1 leads to audio drops when cpu is fully used
	let (spawn_send, mut spawn_recv) = mpsc::channel(5);
	#[cfg(feature = "sdl2")]
	let ts2a = TsToAudio::new(
		sdl::TsToAudioSdl::new(audio_subsystem.clone())?,
		playback,
		connections,
		global_volume,
	)
	.start();
	#[cfg(feature = "oboe")]
	let ts2a = TsToAudio::new(oboe::TsToAudioOboe::new(), playback, connections, global_volume).start();
	#[cfg(feature = "sdl2")]
	let a2ts = AudioToTs::new(sdl::AudioToTsSdl::new(audio_subsystem), capture, spawn_send).start();
	#[cfg(feature = "oboe")]
	let a2ts = AudioToTs::new(oboe::AudioToTsOboe::new(), capture, spawn_send).start();

	let a2ts2 = a2ts.clone();
	thread::spawn(move || {
		let local = tokio::task::LocalSet::new();

		// Run the local task set.
		local.block_on(&mut runtime, async move {
			while let Some(msg) = spawn_recv.recv().await {
				task::spawn_local(a2ts2.send(msg).map(move |r| match r {
					Ok(()) => {}
					Err(error) => error!(%error, "Failed to send audio data to Audio2TS pipeline"),
				}));
			}
		});
	});

	Ok(AudioData { a2ts, ts2a })
}
