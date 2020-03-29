use std::sync::Arc;
use std::task::Waker;
use std::time::Duration;

use actix::*;
use anyhow::Result;
use sdl2::audio::{AudioQueue, AudioSpecDesired, AudioStatus};
use sdl2::AudioSubsystem;
use slog::{debug, error, o, trace, warn, Logger};

use super::*;

/// Buffer for 10 ms at 48 kHz (stereo).
const BUFFER_SIZE: usize = 48_000 / 100 * 2;

pub struct StartPlayingMsg;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum T2AStatus {
	/// Playing but no data have been added.
	PlayingNothing,
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
	device: Option<AudioQueue<f32>>,
	data: Arc<Mutex<TsToAudioData>>,
	queue_buffer_timer: Option<SpawnHandle>,
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
			| T2AStatus::PlayingNothing => {
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
			data: vec![0.0; BUFFER_SIZE],
			wakers: Default::default(),
			gen: 0,
		}));

		Ok((Self {
			logger,
			audio_subsystem,
			device: None,
			data: data.clone(),
			queue_buffer_timer: None,
		}, data))
	}

	fn open_playback(&mut self) {
		let desired_spec = AudioSpecDesired {
			freq: Some(48000),
			channels: Some(2),
			samples: Some(USUAL_SAMPLE_COUNT as u16),
		};

		let logger = self.logger.clone();
		// This spec will always be the desired spec, the sdl wrapper passes
		// zero as `allowed_changes`.
		match self.audio_subsystem.open_queue(None, &desired_spec) {
			Ok(queue) => {
				debug!(logger, "Got playback spec"; "spec" => ?queue.spec(),
					"driver" => self.audio_subsystem.current_audio_driver());
				self.device = Some(queue)
			}
			Err(e) => {
				self.device = None;
				error!(self.logger, "Failed to open playback device"; "error" => e);
			}
		}
	}
}

impl Handler<StartPlayingMsg> for TsToAudio {
	type Result = ();
	fn handle(&mut self, _: StartPlayingMsg, ctx: &mut Self::Context) -> Self::Result {
		if let Some(device) = &self.device {
			if device.status() == AudioStatus::Paused {
				debug!(self.logger, "Resuming playback");
				self.data.lock().unwrap().state = T2AStatus::Playing;
				self.device.as_ref().unwrap().resume();

				self.queue_buffer_timer = Some(ctx.run_interval(Duration::from_millis(10), |t2a, ctx| {
					trace!(t2a.logger, "Filling audio playback buffer");

					let mut data = t2a.data.lock().unwrap();
					if let Some(queue) = &t2a.device {
						queue.queue(&data.data);
					} else {
						debug!(t2a.logger, "Stopping playback because device is lost");
						ctx.cancel_future(t2a.queue_buffer_timer.take().unwrap());
						data.state = T2AStatus::Paused;
						return;
					}

					// Clear buffer
					for d in &mut data.data {
						*d = 0.0;
					}

					data.gen = data.gen.wrapping_add(1);
					let new_state = match data.state {
						T2AStatus::Paused
						| T2AStatus::PlayingNothing => {
							debug!(t2a.logger, "Pausing playback");
							t2a.device.as_ref().unwrap().pause();
							ctx.cancel_future(t2a.queue_buffer_timer.take().unwrap());
							T2AStatus::Paused
						}
						T2AStatus::Playing => {
							T2AStatus::PlayingNothing
						}
					};
					data.state = new_state;
					data.wakers.drain(..).for_each(|w| w.wake());
				}));
			}
		} else {
			warn!(
				self.logger,
				"Unable to play audio packet, device is not initialized"
			);
		}
	}
}
