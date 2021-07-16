use std::sync::Arc;

use crate::QintState;
use actix::Addr;
use futures::{future, StreamExt};
use serde::{Deserialize, Serialize};
use slog::error;
use tsclientlib::prelude::*;

use crate::{connection, MuteState};
use connection::QintConnection;

pub use imp::{Hotkeys, KeyCode};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct Hotkey {
	pub keycode: KeyCode,
	pub action: Action,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct HotkeyConfig {
	pub actions: Vec<Hotkey>,
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
pub enum Action {
	Away,
	InputMute,
	OutputMute,
}

/// Get input mute state for all current connections.
/// The `bool` is `false` if the connection cannot talk, even if unmuted, because it is away or the
/// output is disabled
async fn get_input_mute_states(state: &QintState) -> Vec<(MuteState, bool, Addr<QintConnection>)> {
	state
		.aggregate(|con, con_addr| {
			// Ignore servers where output is muted or disabled or away
			con.get_own_client().map(|c| {
				(
					if !c.input_hardware_enabled {
						MuteState::Disabled
					} else if c.input_muted {
						MuteState::Muted
					} else {
						MuteState::None
					},
					!c.output_muted && c.output_hardware_enabled && c.away_message.is_none(),
					con_addr,
				)
			})
		})
		.filter_map(|f| future::ready(f))
		.collect()
		.await
}

/// Get output mute state for all current connections.
async fn get_output_mute_states(state: &QintState) -> Vec<(MuteState, Addr<QintConnection>)> {
	state
		.aggregate(|con, con_addr| {
			con.get_own_client().map(|c| {
				(
					if !c.output_hardware_enabled {
						MuteState::Disabled
					} else if c.output_muted {
						MuteState::Muted
					} else {
						MuteState::None
					},
					con_addr,
				)
			})
		})
		.filter_map(|f| future::ready(f))
		.collect()
		.await
}

/// Get away state for all current connections.
///
/// Returns `true` for every connection that is muted.
async fn get_away_states(state: &QintState) -> Vec<(bool, Addr<QintConnection>)> {
	state
		.aggregate(|con, con_addr| {
			con.get_own_client().map(|c| (c.away_message.is_some(), con_addr))
		})
		.filter_map(|f| future::ready(f))
		.collect()
		.await
}

impl Action {
	pub async fn run(&self, state: &Arc<QintState>) {
		match self {
			Self::InputMute => {
				let states = get_input_mute_states(state).await;
				if states.is_empty() {
					// No connections, toggle default
					if let (Err(e), _) = QintState::modify_settings(state, |settings| {
						let mut state = settings.get_default_mute_states();
						if state.input != MuteState::None {
							state.input = MuteState::None;
						} else {
							state.input = MuteState::Muted;
						}
						settings.set_default_mute_states(state);
						Ok(Default::default())
					}) {
						error!(state.logger, "Failed to change default mute state"; "error" => %e);
					}
					return;
				}

				// Filter out servers, where we cannot talk anyway (away or output muted), except
				// if this is the case for all servers.
				let states: Vec<_> = if states.iter().any(|(_, can_talk, _)| *can_talk) {
					states
						.into_iter()
						.filter_map(|(s, can_talk, a)| if can_talk { Some((s, a)) } else { None })
						.collect()
				} else {
					states.into_iter().map(|(s, _, a)| (s, a)).collect()
				};
				if states.iter().all(|(s, _)| *s == MuteState::Disabled) {
					// If all servers have disabled input, enable input
					state
						.send_each_con(states.into_iter().map(|(_, c)| c), |c| {
							Some(c.client_update().set_input_hardware_enabled(true))
						})
						.await;
				} else {
					// Toggle mute state, unless input is disabled
					state
						.send_each_con(
							states.into_iter().filter_map(|(m, c)| {
								if m == MuteState::Disabled { None } else { Some(c) }
							}),
							|c| {
								c.clients.get(&c.own_client).and_then(|client| {
									Some(c.client_update().set_input_muted(!client.input_muted))
								})
							},
						)
						.await;
				}
			}
			Self::OutputMute => {
				let states = get_output_mute_states(state).await;
				if states.is_empty() {
					// No connections, toggle default
					if let (Err(e), _) = QintState::modify_settings(state, |settings| {
						let mut state = settings.get_default_mute_states();
						if state.output != MuteState::None {
							state.output = MuteState::None;
						} else {
							state.output = MuteState::Muted;
						}
						settings.set_default_mute_states(state);
						Ok(Default::default())
					}) {
						error!(state.logger, "Failed to change default mute state"; "error" => %e);
					}
					return;
				}

				if states.iter().all(|(s, _)| *s == MuteState::Disabled) {
					// If all servers have disabled output, enable output
					state
						.send_each_con(states.into_iter().map(|(_, c)| c), |c| {
							Some(c.client_update().set_output_hardware_enabled(true))
						})
						.await;
				} else {
					// Toggle mute state, unless output is disabled
					state
						.send_each_con(
							states.into_iter().filter_map(|(m, c)| {
								if m == MuteState::Disabled { None } else { Some(c) }
							}),
							|c| {
								c.clients.get(&c.own_client).and_then(|client| {
									Some(c.client_update().set_output_muted(!client.output_muted))
								})
							},
						)
						.await;
				}
			}
			Self::Away => {
				let states = get_away_states(state).await;
				if states.is_empty() {
					// No connections, toggle default
					if let (Err(e), _) = QintState::modify_settings(state, |settings| {
						let mut state = settings.get_default_mute_states();
						state.away = !state.away;
						settings.set_default_mute_states(state);
						Ok(Default::default())
					}) {
						error!(state.logger, "Failed to change default mute state"; "error" => %e);
					}
					return;
				}

				if states.iter().all(|(s, _)| *s) {
					// If all servers are away, remove away
					state
						.send_each_con(states.into_iter().map(|(_, c)| c), |c| {
							Some(c.client_update().set_away(None))
						})
						.await;
				} else {
					// Set the remaining servers as away
					state
						.send_each_con(
							states
								.into_iter()
								.filter_map(|(away, c)| if !away { Some(c) } else { None }),
							|c| Some(c.client_update().set_away(Some(""))),
						)
						.await;
				}
			}
		}
	}
}

#[cfg(windows)]
mod imp {
	use std::sync::{Arc, Mutex};

	use anyhow::Result;
	use livesplit_hotkey::*;
	use tokio::runtime::Handle;

	use super::HotkeyConfig;
	use crate::QintState;

	pub use livesplit_hotkey::KeyCode;

	pub struct Hotkeys {
		hook: Hook,
		registered: Mutex<Vec<KeyCode>>,
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

	impl Hotkeys {
		pub fn new() -> Result<Self> {
			Ok(Self { hook: Hook::new()?, registered: Vec::new().into() })
		}

		pub fn apply_config(&self, state: &Arc<QintState>, config: HotkeyConfig) -> Result<()> {
			let mut reg = self.registered.lock().unwrap();
			for key in &*reg {
				let _ = self.hook.unregister(*key);
			}
			reg.clear();

			for a in config.actions {
				let action = a.action;
				let state = state.clone();
				let handle = Handle::current();
				self.hook.register(a.keycode, move || {
					let state = state.clone();
					handle.spawn(async move {
						action.run(&state).await;
					});
				})?;
				reg.push(a.keycode);
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

	use super::HotkeyConfig;
	use crate::QintState;

	#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
	pub enum KeyCode {}

	#[derive(Debug)]
	pub struct Hotkeys {}

	pub fn _key_list() -> Vec<String> { Vec::new() }

	impl Hotkeys {
		pub fn new() -> Result<Self> { Ok(Self {}) }

		pub fn apply_config(&self, _: &Arc<QintState>, _: HotkeyConfig) -> Result<()> { Ok(()) }
	}
}
