#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use actix::prelude::*;
use actix::{Actor, Addr};
use anyhow::{bail, format_err, Result};
use futures::prelude::*;
use futures::stream::FuturesUnordered;
use messages::MessageP2F;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, error, info, warn, Logger};
use tokio::runtime::Handle;
use uuid::Uuid;

pub mod audio;
pub mod connection;
pub mod db;
pub mod filecache;
pub mod hotkey;
pub mod identities;
pub mod link_previewer;
pub mod messages;
pub mod search;
pub mod secret;

use connection::QintConnection;
use filecache::FileCache;
use link_previewer::LinkPreviewer;
use secret::Secret;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";
const LAUNCH_CONFIG_FILENAME: &str = "config.toml";
// TODO Rename to settings.json
const SETTINGS_FILENAME: &str = "transient.json";
const SEARCH_FILENAME: &str = "search.db";

// The build environment of qint.
git_testament::git_testament!(TESTAMENT);

#[macro_export]
macro_rules! with_log {
	($fut:expr, $logger:expr, $err:expr) => {{
		let logger = $logger;
		$fut.map(move |r| {
			if let Err(e) = r {
				error!(logger, $err; "error" => %e);
			}
		})
	}};
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConnectionId(pub Uuid);

#[derive(Clone, Debug)]
pub struct Args {
	/// The address where the server listens
	pub listen_address: Option<SocketAddr>,
	/// The id of the identity that is used by default
	pub default_identity: Option<u64>,
	/// The path for all the settings files. This makes only senses as a command line argument, it
	/// is ignored in the settings file.
	///
	/// If no value is given, the configuration path depends on the operating system.
	pub config_path: Option<PathBuf>,
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating system.
	pub cache_path: Option<PathBuf>,
	/// The path for plugins.
	///
	/// If no value is given, this is the path of the config file plus `plugins/`.
	pub plugin_path: Option<String>,
	/// Do not capture and play audio.
	// This is used for testing, which cannot initialize SDL.
	// SDL must only be initialized once per process, at the same time, it can only be used from a
	// single thread, which does not work well with parallel tests.
	pub no_audio: bool,
	/// Do not open database to search messages.
	pub no_search: bool,
	/// Do not cache link previews.
	pub no_link_cache: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	pub verbosity: u8,
}

/// The settings in this struct are saved to the settings file.
///
/// Settings in this struct are meant to be save the little convenient things like size of the
/// sidebar, which panes were last visible, the last entered, unsent text from the message field,
/// etc. In general, settings that change often.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Settings(Value);

#[derive(Debug, Default)]
pub struct SettingsUpdate {
	pub hotkeys_changed: bool,
}

/// The settings in this struct are saved to the main settings file.
///
/// All settings here are meant to be edited by hand, e.g. for the case that a user wants to have
/// this settings file read-only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchConfig {
	#[serde(default = "default_listen_address")]
	pub listen_address: SocketAddr,
	#[serde(skip)]
	pub config_path: PathBuf,
	#[serde(default = "default_cache_path")]
	pub cache_path: PathBuf,
	#[serde(default)]
	pub plugin_path: PathBuf,
	#[serde(default)]
	pub default_identity: u64,
	#[serde(default)]
	pub no_audio: bool,
	#[serde(default)]
	pub no_search: bool,
	#[serde(default)]
	pub no_link_cache: bool,
	#[serde(default)]
	pub no_open: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[serde(default)]
	pub verbosity: u8,
}

pub struct QintState {
	pub handle: Handle,
	pub logger: Logger,
	/// The list of all currently existing connections
	pub connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
	pub audio_data: Option<audio::AudioData>,
	pub hotkeys: hotkey::Hotkeys,
	pub launch_config: RwLock<LaunchConfig>,
	pub settings: RwLock<Settings>,
	pub database: Addr<db::DbHandler>,
	pub graphql_schema: Arc<db::graphql::Schema>,
	pub file_cache: Arc<FileCache>,
	pub link_previewer: link_previewer::LinkPreviewer,
	pub secret: Secret,
	pub search: Option<Arc<search::Search>>,
}

