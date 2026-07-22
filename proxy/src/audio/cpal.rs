use std::time::Duration;

use actix::{AsyncContext, Context};
use anyhow::format_err;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Stream, StreamConfig};
use tracing::{debug, error, trace, warn};

use super::audio_to_ts::*;
use super::ts_to_audio::*;

pub struct TsToAudioCpal {
	host: Host,
	device: Option<Device>,
	stream: Option<Stream>,
	is_playing: bool,
}

pub struct AudioToTsCpal {
	host: Host,
	device: Option<Device>,
	stream: Option<Stream>,
	is_playing: bool,
}

struct TsToAudioCallbackCpal(TsToAudioCallback);

struct AudioToTsCallbackCpal(AudioToTsCallback);

impl TsToAudioImpl for TsToAudioCpal {
	fn started(ts_to_audio: &mut TsToAudio<Self>, ctx: &mut Context<TsToAudio<Self>>) {
		Self::open_playback(ts_to_audio);

		ctx.run_interval(Duration::from_secs(1), |t2a, _| {
			// Restart on errors
			if t2a.real_impl.stream.is_none() {
				// Try to reconnect to audio
				Self::open_playback(t2a);
			}

			if let Some(stream) = &t2a.real_impl.stream {
				let data_empty = t2a.data.lock().unwrap().get_queues().is_empty();
				if !t2a.real_impl.is_playing && !data_empty {
					debug!("Resuming playback");
					if let Err(error) = stream.play() {
						error!(%error, "Failed to start playback stream");
						t2a.real_impl.stream = None;
					} else {
						t2a.real_impl.is_playing = true;
					}
				} else if t2a.real_impl.is_playing && data_empty {
					debug!("Pausing playback");
					if let Err(error) = stream.pause() {
						error!(%error, "Failed to pause playback stream");
						t2a.real_impl.stream = None;
					}
					t2a.real_impl.is_playing = false;
				}
			}
		});
	}

	fn got_play_msg(ts_to_audio: &mut TsToAudio<Self>) {
		if let Some(stream) = &ts_to_audio.real_impl.stream {
			if !ts_to_audio.real_impl.is_playing {
				debug!("Resuming playback");
				if let Err(error) = stream.play() {
					error!(%error, "Failed to start playback stream");
					ts_to_audio.real_impl.stream = None;
				} else {
					ts_to_audio.real_impl.is_playing = true;
				}
			}
		}
	}

	fn reset(ts_to_audio: &mut TsToAudio<Self>) { Self::open_playback(ts_to_audio); }

	fn get_audio_devices(ts_to_audio: &mut TsToAudio<Self>) -> Vec<String> {
		let mut devices = Vec::new();
		match ts_to_audio.real_impl.host.output_devices() {
			Ok(devs) => {
				for dev in devs {
					match dev.id() {
						Ok(id) => devices.push(id.to_string()),
						Err(error) => warn!(%error, "Failed to get device id"),
					}
				}
			}
			Err(error) => warn!(%error, "Failed to list output devices"),
		}
		devices
	}
}

impl TsToAudioCpal {
	pub fn new(host: Host) -> Self { Self { host, device: None, stream: None, is_playing: false } }

	fn open_playback(ts_to_audio: &mut TsToAudio<Self>) {
		ts_to_audio.real_impl.device = None;
		ts_to_audio.real_impl.stream = None;
		ts_to_audio.real_impl.is_playing = false;
		let config = StreamConfig {
			channels: 2,
			sample_rate: super::SAMPLE_RATE as u32,
			buffer_size: cpal::BufferSize::Fixed(super::USUAL_SAMPLE_COUNT as u32),
		};

		let mut callback = TsToAudioCallbackCpal(ts_to_audio.get_callback());

		// Find preferred device
		if let Some(preferred_device) = &ts_to_audio.preferred_device {
			match ts_to_audio.real_impl.host.output_devices() {
				Ok(devs) => {
					for dev in devs {
						match dev.id() {
							Ok(id) => {
								if id.to_string() == *preferred_device {
									ts_to_audio.real_impl.device = Some(dev);
									break;
								}
							}
							Err(error) => warn!(%error, "Failed to get device id"),
						}
					}
				}
				Err(error) => warn!(%error, "Failed to list output devices"),
			}
		}

		if ts_to_audio.real_impl.device.is_none() {
			// Fallback to default device
			ts_to_audio.real_impl.device = ts_to_audio.real_impl.host.default_output_device();
		}

		if let Some(device) = &ts_to_audio.real_impl.device {
			match device.build_output_stream(
				config,
				move |audio_data, _| callback.callback(audio_data),
				|error| error!(%error, "Error during audio playback"),
				Some(Duration::from_secs(5)),
			) {
				Ok(stream) => {
					// pipewire currently starts in playing state (fixed in the next release)
					#[cfg(target_os = "linux")]
					if ts_to_audio.real_impl.host.id() == cpal::HostId::PipeWire {
						let _ = stream.pause();
					}
					ts_to_audio.real_impl.stream = Some(stream);
				}
				Err(error) => {
					ts_to_audio.real_impl.stream = None;
					error!(%error, "Failed to open playback device");
				}
			}
		}
	}
}

