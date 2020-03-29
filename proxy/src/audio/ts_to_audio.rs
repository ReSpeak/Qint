use std::sync::Arc;
use std::task::Waker;
use std::time::Duration;

use actix::*;
use anyhow::Result;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};

use super::*;

pub struct StartPlayingMsg;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum T2AStatus {
	/// Playing but no data have been added.
	PlayingNothing,
	/// Playing and should pause because there were no data the last time.
	PlayingShouldPause,
	/// Playing and has data.
	Playing,
	/// Needs to be started.
	Paused,
}

#[derive(Clone, Debug)]
pub(crate) struct TsToAudioData {
	/// Connections should set this to `true` after writing data into the
	/// buffer.
	state: T2AStatus,
	pub data: Vec<f32>,
	/// The wakers will be woken when a new buffer should be filled.
	pub wakers: Vec<Waker>,
	/// Changes when the buffer has been used.
	pub gen: usize,
}

pub(crate) struct TsToAudio {
	logger: Logger,
	audio_subsystem: AudioSubsystem,
	device: Option<AudioDevice<SdlCallback>>,
	data: Arc<Mutex<TsToAudioData>>,
}

struct SdlCallback {
	logger: Logger,
	data: Arc<Mutex<TsToAudioData>>,
}

impl Message for StartPlayingMsg {
	type Result = ();
}

impl TsToAudioData {
	/// Returns `true` if the audio actor has to be woken upp with a
	/// `StartPlayingMsg`.
	pub fn add_waker(&mut self, waker: Waker) -> bool {
		self.wakers.push(waker);
		match self.state {
			T2AStatus::PlayingNothing
			| T2AStatus::PlayingShouldPause => {
				self.state = T2AStatus::Playing;
			}
			T2AStatus::Paused => {
				self.state = T2AStatus::Playing;
				return true;
			}
			_ => {}
		}
		false
	}
}

impl Actor for TsToAudio {
	type Context = Context<Self>;

	fn started(&mut self, ctx: &mut Self::Context) {
		self.open_playback();

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			{
				let mut data = t2a.data.lock().unwrap();
				if data.state == T2AStatus::PlayingShouldPause {
					if let Some(device) = &t2a.device {
						if device.status() == AudioStatus::Playing {
							debug!(t2a.logger, "Pausing playback");
							device.pause();
							data.state = T2AStatus::Paused;
						}
					}
				}
			}

			// Restart on errors
			if t2a
				.device
				.as_ref()
				.map(|d| d.status() == AudioStatus::Stopped)
				.unwrap_or(true)
			{
				// Try to reconnect to audio
				t2a.open_playback();
			}
		});
	}
}

impl TsToAudio {
	pub(crate) fn new(
		logger: Logger, audio_subsystem: AudioSubsystem,
	) -> Result<(Self, Arc<Mutex<TsToAudioData>>)>
	{
		let logger = logger.new(o!("pipeline" => "ts-to-audio"));
		let data = Arc::new(Mutex::new(TsToAudioData {
			state: T2AStatus::Paused,
			data: Default::default(),
			wakers: Default::default(),
			gen: 0,
		}));

		Ok((Self {
			logger,
			audio_subsystem,
			device: None,
			data: data.clone(),
		}, data))
	}

	fn open_playback(&mut self) {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(2),
			samples: Some(USUAL_SAMPLE_COUNT as u16),
		};

		let logger = self.logger.clone();
		let data = self.data.clone();
		match self.audio_subsystem.open_playback(None, &desired_spec, |spec| {
			// This spec will always be the desired spec, the sdl wrapper passes
			// zero as `allowed_changes`.
			debug!(logger, "Got playback spec"; "spec" => ?spec, "driver" => self.audio_subsystem.current_audio_driver());
			SdlCallback {
				logger,
				data,
			}
		}) {
			Ok(device) => self.device = Some(device),
			Err(e) => {
				self.device = None;
				error!(self.logger, "Failed to open playback device"; "error" => ?e);
			}
		}
	}
}

impl Handler<StartPlayingMsg> for TsToAudio {
	type Result = ();
	fn handle(&mut self, _: StartPlayingMsg, _: &mut Self::Context) -> Self::Result {
		if let Some(device) = &self.device {
			if device.status() == AudioStatus::Paused {
				debug!(self.logger, "Resuming playback");
				self.data.lock().unwrap().state = T2AStatus::Playing;
				self.device.as_ref().unwrap().resume();
			}
		} else {
			warn!(
				self.logger,
				"Unable to play audio packet, device is not initialized"
			);
		}
	}
}

impl AudioCallback for SdlCallback {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) {
		trace!(self.logger, "Filling audio playback buffer"; "len" => buffer.len());

		let mut data = self.data.lock().unwrap();
		if data.data.len() != buffer.len() {
			warn!(self.logger, "Audio buffer has wrong length";
				"has" => data.data.len(), "need" => buffer.len());
			data.data.resize(buffer.len(), 0.0);
		}
		buffer.copy_from_slice(&data.data);

		// Clear buffer
		for d in &mut data.data {
			*d = 0.0;
		}

		data.gen = data.gen.wrapping_add(1);
		let new_state = match data.state {
			T2AStatus::PlayingNothing
			| T2AStatus::PlayingShouldPause => T2AStatus::PlayingShouldPause,
			T2AStatus::Playing => T2AStatus::PlayingNothing,
			T2AStatus::Paused => T2AStatus::Paused,
		};
		data.state = new_state;
		data.wakers.drain(..).for_each(|w| w.wake());
	}
}