/// Triple to represent input and output mute state.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
pub enum MuteState {
	// Not muted
	None,
	// Normal muted
	Muted,
	// Hardware disabled
	Disabled,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MuteStates {
	pub input: MuteState,
	pub output: MuteState,
	// True of away on all servers
	pub away: bool,
}

fn default_listen_address() -> SocketAddr { "127.0.0.1:4422".parse().unwrap() }

fn default_cache_path() -> PathBuf {
	let proj_dirs = match directories_next::ProjectDirs::from("", DIR_ORGANIZATION, DIR_PROJECT) {
		Some(r) => r,
		None => {
			return Default::default();
		}
	};
	proj_dirs.cache_dir().into()
}

impl juniper::Context for QintState {}

pub trait AppToFrontendBridge {
	fn send(&self, msg: &MessageP2F);
	fn close(&self);
}
pub type FrontBridge = Box<dyn AppToFrontendBridge + Send>;

impl QintState {
	pub fn modify_settings<T: FnOnce(&mut Settings) -> Result<SettingsUpdate>>(
		state: &Arc<Self>, f: T,
	) -> (Result<SettingsUpdate>, Result<()>) {
		let mut settings = state.settings.write().unwrap();
		let old_loudness_threshold = settings.get_loudness_threshold();
		let old_global_volume = settings.get_global_volume();
		let (old_capture, old_playback) = settings.get_preferred_audio_device();

		// Reload before changing to prevent overwriting changes from other processes
		if let Err(e) = settings.load(&state.launch_config.read().unwrap().config_path) {
			warn!(state.logger, "Failed to reload settings"; "error" => %e);
		}

		let r = f(&mut *settings);
		let res = settings.save(&state.launch_config.read().unwrap().config_path);
		if let Err(e) = &res {
			error!(state.logger, "Failed to save settings"; "error" => %e);
		}

		// Apply audio changes
		if let Some(ad) = &state.audio_data {
			if let Some(v) = settings.get_loudness_threshold() {
				if Some(v) != old_loudness_threshold {
					state.handle.spawn(with_log!(
						ad.a2ts.send(audio::audio_to_ts::SetLoudnessThresholdMsg(v)),
						state.logger.clone(),
						"Failed to apply loudness threshold"
					));
				}
			}

			if let Some(v) = settings.get_global_volume() {
				if Some(v) != old_global_volume {
					state.handle.spawn(with_log!(
						ad.ts2a.send(audio::ts_to_audio::SetGlobalVolumeMsg(v)),
						state.logger.clone(),
						"Failed to apply global volume"
					));
				}
			}

			let (new_capture, new_playback) = settings.get_preferred_audio_device();
			if old_capture != new_capture {
				state.handle.spawn(with_log!(
					ad.a2ts.send(audio::SetAudioDevice(new_capture)),
					state.logger.clone(),
					"Failed to set new capture device"
				));
			}
			if old_playback != new_playback {
				state.handle.spawn(with_log!(
					ad.ts2a.send(audio::SetAudioDevice(new_playback)),
					state.logger.clone(),
					"Failed to set new playback device"
				));
			}
		}

		if let Ok(changes) = &r {
			if changes.hotkeys_changed {
				if let Ok(hotkeys) = settings.get_hotkeys_config() {
					if let Err(e) = state.hotkeys.apply_config(state, hotkeys) {
						error!(state.logger, "Failed to apply new hotkeys"; "error" => %e);
					}
				}
			}
		}
		(r, res)
	}

	/// Run a function for every connected connection and send a packet.
	pub async fn send_each_con<
		P: tsclientlib::OutCommandExt,
		F: FnOnce(&tsclientlib::data::Connection) -> Option<P> + Clone + Send + 'static,
	>(
		&self, cons: impl Iterator<Item = Addr<QintConnection>>, f: F,
	) {
		let fut: FuturesUnordered<_> = cons
			.map(|c| {
				let logger = self.logger.clone();
				let f = f.clone();
				async move {
					let logger2 = logger.clone();
					if let Err(e) = c
						.send(connection::RunOnConMsg(move |c| {
							if let Some(con) = c.get_mut_connection() {
								if let Ok(book) = con.get_state() {
									if let Some(p) = f(book) {
										if let Err(e) = p.send(con) {
											warn!(logger2, "Failed to send message action";
												"error" => %e);
										}
									}
								}
							}
						}))
						.await
					{
						warn!(logger, "Failed to run action"; "error" => %e);
					}
				}
			})
			.collect();
		fut.collect::<()>().await;
	}

