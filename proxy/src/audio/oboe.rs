use std::time::Duration;

use actix::{AsyncContext, Context};
use oboe::{
	AudioInputCallback, AudioInputStreamSafe, AudioOutputCallback, AudioOutputStreamSafe,
	AudioStream, AudioStreamAsync, AudioStreamBuilder, AudioStreamSafe, DataCallbackResult, Input,
	Mono, Output, PerformanceMode, SharingMode, Stereo, StreamState,
};
use tracing::{debug, error, warn};

use super::audio_to_ts::*;
use super::ts_to_audio::*;

pub struct TsToAudioOboe {
	stream: Option<AudioStreamAsync<Output, TsToAudioCallbackOboe>>,
}

pub struct AudioToTsOboe {
	stream: Option<AudioStreamAsync<Input, AudioToTsCallbackOboe>>,
}

struct TsToAudioCallbackOboe(TsToAudioCallback);

struct AudioToTsCallbackOboe(AudioToTsCallback);

impl TsToAudioImpl for TsToAudioOboe {
	fn started(_: &mut TsToAudio<Self>, ctx: &mut Context<TsToAudio<Self>>) {
		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			// Restart on errors
			if t2a
				.real_impl
				.stream
				.as_ref()
				.map(|d| {
					let state = d.get_state();
					debug!(?state, "Checking playback state");
					state != StreamState::Open
						&& state != StreamState::Starting
						&& state != StreamState::Started
						&& state != StreamState::Pausing
						&& state != StreamState::Paused
				})
				.unwrap_or(false)
			{
				debug!("Re-opening playback");
				// Try to reconnect to audio
				Self::open_playback(t2a);
			}

			let data_empty = t2a.data.lock().unwrap().get_queues().is_empty();
			if !data_empty {
				Self::got_play_msg(t2a);
			} else if let Some(stream) = &mut t2a.real_impl.stream {
				let state = stream.get_state();
				if state == StreamState::Starting || state == StreamState::Started {
					debug!(?state, "Pausing playback");
					t2a.real_impl.stop();
					// Resuming a paused stream hangs the thread, so stop and re-open instead.
				}
			}
		});
	}

	fn got_play_msg(ts_to_audio: &mut TsToAudio<Self>) {
		if let Some(stream) = &mut ts_to_audio.real_impl.stream {
			let state = stream.get_state();
			if state != StreamState::Starting && state != StreamState::Started {
				debug!(?state, "Resuming playback");
				if let Err(error) = stream.start() {
					error!(%error, "Failed to start playback stream");
				}
			}
		} else {
			Self::open_playback(ts_to_audio);
		}
	}

	fn reset(ts_to_audio: &mut TsToAudio<Self>) { ts_to_audio.real_impl.stop(); }

	fn get_audio_devices(_: &mut TsToAudio<Self>) -> Vec<String> { Vec::new() }
}

impl TsToAudioOboe {
	pub fn new() -> Self { Self { stream: None } }

	fn open_playback(ts_to_audio: &mut TsToAudio<Self>) {
		// Stop previous stream
		ts_to_audio.real_impl.stop();

		let callback = TsToAudioCallbackOboe(ts_to_audio.get_callback());

		match AudioStreamBuilder::default()
			.set_performance_mode(PerformanceMode::LowLatency)
			.set_sharing_mode(SharingMode::Shared)
			.set_format::<f32>()
			.set_channel_count::<Stereo>()
			.set_sample_rate(super::SAMPLE_RATE as i32)
			.set_buffer_capacity_in_frames(super::USUAL_SAMPLE_COUNT as i32)
			.set_callback(callback)
			.open_stream()
		{
			Ok(mut stream) => {
				if let Err(error) = stream.start() {
					error!(%error, "Failed to start playback stream");
				}
				debug!("Initial start returned");
				ts_to_audio.real_impl.stream = Some(stream);
			}
			Err(error) => {
				ts_to_audio.real_impl.stream = None;
				error!(%error, "Failed to open playback stream");
			}
		}
	}

	fn stop(&mut self) {
		if let Some(mut stream) = self.stream.take() {
			if let Err(error) = stream.stop() {
				warn!(%error, "Failed to stop playback stream");
			}
			if let Err(error) = stream.close() {
				warn!(%error, "Failed to close playback stream");
			}
		}
	}
}

