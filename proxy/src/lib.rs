#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};

use actix::prelude::*;
use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::dev::{Service, ServiceResponse};
use actix_web::middleware::Condition;
use actix_web::web::Bytes;
use actix_web::web::{Data, Query};
use actix_web::*;

use actix_web_actors::ws;
use anyhow::{bail, format_err, Result};
use db::{
	models::UpdateIdentity, DeleteIdentityMsg, FindIdentity, GenrateNewIdentityMsg,
	GetIdentitiesMsg, UpdateIdentityMsg,
};
use futures::prelude::*;
use futures::stream::{FuturesUnordered, Peekable};
use http::{header::CACHE_CONTROL, header::ETAG, HeaderValue};
use identities::import_ts_identities_from_string;
use messages::ResultDetails;
use messages::{MessageF2P, MessageP2F};
use rand::Rng;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, error, info, warn, Logger};
use tokio::runtime::Runtime;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::time::{self, Duration};
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::ChannelId;
use tsclientlib::Error as TsError;
use tsproto_types::crypto::EccKeyPubP256;
use uuid::Uuid;

mod audio;
mod db;
mod filecache;
mod hotkey;
mod identities;
mod link_previewer;
mod loudness_ws;
mod markdown_ws;
pub mod messages;
mod search;
mod secret;
mod websocket;

use filecache::FileCache;
use link_previewer::LinkPreviewer;
use loudness_ws::LoudnessService;
use markdown_ws::MarkdownService;
use secret::Secret;
use websocket::CustomWsMsg;
use websocket::Ws;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";
const LAUNCH_CONFIG_FILENAME: &str = "config.toml";
// TODO Rename to settings.json
const SETTINGS_FILENAME: &str = "transient.json";
const SEARCH_FILENAME: &str = "search.db";

// The build environment of qint.
git_testament::git_testament!(TESTAMENT);

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
struct Settings(Value);

#[derive(Debug, Default)]
struct SettingsUpdate {
	hotkeys_changed: bool,
}

/// The settings in this struct are saved to the main settings file.
///
/// All settings here are meant to be edited by hand, e.g. for the case that a user wants to have
/// this settings file read-only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LaunchConfig {
	#[serde(default = "default_listen_address")]
	listen_address: SocketAddr,
	#[serde(skip)]
	config_path: PathBuf,
	#[serde(default = "default_cache_path")]
	cache_path: PathBuf,
	#[serde(default)]
	plugin_path: PathBuf,
	#[serde(default)]
	default_identity: u64,
	#[serde(default)]
	no_audio: bool,
	#[serde(default)]
	no_search: bool,
	#[serde(default)]
	no_link_cache: bool,
	#[serde(default)]
	no_open: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[serde(default)]
	verbosity: u8,
}

pub struct QintState {
	logger: Logger,
	/// The list of all currently existing connections
	pub connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	audio_data: Option<audio::AudioData>,
	hotkeys: hotkey::Hotkeys,
	launch_config: RwLock<LaunchConfig>,
	settings: RwLock<Settings>,
	database: Addr<db::DbHandler>,
	graphql_schema: Arc<db::graphql::Schema>,
	file_cache: Arc<FileCache>,
	link_previewer: link_previewer::LinkPreviewer,
	secret: Secret,
	search: Option<Arc<search::Search>>,
	/// Authentication token, this needs to be set in the qint-auth cookie.
	token: String,
}

#[derive(Clone)] // !!!! TODO ONLY FOR DEBUGGING
pub struct QintCore {
	pub state: Arc<QintState>,
}

impl Actor for QintCore {
	type Context = Context<Self>;
}
pub type FrontBridge = Box<dyn AppToFrontendBridge + Send>;
pub struct CreateWs {
	pub id: ConnectionId,
	pub sender: FrontBridge,
}
pub struct DispatchWsMsg {
	pub id: ConnectionId,
	pub msg: MessageF2P,
}