	/// Aggregate over all connections.
	///
	/// Ignore connections where sending the message fails.
	pub fn aggregate<
		R: Send + 'static,
		F: FnOnce(&mut QintConnection, Addr<QintConnection>) -> R + Clone + Send + 'static,
	>(
		&self, f: F,
	) -> impl Stream<Item = R> {
		let cons = self.connections.lock().unwrap().values().cloned().collect::<Vec<_>>();
		let fut: FuturesUnordered<_> = cons
			.into_iter()
			.map(|c| {
				let logger = self.logger.clone();
				let f = f.clone();
				let c2 = c.clone();
				async move {
					match c.send(connection::RunOnConMsg(|con| f(con, c2))).await {
						Err(e) => {
							warn!(logger, "Failed to run action"; "error" => %e);
							None
						}
						Ok(r) => Some(r),
					}
				}
			})
			.collect();
		fut.filter_map(|f| future::ready(f))
	}

	pub async fn get_mute_state(&self) -> MuteStates {
		struct OptionMuteStates {
			// Input state for servers, where we can talk (not away and output not muted)
			input_can_talk: Option<MuteState>,
			// Input state for servers, where we cannot talk
			input_cannot_talk: Option<MuteState>,
			output: MuteState,
			away: bool,
		}

		let res = self
			.aggregate(|con, _| {
				con.get_own_client().map(|c| MuteStates {
					input: if !c.input_hardware_enabled {
						MuteState::Disabled
					} else if c.input_muted {
						MuteState::Muted
					} else {
						MuteState::None
					},
					output: if !c.output_hardware_enabled {
						MuteState::Disabled
					} else if c.output_muted {
						MuteState::Muted
					} else {
						MuteState::None
					},
					away: c.away_message.is_some(),
				})
			})
			.fold(None, |res: Option<OptionMuteStates>, state| {
				future::ready(if let (Some(res), Some(state)) = (&res, &state) {
					let can_talk = !state.away && state.output == MuteState::None;
					Some(OptionMuteStates {
						input_can_talk: if can_talk {
							if let Some(i) = res.input_can_talk {
								Some(i.merge(state.input))
							} else {
								Some(state.input)
							}
						} else {
							res.input_can_talk
						},
						input_cannot_talk: if !can_talk {
							if let Some(i) = res.input_cannot_talk {
								Some(i.merge(state.input))
							} else {
								Some(state.input)
							}
						} else {
							res.input_can_talk
						},
						output: res.output.merge(state.output),
						away: res.away && state.away,
					})
				} else if let Some(state) = state {
					let can_talk = !state.away && state.output == MuteState::None;
					Some(OptionMuteStates {
						input_can_talk: if can_talk { Some(state.input) } else { None },
						input_cannot_talk: if !can_talk { Some(state.input) } else { None },
						output: state.output,
						away: state.away,
					})
				} else {
					res
				})
			})
			.await;
		let res = res
			.map(|res| MuteStates {
				input: res.input_can_talk.or(res.input_cannot_talk).unwrap_or(MuteState::None),
				output: res.output,
				away: res.away,
			})
			.unwrap_or_else(|| self.settings.read().unwrap().get_default_mute_states());
		res
	}
}

impl Default for LaunchConfig {
	fn default() -> Self {
		Self {
			listen_address: default_listen_address(),
			config_path: Default::default(),
			cache_path: default_cache_path(),
			plugin_path: Default::default(),
			default_identity: Default::default(),
			no_audio: Default::default(),
			no_search: Default::default(),
			no_link_cache: Default::default(),
			no_open: Default::default(),
			verbosity: Default::default(),
		}
	}
}

impl LaunchConfig {
	fn load(&mut self, config_path: &Path) -> Result<()> {
		let s = fs::read_to_string(&config_path.join(LAUNCH_CONFIG_FILENAME))?;
		*self = toml::from_str(&s)?;
		Ok(())
	}
}

impl Settings {
	pub const KEY_HOTKEYS: &'static str = "hotkeys";

	fn load(&mut self, config_path: &Path) -> Result<()> {
		let s = fs::read_to_string(&config_path.join(SETTINGS_FILENAME))?;
		*self = serde_json::from_str(&s)?;
		Ok(())
	}