impl Drop for TsToAudioOboe {
	fn drop(&mut self) { self.stop(); }
}

impl AudioOutputCallback for TsToAudioCallbackOboe {
	type FrameType = (f32, Stereo);
	fn on_audio_ready(
		&mut self, _: &mut dyn AudioOutputStreamSafe, buffer: &mut [(f32, f32)],
	) -> DataCallbackResult {
		let buffer: &mut [f32] = unsafe {
			std::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut f32, buffer.len() * 2)
		};

		self.0.callback(buffer);

		DataCallbackResult::Continue
	}
}

impl AudioToTsImpl for AudioToTsOboe {
	fn started(_: &mut AudioToTs<Self>, ctx: &mut Context<AudioToTs<Self>>) {
		ctx.run_interval(Duration::from_secs(1), |a2t, _| {
			// Restart on errors
			if a2t
				.real_impl
				.stream
				.as_ref()
				.map(|d| {
					let state = d.get_state();
					debug!(?state, "Checking capture state");
					state != StreamState::Open
						&& state != StreamState::Starting
						&& state != StreamState::Started
						&& state != StreamState::Pausing
						&& state != StreamState::Paused
				})
				.unwrap_or(false)
			{
				debug!("Re-opening capture");
				// Try to reconnect to audio
				a2t.real_impl.stop();
			}
			a2t.update_device_state();
		});
	}

	fn reset(audio_to_ts: &mut AudioToTs<Self>) { audio_to_ts.real_impl.stop(); }

	fn set_playing(audio_to_ts: &mut AudioToTs<Self>, playing: bool) {
		if playing {
			if let Some(stream) = &mut audio_to_ts.real_impl.stream {
				let state = stream.get_state();
				if state != StreamState::Starting && state != StreamState::Started {
					debug!(?state, "Resuming capture");
					if let Err(error) = stream.start() {
						error!(%error, "Failed to start capture stream");
					}
				}
			} else {
				Self::open_capture(audio_to_ts);
			}
		} else {
			audio_to_ts.real_impl.stop();
		}
	}

	fn get_audio_devices(_: &mut AudioToTs<Self>) -> Vec<String> { Vec::new() }
}

impl AudioToTsOboe {
	pub fn new() -> Self { Self { stream: None } }

	fn open_capture(audio_to_ts: &mut AudioToTs<Self>) {
		// Stop previous stream
		audio_to_ts.real_impl.stop();

		let callback = AudioToTsCallbackOboe(audio_to_ts.get_callback(audiopus::Channels::Mono));

		match AudioStreamBuilder::default()
			.set_input()
			.set_performance_mode(PerformanceMode::LowLatency)
			.set_sharing_mode(SharingMode::Shared)
			.set_format::<f32>()
			.set_channel_count::<Mono>()
			.set_sample_rate(super::SAMPLE_RATE as i32)
			.set_buffer_capacity_in_frames(super::USUAL_SAMPLE_COUNT as i32)
			.set_callback(callback)
			.open_stream()
		{
			Ok(mut stream) => {
				if let Err(error) = stream.start() {
					error!(%error, "Failed to start capture stream");
				}
				debug!("Initial start returned");
				audio_to_ts.real_impl.stream = Some(stream);
			}
			Err(error) => {
				audio_to_ts.real_impl.stream = None;
				error!(%error, "Failed to open capture stream");
			}
		}
	}

	fn stop(&mut self) {
		if let Some(mut stream) = self.stream.take() {
			if let Err(error) = stream.stop() {
				warn!(%error, "Failed to stop capture stream");
			}
			if let Err(error) = stream.close() {
				warn!(%error, "Failed to close capture stream");
			}
		}
	}
}

impl Drop for AudioToTsOboe {
	fn drop(&mut self) { self.stop(); }
}

impl AudioInputCallback for AudioToTsCallbackOboe {
	type FrameType = (f32, Mono);
	fn on_audio_ready(
		&mut self, _: &mut dyn AudioInputStreamSafe, buffer: &[f32],
	) -> DataCallbackResult {
		self.0.callback(buffer);

		DataCallbackResult::Continue
	}
}