impl Message for CreateWs {
	type Result = ();
}
impl Message for DispatchWsMsg {
	type Result = ();
}
impl Handler<CreateWs> for QintCore {
	type Result = ();
	fn handle(&mut self, msg: CreateWs, _: &mut Self::Context) -> Self::Result {
		let CreateWs { id, sender } = msg;

		let mut cons = self.state.connections.lock().unwrap();
		// Check that the id does not exist
		if cons.contains_key(&id) || id.0.is_nil() {
			// TODO
			println!("uuid fuk up");
			return;
		}

		let ws = Ws::new(
			self.state.logger.clone(),
			self.state.clone(),
			WsOptions { format: WsFormat::Json },
			id.clone(),
			sender,
		);
		let addr = ws.start();
		cons.insert(id, addr);
	}
}
impl Handler<DispatchWsMsg> for QintCore {
	type Result = ();
	fn handle(&mut self, msg: DispatchWsMsg, _: &mut Self::Context) -> Self::Result {
		let DispatchWsMsg { id, msg } = msg;

		println!("Sending {:?} {:?}", id, msg);
		let con = {
			match self.state.connections.lock().unwrap().get(&id) {
				Some(con) => con.clone(),
				None => {
					println!("No con for msg found {:?}", id);
					warn!(self.state.logger, "No con for msg found"; "error" => ?id);
					return;
				}
			}
		};

		actix::spawn(con.send(CustomWsMsg(msg)).map(
			move |r| {
			},
		));
		println!("Sent !!");
	}
}

#[derive(Clone, Copy, Debug, Deserialize)]
enum WsFormat {
	Msgpack,
	Json,
}

#[derive(Clone, Debug, Deserialize)]
pub struct WsOptions {
	format: WsFormat,
}

/// Triple to represent input and output mute state.
#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, Deserialize, Serialize)]
enum MuteState {
	// Not muted
	None,
	// Normal muted
	Muted,
	// Hardware disabled
	Disabled,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct MuteStates {
	input: MuteState,
	output: MuteState,
	// True of away on all servers
	away: bool,
}

fn default_listen_address() -> SocketAddr {
	"127.0.0.1:4422".parse().unwrap()
}

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
}

