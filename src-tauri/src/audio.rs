use slog::Logger;
use std::sync::{
	atomic::{AtomicBool, Ordering},
	Arc, Mutex,
};
use tauri::Window;

use qint_proxy::{
	audio::audio_to_ts::{AddLoudnessListenerMsg, LoudnessTrait},
	connection::CaptureLoudnessMsg,
	QintState,
};

pub struct LoudnessShare {
	pub current_listener: Mutex<Option<Arc<AtomicBool>>>,
}

impl LoudnessShare {
	pub fn new() -> Self {
		Self { current_listener: Mutex::new(None) }
	}

	pub async fn enable(&self, state: &QintState, window: Window) {
		let callback = {
			let mut current = self.current_listener.lock().unwrap();
			if current.is_some() {
				return;
			} else {
				let listening = Arc::new(AtomicBool::new(true));
				*current = Some(listening.clone());
				LoudnessCallback { logger: state.logger.clone(), window, listening }
			}
		};

		if let Some(ad) = &state.audio_data {
			match ad.a2ts.send(AddLoudnessListenerMsg(Box::new(callback))).await {
				Ok(_handle) => {}
				Err(e) => warn!(state.logger, "Failed add loudness listener"; "error" => %e),
			}
		}
	}

	pub fn disable(&self) {
		let mut current = self.current_listener.lock().unwrap();
		if let Some(listener) = current.take() {
			listener.store(false, Ordering::Relaxed);
		}
	}
}

pub struct LoudnessCallback {
	pub logger: Logger,
	pub window: Window,
	pub listening: Arc<AtomicBool>,
}

#[derive(Serialize)]
struct LoudnessFrame(pub f64, pub f32);

impl LoudnessTrait for LoudnessCallback {
	fn send(&self, msg: CaptureLoudnessMsg) {
		let res = self.window.emit("loudness", LoudnessFrame(msg.0, msg.1));
		if let Err(e) = res {
			warn!(self.logger, "Failed sending to frontend"; "error" => %e);
		}
	}

	fn connected(&self) -> bool {
		self.listening.load(Ordering::Relaxed)
	}
}