impl TsToAudioCallbackCpal {
	fn callback(&mut self, buffer: &mut [f32]) { self.0.callback(buffer); }
}

impl AudioToTsImpl for AudioToTsCpal {
	fn started(audio_to_ts: &mut AudioToTs<Self>, ctx: &mut Context<AudioToTs<Self>>) {
		Self::open_capture(audio_to_ts);

		ctx.run_interval(Duration::from_secs(1), |a2t, _| {
			if a2t.real_impl.device.is_none() {
				// Try to reconnect to audio
				Self::open_capture(a2t);
			}
			a2t.update_device_state();
		});
	}

	fn reset(audio_to_ts: &mut AudioToTs<Self>) { Self::open_capture(audio_to_ts); }

	fn set_playing(audio_to_ts: &mut AudioToTs<Self>, playing: bool) {
		if playing {
			Self::open_stream(audio_to_ts);
			if let Some(stream) = &audio_to_ts.real_impl.stream {
				if !audio_to_ts.real_impl.is_playing {
					if let Err(error) = stream.play() {
						error!(%error, "Failed to start capture stream");
						audio_to_ts.real_impl.stream = None;
					}
					audio_to_ts.real_impl.is_playing = true;
				}
			}
		} else {
			// Remove stream instead of pausing.
			// Not be supported on all platforms, e.g. Android.
			// Other platforms with a privacy widget show if there are open streams.
			// This should only happen on muting, so rather seldom.
			audio_to_ts.real_impl.stream = None;
		}
	}

	fn get_audio_devices(audio_to_ts: &mut AudioToTs<Self>) -> Vec<String> {
		let mut devices = Vec::new();
		match audio_to_ts.real_impl.host.input_devices() {
			Ok(devs) => {
				for dev in devs {
					match dev.id() {
						Ok(id) => devices.push(id.to_string()),
						Err(error) => warn!(%error, "Failed to get device id"),
					}
				}
			}
			Err(error) => warn!(%error, "Failed to list input devices"),
		}
		devices
	}
}

impl AudioToTsCpal {
	pub fn new(host: Host) -> Self { Self { host, device: None, stream: None, is_playing: false } }

	fn open_capture(audio_to_ts: &mut AudioToTs<Self>) {
		audio_to_ts.real_impl.device = None;
		audio_to_ts.real_impl.stream = None;
		audio_to_ts.real_impl.is_playing = false;

		// Find preferred device
		if let Some(preferred_device) = &audio_to_ts.preferred_device {
			match audio_to_ts.real_impl.host.input_devices() {
				Ok(devs) => {
					for dev in devs {
						match dev.id() {
							Ok(id) => {
								if id.to_string() == *preferred_device {
									audio_to_ts.real_impl.device = Some(dev);
									break;
								}
							}
							Err(error) => warn!(%error, "Failed to get device id"),
						}
					}
				}
				Err(error) => warn!(%error, "Failed to list input devices"),
			}
		}

		if audio_to_ts.real_impl.device.is_none() {
			// Fallback to default device
			audio_to_ts.real_impl.device = audio_to_ts.real_impl.host.default_input_device();
		}
		Self::open_stream(audio_to_ts);
	}

	fn open_stream(audio_to_ts: &mut AudioToTs<Self>) {
		if audio_to_ts.real_impl.stream.is_some() || !audio_to_ts.should_play() {
			return;
		}

		if let Some(device) = &audio_to_ts.real_impl.device {
			let mut callback =
				AudioToTsCallbackCpal(audio_to_ts.get_callback(audiopus::Channels::Mono));

			let config = StreamConfig {
				channels: 1,
				sample_rate: super::SAMPLE_RATE as u32,
				buffer_size: cpal::BufferSize::Fixed(super::USUAL_SAMPLE_COUNT as u32),
			};
			match device
				.build_input_stream(
					config,
					move |audio_data, _| callback.callback(audio_data),
					|error| error!(%error, "Error during audio capture"),
					Some(Duration::from_secs(5)),
				)
				.map_err(|e| format_err!("cpal error: {}", e))
			{
				Ok(stream) => {
					// Reset is_playing, so we start the stream
					audio_to_ts.real_impl.is_playing = false;
					audio_to_ts.real_impl.stream = Some(stream);
					Self::set_playing(audio_to_ts, true);
				}
				Err(error) => {
					error!(%error, "Failed to open capture device");
				}
			}
		}
	}
}

impl AudioToTsCallbackCpal {
	fn callback(&mut self, buffer: &[f32]) { self.0.callback(buffer); }
}
