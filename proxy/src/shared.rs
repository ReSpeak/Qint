/// Functionality which usually doesn't belong directly to the proxy
/// but is shared between the web and tauri backend.
use serde::Serialize;

use crate::{audio::GetAudioDevices, QintState};

#[derive(Default, Serialize)]
pub struct AudioDeviceList {
	pub capture: Vec<String>,
	pub playback: Vec<String>,
}

pub async fn audio_device_list(state: &QintState) -> AudioDeviceList {
	if let Some(ad) = &state.audio_data {
		let capture = ad.a2ts.send(GetAudioDevices()).await.unwrap_or(Vec::new());
		let playback = ad.ts2a.send(GetAudioDevices()).await.unwrap_or(Vec::new());
		AudioDeviceList { capture, playback }
	} else {
		AudioDeviceList::default()
	}
}
