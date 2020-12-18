use anyhow::Result;
use serde::{Deserialize, Serialize};
use slog::warn;

use crate::{websocket, Tristate};

pub use imp::{KeyCode, Shortcuts};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Shortcut {
	pub keycode: KeyCode,
	pub action: Action,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ShortcutConfig {
	pub actions: Vec<Shortcut>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
pub enum Action {
	Away(Tristate),
	InputMute(Tristate),
	OutputMute(Tristate),
}

async fn h<T: Clone + Send + 'static>(state: &crate::State, t: T)
where
	websocket::Ws: actix::Handler<T>,
	T: actix::Message<Result = Result<()>>,
{
	let cons = state.connections.lock().unwrap().values().cloned().collect::<Vec<_>>();
	for c in cons {
		match c.send(t.clone()).await {
			Err(e) => warn!(state.logger, "Failed to run action"; "error" => %e),
			Ok(Err(e)) => warn!(state.logger, "Failed to run action"; "error" => %e),
			_ => {}
		}
	}
}

impl Action {
	pub async fn run(&self, state: &crate::State) {
		match self {
			Self::InputMute(b) => h(state, websocket::SetInputMutedMsg(*b)).await,
			Self::OutputMute(b) => h(state, websocket::SetOutputMutedMsg(*b)).await,
			Self::Away(b) => h(state, websocket::SetAwayMsg(*b)).await,
		}
	}
}

#[cfg(windows)]
mod imp {
	use std::sync::Arc;

	use anyhow::Result;
	use livesplit_hotkey::*;
	use tokio::runtime::Handle;

	use super::ShortcutConfig;
	use crate::State;

	pub use livesplit_hotkey::KeyCode;

	pub struct Shortcuts {
		config: ShortcutConfig,
		hook: Hook,
	}

	pub fn _key_list() -> Vec<String> {
		// https://github.com/LiveSplit/livesplit-core/blob/master/crates/livesplit-hotkey/src/windows/key_code.rs
		[
			"LButton",
			"RButton",
			"Cancel",
			"MButton",
			"XButton1",
			"XButton2",
			"Back",
			"Tab",
			"Clear",
			"Return",
			"Shift",
			"Control",
			"Menu",
			"Pause",
			"Capital",
			"Kana",
			"Junja",
			"Final",
			"Kanji",
			"Escape",
			"Convert",
			"NonConvert",
			"Accept",
			"ModeChange",
			"Space",
			"Prior",
			"Next",
			"End",
			"Home",
			"Left",
			"Up",
			"Right",
			"Down",
			"Select",
			"Print",
			"Execute",
			"Snapshot",
			"Insert",
			"Delete",
			"Help",
			"D0",
			"D1",
			"D2",
			"D3",
			"D4",
			"D5",
			"D6",
			"D7",
			"D8",
			"D9",
			"A",
			"B",
			"C",
			"D",
			"E",
			"F",
			"G",
			"H",
			"I",
			"J",
			"K",
			"L",
			"M",
			"N",
			"O",
			"P",
			"Q",
			"R",
			"S",
			"T",
			"U",
			"V",
			"W",
			"X",
			"Y",
			"Z",
			"LeftWin",
			"RightWin",
			"Apps",
			"Sleep",
			"NumPad0",
			"NumPad1",
			"NumPad2",
			"NumPad3",
			"NumPad4",
			"NumPad5",
			"NumPad6",
			"NumPad7",
			"NumPad8",
			"NumPad9",
			"Multiply",
			"Add",
			"Separator",
			"Subtract",
			"Decimal",
			"Divide",
			"F1",
			"F2",
			"F3",
			"F4",
			"F5",
			"F6",
			"F7",
			"F8",
			"F9",
			"F10",
			"F11",
			"F12",
			"F13",
			"F14",
			"F15",
			"F16",
			"F17",
			"F18",
			"F19",
			"F20",
			"F21",
			"F22",
			"F23",
			"F24",
			"NumLock",
			"Scroll",
			"LeftShift",
			"RightShift",
			"LeftControl",
			"RightControl",
			"LeftMenu",
			"RightMenu",
			"BrowserBack",
			"BrowserForward",
			"BrowserRefresh",
			"BrowserStop",
			"BrowserSearch",
			"BrowserFavorites",
			"BrowserHome",
			"VolumeMute",
			"VolumeDown",
			"VolumeUp",
			"MediaNextTrack",
			"MediaPrevTrack",
			"MediaStop",
			"MediaPlayPause",
			"LaunchMail",
			"LaunchMediaSelect",
			"LaunchApp1",
			"LaunchApp2",
			"Oem1",
			"OemPlus",
			"OemComma",
			"OemMinus",
			"OemPeriod",
			"Oem2",
			"Oem3",
			"Oem4",
			"Oem5",
			"Oem6",
			"Oem7",
			"Oem8",
			"Oem102",
			"ProcessKey",
			"Packet",
			"Attn",
			"CrSel",
			"ExSel",
			"ErEof",
			"Play",
			"Zoom",
			"NoName",
			"Pa1",
			"OemClear",
		]
		.iter()
		.map(|s| s.to_string())
		.collect()
	}

	impl Shortcuts {
		pub fn new(config: ShortcutConfig) -> Result<Self> {
			Ok(Self { config, hook: Hook::new()? })
		}

		pub fn apply_config(&self, state: &Arc<State>) -> Result<()> {
			for a in &self.config.actions {
				let action = a.action;
				let state = state.clone();
				let handle = Handle::current();
				self.hook.register(a.keycode, move || {
					let state = state.clone();
					handle.spawn(async move {
						action.run(&state).await;
					});
				})?;
			}
			Ok(())
		}
	}
}

#[cfg(not(windows))]
mod imp {
	use std::sync::Arc;

	use anyhow::Result;
	use serde::{Deserialize, Serialize};

	use super::ShortcutConfig;
	use crate::State;

	#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
	pub enum KeyCode {}

	#[derive(Debug)]
	pub struct Shortcuts {
		config: ShortcutConfig,
	}

	pub fn _key_list() -> Vec<String> { Vec::new() }

	impl Shortcuts {
		pub fn new(config: ShortcutConfig) -> Result<Self> { Ok(Self { config }) }

		pub fn apply_config(&self, _: &Arc<State>) -> Result<()> { Ok(()) }
	}
}
