use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use serde::Serialize;
use tauri::{Emitter, Window};
use tracing::warn;

use qint_proxy::{
	audio::audio_to_ts::{AddLoudnessListenerMsg, LoudnessTrait},
	connection::CaptureLoudnessMsg,
	QintState,
};

pub struct LoudnessShare {
	pub current_listener: Arc<AtomicU32>,
}

impl LoudnessShare {
	pub fn new() -> Self { Self { current_listener: Default::default() } }

	pub async fn enable(&self, state: &QintState, window: Window) {
		let own_id = self.current_listener.fetch_add(1, Ordering::Relaxed) + 1;
		let callback =
			LoudnessCallback { window, listening: self.current_listener.clone(), own_id };

		if let Some(ad) = &state.audio_data {
			if let Err(error) = ad.a2ts.send(AddLoudnessListenerMsg(Box::new(callback))).await {
				warn!(%error, "Failed add loudness listener");
			}
		}
	}

	pub fn disable(&self) {
		// Increase id, so the listener discards itself
		self.current_listener.fetch_add(1, Ordering::Relaxed);
	}
}

struct LoudnessCallback {
	window: Window,
	listening: Arc<AtomicU32>,
	own_id: u32,
}

#[derive(Serialize, Copy, Clone)]
struct LoudnessFrame(pub f64, pub f32);

impl LoudnessTrait for LoudnessCallback {
	fn send(&self, msg: CaptureLoudnessMsg) {
		let res = self.window.emit("loudness", LoudnessFrame(msg.0, msg.1));
		if let Err(error) = res {
			warn!(%error, "Failed sending to frontend");
		}
	}

	fn connected(&self) -> bool { self.listening.load(Ordering::Relaxed) == self.own_id }
}
