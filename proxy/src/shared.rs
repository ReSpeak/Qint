/// Functionality which usually doesn't belong directly to the proxy
/// but is shared between the web and tauri backend.
use serde::{Deserialize, Serialize};

#[derive(Default, Serialize)]
pub struct AudioDeviceList {
	pub capture: Vec<String>,
	pub playback: Vec<String>,
}

#[derive(Deserialize)]
pub struct UpdateIdentityOptions {
	pub name: Option<String>,
}