	fn save(&self, config_path: &Path) -> Result<()> {
		let data = serde_json::to_string(self)?;
		fs::write(&config_path.join(SETTINGS_FILENAME), data)?;
		Ok(())
	}

	pub fn merge(&mut self, v: &Value) { merge_json(&mut self.0, v); }

	fn get_global_volume(&self) -> Option<f32> {
		Some(self.0.as_object()?.get("audio")?.as_object()?.get("globalVolume")?.as_f64()? as f32)
	}

	fn get_loudness_threshold(&self) -> Option<f64> {
		self.0.as_object()?.get("audio")?.as_object()?.get("loudnessThreshold")?.as_f64()
	}

	fn get_hotkeys_config(&self) -> Result<hotkey::HotkeyConfig> {
		Ok(hotkey::HotkeyConfig::deserialize(
			self.0
				.as_object()
				.ok_or_else(|| format_err!("Settings root is no object"))?
				.get(Settings::KEY_HOTKEYS)
				.ok_or_else(|| format_err!("hotkeys not found in settings"))?
				.clone()
				.into_deserializer(),
		)?)
	}

	fn get_default_mute_states(&self) -> MuteStates {
		self.0
			.as_object()
			.and_then(|p| p.get("audio"))
			.and_then(|p| p.as_object())
			.map(|ui| MuteStates {
				input: if ui.get("defaultInputMuted").and_then(|p| p.as_bool()).unwrap_or_default()
				{
					MuteState::Muted
				} else {
					MuteState::None
				},
				output: if ui
					.get("defaultOutputMuted")
					.and_then(|p| p.as_bool())
					.unwrap_or_default()
				{
					MuteState::Muted
				} else {
					MuteState::None
				},
				away: ui.get("defaultAway").and_then(|p| p.as_bool()).unwrap_or_default(),
			})
			.unwrap_or(MuteStates { input: MuteState::None, output: MuteState::None, away: false })
	}

	fn set_default_mute_states(&mut self, state: MuteStates) {
		let input = if state.input == MuteState::None { None } else { Some(true) };
		let output = if state.output == MuteState::None { None } else { Some(true) };
		let away = if state.away { Some(true) } else { None };
		self.merge(&serde_json::json!({
			"audio": {
				"defaultInputMuted": input,
				"defaultOutputMuted": output,
				"defaultAway": away,
			}
		}));
	}