impl QintState {
	fn modify_settings<T: FnOnce(&mut Settings) -> Result<SettingsUpdate>>(
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
					let logger = state.logger.clone();
					actix::spawn(ad.a2ts.send(audio::audio_to_ts::SetLoudnessThresholdMsg(v)).map(
						move |r| {
							if let Err(e) = r {
								error!(logger, "Failed to apply loudness threshold";
										"error" => %e);
							}
						},
					));
				}
			}

			if let Some(v) = settings.get_global_volume() {
				if Some(v) != old_global_volume {
					let logger = state.logger.clone();
					actix::spawn(ad.ts2a.send(audio::ts_to_audio::SetGlobalVolumeMsg(v)).map(
						move |r| {
							if let Err(e) = r {
								error!(logger, "Failed to apply global volume"; "error" => %e);
							}
						},
					));
				}
			}

			let (new_capture, new_playback) = settings.get_preferred_audio_device();
			if old_capture != new_capture {
				let logger = state.logger.clone();
				actix::spawn(ad.a2ts.send(audio::SetAudioDevice(new_capture)).map(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to apply global volume"; "error" => %e);
					}
				}));
			}
			if old_playback != new_playback {
				let logger = state.logger.clone();
				actix::spawn(ad.ts2a.send(audio::SetAudioDevice(new_playback)).map(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to apply global volume"; "error" => %e);
					}
				}));
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
	async fn send_each_con<
		P: tsclientlib::OutCommandExt,
		F: FnOnce(&tsclientlib::data::Connection) -> Option<P> + Clone + Send + 'static,
	>(
		&self, cons: impl Iterator<Item = Addr<Ws>>, f: F,
	) {
		let fut: FuturesUnordered<_> = cons
			.map(|c| {
				let logger = self.logger.clone();
				let f = f.clone();
				async move {
					let logger2 = logger.clone();
					if let Err(e) = c
						.send(websocket::RunOnConMsg(move |c| {
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
	fn aggregate<R: Send + 'static, F: FnOnce(&mut Ws, Addr<Ws>) -> R + Clone + Send + 'static>(
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
					match c.send(websocket::RunOnConMsg(|con| f(con, c2))).await {
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
	const KEY_HOTKEYS: &'static str = "hotkeys";

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

	fn merge(&mut self, v: &Value) {
		merge_json(&mut self.0, v);
	}

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
	fn merge(self, other: Self) -> Self {
		if self == Self::None || other == Self::None {
			Self::None
		} else if self == Self::Muted || other == Self::Muted {
			Self::Muted
		} else {
			Self::Disabled
		}
	}
}

async fn guess_content_type<
	E: Into<Error> + 'static,
	S: Stream<Item = Result<Bytes, E>> + Unpin + 'static,
>(
	stream: S,
) -> (Peekable<S>, HttpResponseBuilder) {
	let mut stream = stream.peekable();
	let mut response = HttpResponse::Ok();
	if let Some(Ok(r)) = Pin::new(&mut stream).peek().await {
		// https://en.wikipedia.org/wiki/List_of_file_signatures
		if r.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
			response.content_type("image/png");
		} else if r.starts_with(&[0xFF, 0xD8, 0xFF, 0xDB])
			|| r.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0])
			|| r.starts_with(&[0xFF, 0xD8, 0xFF, 0xEE])
		{
			response.content_type("image/jpeg");
		} else if r.windows(3).any(|w| w == b"svg") {
			response.content_type("image/svg+xml");
		} else if r.starts_with(&[0x42, 0x4D]) {
			response.content_type("image/bmp");
		} else if r.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x37, 0x61])
			|| r.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x39, 0x61])
		{
			response.content_type("image/gif");
		} else if r
			.starts_with(&[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6F, 0x6D])
		{
			response.content_type("video/mp4");
		}
	}
	(stream, response)
}

// #[get("/con/{id}/ws")]
// async fn create_ws(
// 	state: web::Data<Arc<QintState>>, uuid: web::Path<Uuid>, options: web::Query<WsOptions>,
// 	req: HttpRequest, stream: web::Payload,
// ) -> impl Responder {
// 	let id = ConnectionId(*uuid);

// 	// Check that the id does not exist
// 	let mut cons = state.connections.lock().unwrap();
// 	if cons.contains_key(&id) || uuid.is_nil() {
// 		return Either::Left(
// 			HttpResponse::PreconditionFailed()
// 				.body("Connection id is already occupied".to_string()),
// 		);
// 	}

// 	let ws_con = Ws::new(state.logger.clone(), (**state).clone(), options.0, id);
// 	match ws::start_with_addr(ws_con, &req, stream) {
// 		Err(e) => {
// 			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
// 			Either::Left(HttpResponse::InternalServerError().body("Failed to start connection"))
// 		}
// 		Ok((addr, ws)) => {
// 			cons.insert(id, addr);
// 			Either::Right(ws)
// 		}
// 	}
// }

#[post("/hotkey")]
async fn run_hotkey(
	state: web::Data<Arc<QintState>>, action: web::Json<hotkey::Action>,
) -> impl Responder {
	action.run(&state).await;
	HttpResponse::Ok()
}

#[post("/audio/reset")]
async fn audio_reset(state: web::Data<Arc<QintState>>) -> impl Responder {
	if let Some(ad) = &state.audio_data {
		if ad.a2ts.send(audio::ResetMsg).await.is_err() {
			error!(state.logger, "Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else if ad.ts2a.send(audio::ResetMsg).await.is_err() {
			error!(state.logger, "Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else {
			HttpResponse::Ok()
		}
	} else {
		HttpResponse::Ok()
	}
}

#[get("/audio/device_list")]
async fn audio_device_list(state: web::Data<Arc<QintState>>) -> impl Responder {
	if let Some(ad) = &state.audio_data {
		let captures = ad.a2ts.send(audio::GetAudioDevices()).await.unwrap_or(Vec::new());
		let playbacks = ad.ts2a.send(audio::GetAudioDevices()).await.unwrap_or(Vec::new());
		HttpResponse::Ok().json(&serde_json::json!({
			"capture": captures,
			"playback": playbacks,
		}))
	} else {
		HttpResponse::Ok().json(&serde_json::json!({ "capture": [], "playback": [] }))
	}
}

fn list_plugins_intern(state: &QintState) -> Vec<String> {
	let path = &state.launch_config.read().unwrap().plugin_path;
	let mut res = Vec::new();
	let dir = match path.read_dir() {
		Ok(r) => r,
		Err(e) => {
			warn!(state.logger, "Failed to list plugins"; "dir" => ?path, "error" => %e);
			return Vec::new();
		}
	};
	for p in dir {
		if let Some(p) = p.ok().and_then(|p| p.file_name().into_string().ok()) {
			res.push(p);
		}
	}
	res
}

#[get("/plugins")]
async fn list_plugins(state: web::Data<Arc<QintState>>) -> impl Responder {
	web::Json(list_plugins_intern(&**state))
}

#[get("/plugins/{name}")]
async fn get_plugin(state: web::Data<Arc<QintState>>, name: web::Path<String>) -> impl Responder {
	let path = state.launch_config.read().unwrap().plugin_path.join(&*name);
	fs::read_to_string(path)
		.with_header((http::header::CONTENT_TYPE, "application/javascript; charset=utf-8"))
}

#[put("/plugins/{name}")]
async fn put_plugin(
	state: web::Data<Arc<QintState>>, name: web::Path<String>, body: web::Bytes,
) -> impl Responder {
	if let Ok(s) = std::str::from_utf8(body.as_ref()) {
		let path = state.launch_config.read().unwrap().plugin_path.join(&*name);
		if let Err(e) = fs::write(path, s) {
			HttpResponse::InternalServerError().body(e.to_string())
		} else {
			HttpResponse::Ok().finish()
		}
	} else {
		HttpResponse::BadRequest().body("Invalid text data")
	}
}

#[delete("/plugins/{name}")]
async fn delete_plugin(
	state: web::Data<Arc<QintState>>, name: web::Path<String>,
) -> impl Responder {
	let path = state.launch_config.read().unwrap().plugin_path.join(&*name);
	if let Err(e) = fs::remove_file(path) {
		HttpResponse::InternalServerError().body(e.to_string())
	} else {
		HttpResponse::Ok().finish()
	}
}

#[derive(Deserialize)]
struct GetFileOptions {
	dl: Option<String>,
	return_code: Option<String>,
	#[serde(default)]
	cache: bool,
}

impl ResultDetails {
	fn gone() -> Self {
		Self::from_desc("gone".into())
	}
}

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(Uuid, u64, String)>,
	query_opt: Query<GetFileOptions>,
) -> impl Responder {
	let cons = state.connections.lock().unwrap();
	let (id, channel, path) = path.into_inner();
	let channel = ChannelId(channel);
	let GetFileOptions { dl, return_code, cache } = query_opt.into_inner();
	if let Some(con) = cons.get(&ConnectionId(id)).cloned() {
		drop(cons);

		// Lookup in cache
		let server = match con.send(websocket::GetPublicKeyMsg).await {
			Ok(Ok(r)) => r,
			Ok(Err(e)) => {
				error!(state.logger, "Failed to get server public key"; "error" => %e);
				return HttpResponse::Gone().json(ResultDetails::gone());
			}
			Err(_) => {
				return HttpResponse::Gone().json(ResultDetails::gone());
			}
		};

		let process_answer = |response: &mut HttpResponseBuilder, len: u64| {
			response.no_chunking(len);
			if let Some(filename) = dl.as_ref() {
				response.insert_header((
					"Content-Disposition",
					format!("attachment; filename=\"{}\"", filename),
				));
			}
		};

		if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await
		{
			let (stream, mut response) = guess_content_type(stream).await;
			process_answer(&mut response, len);
			return response.streaming(stream);
		}

		debug!(state.logger, "Downloading file"; "channel" => channel.0, "path" => &path);
		let (len, file_stream, server) = match con
			.send(websocket::DownloadFile { channel, path: path.clone(), return_code })
			.await
		{
			Err(_) => {
				return HttpResponse::Gone().json(ResultDetails::gone());
			}
			Ok(Err(websocket::Error::TsError(TsError::CommandError(err)))) => {
				debug!(state.logger, "File download error"; "error" => %err, "path" => &path);
				return match err.error {
					tsclientlib::TsError::FileInvalidPath => {
						HttpResponse::NotFound().json(Into::<ResultDetails>::into(err))
					}
					tsclientlib::TsError::PermissionsClientInsufficient => {
						HttpResponse::Forbidden().json(Into::<ResultDetails>::into(err))
					}
					_ => HttpResponse::BadRequest().json(Into::<ResultDetails>::into(err)),
				};
			}
			Ok(Err(e)) => {
				error!(state.logger, "File download failed"; "error" => %e, "path" => &path);
				return HttpResponse::InternalServerError()
					.json(ResultDetails::from_desc(format!("Failed to download file: {}", e)));
			}
			Ok(Ok(r)) => r,
		};

		let stream =
			FramedRead::new(file_stream, BytesCodec::new()).map(|r| r.map(web::BytesMut::freeze));
		let (stream, mut response) = guess_content_type(stream).await;
		process_answer(&mut response, len);

		// Cache for offline usage if smaller than 5 MiB
		if cache && len < 5 * 1024 * 1024 {
			let stream = state.file_cache.cache_file(&server, channel, &path, stream).await;
			response.streaming(stream)
		} else {
			response.streaming(stream)
		}
	} else {
		HttpResponse::Gone().json(ResultDetails::gone())
	}
}

#[derive(Deserialize)]
struct PutFileOptions {
	return_code: Option<String>,
}

#[put("/con/{id}/file/{channel}/{path:.*}")]
async fn upload_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(Uuid, u64, String)>, req: web::HttpRequest,
	body: web::Payload, query_opt: Query<PutFileOptions>,
) -> impl Responder {
	let (id, channel, path) = path.into_inner();
	let channel = ChannelId(channel);
	let cons = state.connections.lock().unwrap();
	if let Some(con) = cons.get(&ConnectionId(id)).cloned() {
		drop(cons);

		debug!(state.logger, "Uploading file"; "channel" => channel.0, "path" => &path);
		let size = if let Some(r) = req.headers().get(http::header::CONTENT_LENGTH) {
			match r.to_str() {
				Err(e) => {
					warn!(state.logger, "Invalid content length header"; "error" => %e);
					return HttpResponse::BadRequest().body("Invalid content length header");
				}
				Ok(s) => match s.parse() {
					Err(e) => {
						warn!(state.logger, "Invalid content length header value"; "error" => %e);
						return HttpResponse::BadRequest()
							.body("Invalid content length header - not a number");
					}
					Ok(r) => r,
				},
			}
		} else {
			return HttpResponse::BadRequest().body("Content length header is missing");
		};
		let mut file_stream = match con
			.send(websocket::UploadFile {
				channel,
				path: path.clone(),
				channel_password: None,
				size,
				overwrite: true,
				resume: false,
				return_code: query_opt.return_code.clone(),
			})
			.await
		{
			Err(_) => {
				return HttpResponse::Gone().json(ResultDetails::gone());
			}
			Ok(Err(websocket::Error::TsError(TsError::CommandError(err)))) => {
				debug!(state.logger, "File upload error"; "error" => %err, "path" => &path);
				return match err.error {
					tsclientlib::TsError::FileInvalidPath => {
						HttpResponse::NotFound().json(Into::<ResultDetails>::into(err))
					}
					tsclientlib::TsError::PermissionsClientInsufficient => {
						HttpResponse::Forbidden().json(Into::<ResultDetails>::into(err))
					}
					_ => HttpResponse::BadRequest().json(Into::<ResultDetails>::into(err)),
				};
			}
			Ok(Err(e)) => {
				error!(state.logger, "File upload failed"; "error" => %e, "path" => &path);
				return HttpResponse::InternalServerError()
					.json(ResultDetails::from_desc(format!("Failed to upload file: {}", e)));
			}
			Ok(Ok(r)) => r,
		};
		// Upload
		let mut body_reader = tokio_util::io::StreamReader::new(body.map_err(|e| {
			std::io::Error::new(std::io::ErrorKind::Other, format!("Payload error {}", e))
		}));
		if let Err(e) = tokio::io::copy(&mut body_reader, &mut file_stream).await {
			warn!(state.logger, "File upload aborted"; "error" => %e);
			return HttpResponse::BadGateway()
				.json(ResultDetails::from_desc(format!("Upload failed: {}", e)));
		}
		HttpResponse::Ok().json(ResultDetails::ok())
	} else {
		HttpResponse::Gone().json(ResultDetails::gone())
	}
}

/// Get a cached file by server id, channel and path.
#[get("/filecache/{id}/{channel}/{path:.*}")]
async fn download_cache_file(
	state: web::Data<Arc<QintState>>, path: web::Path<(String, u64, String)>,
) -> impl Responder {
	let (id, channel, path) = path.into_inner();
	let server = match base64::decode_config(&id, base64::URL_SAFE_NO_PAD)
		.map_err(|e| e.into())
		.and_then(|id| EccKeyPubP256::from_short(&id))
	{
		Err(e) => {
			return HttpResponse::BadRequest().body(format!("Not a valid server id: {}", e));
		}
		Ok(id) => id,
	};
	let channel = ChannelId(channel);
	if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await {
		let (stream, mut response) = guess_content_type(stream).await;
		response.no_chunking(len);
		response.streaming(stream)
	} else {
		HttpResponse::NotFound().finish()
	}
}

#[get("/peek_link/{url}")]
async fn get_link_preview(
	state: web::Data<Arc<QintState>>, url: web::Path<String>,
) -> impl Responder {
	HttpResponse::Ok().json(state.link_previewer.decode_and_analyze_link(&url).await)
}

#[get("/loudness")]
async fn loudness_service(
	state: web::Data<Arc<QintState>>, req: HttpRequest, stream: web::Payload,
) -> impl Responder {
	let ws = LoudnessService::new(Arc::clone(&state));
	match ws::start_with_addr(ws, &req, stream) {
		Err(e) => {
			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
			Either::Left(HttpResponse::InternalServerError().body("Failed to start connection"))
		}
		Ok((_, ws)) => Either::Right(ws),
	}
}

#[get("/render_md_service")]
async fn render_md_service(
	state: web::Data<Arc<QintState>>, req: HttpRequest, stream: web::Payload,
) -> impl Responder {
	let ws = MarkdownService::new();
	match ws::start_with_addr(ws, &req, stream) {
		Err(e) => {
			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
			Either::Left(HttpResponse::InternalServerError().body("Failed to start connection"))
		}
		Ok((_, ws)) => Either::Right(ws),
	}
}

// TODO Rename endpoint
#[get("/transient")]
async fn get_setting(state: web::Data<Arc<QintState>>) -> impl Responder {
	let values = state.settings.read().unwrap();
	HttpResponse::Ok().json(serde_json::to_value(&*values).unwrap())
}

#[put("/transient")]
async fn set_setting(state: web::Data<Arc<QintState>>, body: web::Json<Value>) -> impl Responder {
	let (r, res) = QintState::modify_settings(&state.into_inner(), |values| {
		let hotkeys_changed;
		if let Value::Object(o) = &body.0 {
			hotkeys_changed = o.contains_key(Settings::KEY_HOTKEYS);
			values.merge(&body.0);
		} else {
			bail!("body must be an object");
		}
		Ok(SettingsUpdate { hotkeys_changed })
	});

	if let Err(e) = r {
		HttpResponse::BadRequest().body(e.to_string())
	} else if let Err(e) = res {
		HttpResponse::InternalServerError().body(e.to_string())
	} else {
		HttpResponse::Ok().finish()
	}
}

// get /ident/all
// get /ident/by_name/{name}
// put /ident/{name}?nickname?phonetic_name
// post /ident/import [Body:ini_file]

#[get("/ident/all")]
async fn get_ident_all(state: web::Data<Arc<QintState>>) -> impl Responder {
	match state.database.send(GetIdentitiesMsg(FindIdentity::All)).await {
		Ok(Ok(idents)) => HttpResponse::Ok().json(idents),
		Ok(Err(err)) => HttpResponse::BadRequest().body(err.to_string()),
		Err(_) => HttpResponse::Gone().finish(),
	}
}

#[get("/ident/by_id/{id}")]
async fn get_ident_by_id(state: web::Data<Arc<QintState>>, path: web::Path<u64>) -> impl Responder {
	let id = path.into_inner();
	get_single_ident_by(state, FindIdentity::ById(id)).await
}

#[get("/ident/by_name/{name}")]
async fn get_ident_by_name(
	state: web::Data<Arc<QintState>>, path: web::Path<String>,
) -> impl Responder {
	let name = path.into_inner();
	get_single_ident_by(state, FindIdentity::ByName(name)).await
}

async fn get_single_ident_by(state: web::Data<Arc<QintState>>, by: FindIdentity) -> impl Responder {
	match state.database.send(GetIdentitiesMsg(by)).await {
		Ok(Ok(idents)) => {
			if let Some(ident) = idents.first() {
				HttpResponse::Ok().json(ident)
			} else {
				HttpResponse::NotFound().finish()
			}
		}
		Ok(Err(err)) => HttpResponse::BadRequest().body(err.to_string()),
		Err(_) => HttpResponse::Gone().finish(),
	}
}

#[derive(Deserialize)]
struct UpdateIdentityOptions {
	name: Option<String>,
}

#[put("/ident/{id}")]
async fn put_ident(
	state: web::Data<Arc<QintState>>, path: web::Path<u64>, query_opt: Query<UpdateIdentityOptions>,
) -> impl Responder {
	let query = query_opt.into_inner();
	match state
		.database
		.send(UpdateIdentityMsg(
			FindIdentity::ById(path.into_inner()),
			UpdateIdentity { name: query.name, ..Default::default() },
		))
		.await
	{
		Ok(Ok(())) => HttpResponse::Ok().finish(),
		Ok(Err(err)) => HttpResponse::BadRequest().body(err.to_string()),
		Err(_) => HttpResponse::Gone().finish(),
	}
}

#[delete("/ident/{id}")]
async fn delete_ident(state: web::Data<Arc<QintState>>, path: web::Path<u64>) -> impl Responder {
	match state.database.send(DeleteIdentityMsg(FindIdentity::ById(path.into_inner()))).await {
		Ok(Ok(_)) => HttpResponse::Ok().finish(),
		Ok(Err(err)) => HttpResponse::BadRequest().body(err.to_string()),
		Err(_) => HttpResponse::Gone().finish(),
	}
}

#[post("/ident/import")]
async fn post_ident_import(state: web::Data<Arc<QintState>>, body: web::Bytes) -> impl Responder {
	if let Ok(import_str) = std::str::from_utf8(body.as_ref()) {
		match import_ts_identities_from_string(&state, import_str).await {
			Ok(_) => HttpResponse::Ok().finish(),
			Err(e) => HttpResponse::BadRequest().body(e.to_string()),
		}
	} else {
		HttpResponse::BadRequest().body("Invalid text data")
	}
}

#[post("/ident/new")]
async fn post_ident_new(state: web::Data<Arc<QintState>>) -> impl Responder {
	match state.database.send(GenrateNewIdentityMsg()).await {
		Ok(Ok(ident)) => HttpResponse::Ok().json(ident),
		Ok(Err(err)) => HttpResponse::BadRequest().body(err.to_string()),
		Err(_) => HttpResponse::Gone().finish(),
	}
}

#[get("/mutestate")]
async fn get_mute_state(state: web::Data<Arc<QintState>>) -> impl Responder {
	struct OptionMuteStates {
		// Input state for servers, where we can talk (not away and output not muted)
		input_can_talk: Option<MuteState>,
		// Input state for servers, where we cannot talk
		input_cannot_talk: Option<MuteState>,
		output: MuteState,
		away: bool,
	}

	let res = state
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
		.unwrap_or_else(|| state.settings.read().unwrap().get_default_mute_states());
	HttpResponse::Ok().json(res)
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

/// Check the authentication token.
///
/// Returns an http response if this request is handled by an error or redirect.
/// If the result is `None`, the token is ok.
fn check_authentication(token: &str, req: &actix_web::dev::ServiceRequest) -> Option<HttpResponse> {
	#[derive(Deserialize)]
	pub struct TokenQuery {
		token: String,
	}

	if req.path() == "/" {
		if let Ok(Query(TokenQuery { token })) = Query::from_query(req.query_string()) {
			// Redirect to / and set cookie with token
			return Some(
				HttpResponse::SeeOther()
					.append_header((http::header::LOCATION, "/"))
					.cookie(cookie::Cookie::build("qint-auth", token).http_only(true).finish())
					.finish(),
			);
		}
	}

	// Check auth cookie
	if let Some(cookie) = req.cookie("qint-auth") {
		if cookie.value() == token {
			None
		} else {
			Some(HttpResponse::Forbidden().body(
				"Authentication token is wrong, please get a valid authentication token from the \
				 qint proxy",
			))
		}
	} else {
		Some(HttpResponse::Forbidden().body(
			"Authentication token is missing, please get a valid authentication token from the \
			 qint proxy",
		))
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
			let logger = logger.clone();
			if let Some(ad) = &audio_data {
				actix::spawn(
					ad.a2ts.send(audio::audio_to_ts::SetLoudnessThresholdMsg(threshold)).map(
						move |r| {
							if let Err(e) = r {
								error!(logger, "Failed to apply loudness threshold"; "error" => %e);
							}
						},
					),
				);
			}
		}

		if let Some(volume) = settings.get_global_volume() {
			let logger = logger.clone();
			if let Some(ad) = &audio_data {
				actix::spawn(ad.ts2a.send(audio::ts_to_audio::SetGlobalVolumeMsg(volume)).map(
					move |r| {
						if let Err(e) = r {
							error!(logger, "Failed to apply global volume"; "error" => %e);
						}
					},
				));
			}
		}

		let graphql_schema = db::graphql::create_schema();
		let link_previewer = LinkPreviewer::new(
			logger.clone(),
			if launch_config.no_link_cache { Some(launch_config.cache_path.clone()) } else { None },
		);
		let mut rng = rand::thread_rng();
		let token = format!("{:0x}{:0x}", rng.gen::<u64>(), rng.gen::<u64>());

		let state = Arc::new(QintState {
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
			token,
		});

		state.hotkeys.apply_config(&state, hotkey_config)?;

		if search_is_new {
			search::Search::start_setup(&state);
		}

		Ok(state)
	}
}
