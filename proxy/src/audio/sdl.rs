use std::time::Duration;

use actix::{AsyncContext, Context};
use anyhow::{Result, format_err};
use sdl2::AudioSubsystem;
use sdl2::audio::{AudioCallback, AudioDevice, AudioSpecDesired, AudioStatus};
use tracing::{debug, error};

use super::audio_to_ts::*;
use super::ts_to_audio::*;

pub struct TsToAudioSdl {
	audio_subsystem: AudioSubsystem,
	device: Option<AudioDevice<TsToAudioCallbackSdl>>,
}

pub struct AudioToTsSdl {
	audio_subsystem: AudioSubsystem,
	device: Option<AudioDevice<AudioToTsCallbackSdl>>,
}

struct TsToAudioCallbackSdl(TsToAudioCallback);

struct AudioToTsCallbackSdl(AudioToTsCallback);

impl TsToAudioImpl for TsToAudioSdl {
	fn started(ts_to_audio: &mut TsToAudio<Self>, ctx: &mut Context<TsToAudio<Self>>) {
		Self::open_playback(ts_to_audio);

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			// Restart on errors
			if t2a
				.real_impl
				.device
				.as_ref()
				.map(|d| d.status() == AudioStatus::Stopped)
				.unwrap_or(true)
			{
				// Try to reconnect to audio
				Self::open_playback(t2a);
			}

			if let Some(device) = &t2a.real_impl.device {
				let data_empty = t2a.data.lock().unwrap().get_queues().is_empty();
				if device.status() == AudioStatus::Paused && !data_empty {
					debug!("Resuming playback");
					device.resume();
				} else if device.status() == AudioStatus::Playing && data_empty {
					debug!("Pausing playback");
					device.pause();
				}
			}
		});
	}

	fn got_play_msg(ts_to_audio: &mut TsToAudio<Self>) {
		if let Some(device) = &ts_to_audio.real_impl.device {
			if device.status() == AudioStatus::Paused {
				debug!("Resuming playback");
				device.resume();
			}
		}
	}

	fn reset(ts_to_audio: &mut TsToAudio<Self>) { Self::open_playback(ts_to_audio); }

	fn get_audio_devices(ts_to_audio: &mut TsToAudio<Self>) -> Vec<String> {
		let mut devices = Vec::new();
		if let Some(dev_cnt) = ts_to_audio.real_impl.audio_subsystem.num_audio_playback_devices() {
			for dev_index in 0..dev_cnt {
				if let Ok(dev_name) =
					ts_to_audio.real_impl.audio_subsystem.audio_playback_device_name(dev_index)
				{
					devices.push(dev_name);
				}
			}
		}
		devices
	}
}

impl TsToAudioSdl {
	pub fn new(audio_subsystem: AudioSubsystem) -> Result<Self> {
		Ok(Self { audio_subsystem, device: None })
	}

	fn open_playback(ts_to_audio: &mut TsToAudio<Self>) {
		let desired_spec = AudioSpecDesired {
			freq: Some(super::SAMPLE_RATE as i32),
			channels: Some(2),
			samples: Some(super::USUAL_SAMPLE_COUNT as u16),
		};

		let callback = TsToAudioCallbackSdl(ts_to_audio.get_callback());
		match ts_to_audio.real_impl.audio_subsystem.open_playback(
			ts_to_audio.preferred_device.as_deref(),
			&desired_spec,
			|spec| {
				// This spec will always be the desired spec, the sdl wrapper passes
				// zero as `allowed_changes`.
				debug!(
					?spec,
					driver = ts_to_audio.real_impl.audio_subsystem.current_audio_driver(),
					"Got playback spec"
				);
				callback
			},
		) {
			Ok(device) => ts_to_audio.real_impl.device = Some(device),
			Err(error) => {
				ts_to_audio.real_impl.device = None;
				error!(%error, "Failed to open playback device");
			}
		}
	}
}

impl AudioCallback for TsToAudioCallbackSdl {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) { self.0.callback(buffer); }
}

impl AudioToTsImpl for AudioToTsSdl {
	fn started(audio_to_ts: &mut AudioToTs<Self>, ctx: &mut Context<AudioToTs<Self>>) {
		Self::open_capture(audio_to_ts);

		ctx.run_interval(Duration::from_secs(1), |a2t, _| {
			if a2t
				.real_impl
				.device
				.as_ref()
				.map(|d| d.status() == AudioStatus::Stopped)
				.unwrap_or(true)
			{
				// Try to reconnect to audio
				Self::open_capture(a2t);
			}
		});
	}

	fn reset(audio_to_ts: &mut AudioToTs<Self>) { Self::open_capture(audio_to_ts); }

	fn set_playing(audio_to_ts: &mut AudioToTs<Self>, playing: bool) {
		if let Some(device) = &audio_to_ts.real_impl.device {
			if playing {
				device.resume();
			} else {
				device.pause();
			}
		}
	}

	fn get_audio_devices(audio_to_ts: &mut AudioToTs<Self>) -> Vec<String> {
		let mut devices = Vec::new();
		if let Some(dev_cnt) = audio_to_ts.real_impl.audio_subsystem.num_audio_capture_devices() {
			for dev_index in 0..dev_cnt {
				if let Ok(dev_name) =
					audio_to_ts.real_impl.audio_subsystem.audio_capture_device_name(dev_index)
				{
					devices.push(dev_name);
				}
			}
		}
		devices
	}
}

impl AudioToTsSdl {
	pub fn new(audio_subsystem: AudioSubsystem) -> Self { Self { audio_subsystem, device: None } }

	fn open_capture(audio_to_ts: &mut AudioToTs<Self>) {
		let desired_spec = AudioSpecDesired {
			freq: Some(super::SAMPLE_RATE as i32),
			channels: Some(1),
			// Default sample size, 20 ms per packet
			samples: Some(super::USUAL_SAMPLE_COUNT as u16),
		};

		match audio_to_ts
			.real_impl
			.audio_subsystem
			.open_capture(audio_to_ts.preferred_device.as_deref(), &desired_spec, |spec| {
				// This spec will always be the desired spec, the sdl wrapper
				// passes zero as `allowed_changes`.
				debug!(
					?spec,
					driver = audio_to_ts.real_impl.audio_subsystem.current_audio_driver(),
					"Got capture spec"
				);
				let channels = if spec.channels == 1 {
					audiopus::Channels::Mono
				} else {
					audiopus::Channels::Stereo
				};

				AudioToTsCallbackSdl(audio_to_ts.get_callback(channels))
			})
			.map_err(|e| format_err!("SDL error: {}", e))
		{
			Ok(device) => {
				audio_to_ts.real_impl.device = Some(device);
				audio_to_ts.update_device_state();
			}
			Err(error) => {
				error!(%error, "Failed to open capture device");
			}
		}
	}
}

impl AudioCallback for AudioToTsCallbackSdl {
	type Channel = f32;
	fn callback(&mut self, buffer: &mut [Self::Channel]) { self.0.callback_mut_buffer(buffer); }
}