	/* (Capture, Playback) */
	fn get_preferred_audio_device(&self) -> (Option<String>, Option<String>) {
		self.0
			.as_object()
			.and_then(|p| p.get("audio"))
			.and_then(|p| p.as_object())
			.map(|audio| {
				(
					audio.get("capture").and_then(|p| p.as_str().map(|p| p.to_string())),
					audio.get("playback").and_then(|p| p.as_str().map(|p| p.to_string())),
				)
			})
			.unwrap_or((None, None))
	}
}

impl MuteState {
	pub fn merge(self, other: Self) -> Self {
		if self == Self::None || other == Self::None {
			Self::None
		} else if self == Self::Muted || other == Self::Muted {
			Self::Muted
		} else {
			Self::Disabled
		}
	}
}

fn merge_json(a: &mut Value, b: &Value) {
	match (a, b) {
		(&mut Value::Object(ref mut a), &Value::Object(ref b)) => {
			for (k, v) in b {
				if v.is_null() {
					a.remove(k);
				} else {
					merge_json(a.entry(k).or_insert(Value::Null), &v);
				}
			}
		}
		(a, b) => *a = b.clone(),
	}
}

impl QintState {
	pub fn new(logger: Logger, args: Args) -> Result<Arc<Self>> {
		#[cfg(debug_assertions)]
		let profile = "Debug";
		#[cfg(not(debug_assertions))]
		let profile = "Release";

		info!(logger, "qint";
			"version" => git_testament::render_testament!(TESTAMENT),
			"profile" => profile,
		);

		let handle = Handle::current().clone(); // could also get as param

		let config_path: PathBuf = if let Some(p) = args.config_path {
			p
		} else {
			let proj_dirs =
				match directories_next::ProjectDirs::from("", DIR_ORGANIZATION, DIR_PROJECT) {
					Some(r) => r,
					None => bail!("Failed to get project directory"),
				};
			proj_dirs.config_dir().into()
		};

		// Load settings
		let mut launch_config = LaunchConfig::default();
		if let Err(e) = launch_config.load(&config_path) {
			debug!(logger, "Failed to read launch config, using defaults"; "error" => %e);
			// Create settings directory
			fs::create_dir_all(&config_path)?;
		}

		let settings = {
			let mut set = Settings::default();
			if let Err(e) = set.load(&config_path) {
				info!(logger, "Failed to read settings, using defaults"; "error" => %e);
			}
			set
		};

		// Load secret key
		let key_path = config_path.join("secret.key");
		let secret = match fs::read(&key_path) {
			Ok(r) => Secret::from_slice(&r)?,
			Err(e) => {
				warn!(logger, "Failed to read secret key, all your current \
					identities cannot be used anymore, creating new secret";
					"error" => %e);

				let secret = Secret::new();
				fs::write(&key_path, &secret.0)?;

				secret
			}
		};

		launch_config.config_path = config_path;
		// Override settings with args
		if let Some(a) = args.cache_path {
			launch_config.cache_path = a;
		}
		if let Some(a) = args.plugin_path {
			launch_config.plugin_path = a.into();
		}
		if let Some(a) = args.listen_address {
			launch_config.listen_address = a;
		}
		if let Some(a) = args.default_identity {
			launch_config.default_identity = a;
		}
		if args.no_audio {
			launch_config.no_audio = true;
		}
		if args.no_search {
			launch_config.no_search = true;
		}
		if args.no_link_cache {
			launch_config.no_link_cache = true;
		}
		if args.verbosity > launch_config.verbosity {
			launch_config.verbosity = args.verbosity;
		}

		if launch_config.plugin_path.to_str() == Some("") {
			launch_config.plugin_path = launch_config.config_path.join("plugins");
		}

		let file_cache = Arc::new(FileCache::new(logger.clone(), launch_config.cache_path.clone()));

		// Open search database
		let (search, search_is_new) = if launch_config.no_search {
			(None, false)
		} else {
			let (s, new) = search::Search::new(
				logger.clone(),
				&launch_config.cache_path.join(SEARCH_FILENAME),
			)?;
			(Some(Arc::new(s)), new)
		};

		// Open database
		let database = db::DbHandler::new(
			logger.clone(),
			file_cache.clone(),
			search.clone(),
			&launch_config,
			secret.clone(),
		)?
		.start();

		let connections = Arc::new(Mutex::new(HashMap::new()));

		// Start sound
		let audio_data = if launch_config.no_audio {
			None
		} else {
			Some(audio::start(logger.clone(), connections.clone(), &settings)?)
		};

		// Read hotkeys config
		let hotkey_config = match settings.get_hotkeys_config() {
			Ok(r) => r,
			Err(e) => {
				debug!(logger, "Failed to read hotkey config, ignoring"; "error" => %e);
				hotkey::HotkeyConfig::default()
			}
		};
		let hotkeys = hotkey::Hotkeys::new()?;

		if let Some(threshold) = settings.get_loudness_threshold() {
			if let Some(ad) = &audio_data {
				handle.spawn(with_log!(
					ad.a2ts.send(audio::audio_to_ts::SetLoudnessThresholdMsg(threshold)),
					logger.clone(),
					"Failed to apply loudness threshold"
				));
			}
		}

		if let Some(volume) = settings.get_global_volume() {
			if let Some(ad) = &audio_data {
				handle.spawn(with_log!(
					ad.ts2a.send(audio::ts_to_audio::SetGlobalVolumeMsg(volume)),
					logger.clone(),
					"Failed to apply global volume"
				));
			}
		}

		let graphql_schema = db::graphql::create_schema();
		let link_previewer = LinkPreviewer::new(
			logger.clone(),
			if launch_config.no_link_cache { Some(launch_config.cache_path.clone()) } else { None },
		);

		let state = Arc::new(QintState {
			handle,
			logger,
			connections,
			audio_data,
			hotkeys,
			launch_config: RwLock::new(launch_config),
			settings: RwLock::new(settings),
			database,
			graphql_schema,
			file_cache,
			link_previewer,
			secret,
			search,
		});

		state.hotkeys.apply_config(&state, hotkey_config)?;

		if search_is_new {
			search::Search::start_setup(&state);
		}

		Ok(state)
	}
}
