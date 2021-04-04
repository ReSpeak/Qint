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

use actix::{Actor, Addr};
use actix_cors::Cors;
use actix_files::Files;
use actix_web::middleware::Condition;
use actix_web::web::Bytes;
use actix_web::*;
use actix_web::{
	dev::{HttpResponseBuilder, Service},
	web::Query,
};
use actix_web_actors::ws;
use anyhow::{bail, format_err, Result};
use futures::prelude::*;
use futures::stream::Peekable;
use http::{header::CACHE_CONTROL, header::ETAG, HeaderValue};
use messages::ResultDetails;
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, error, info, warn, Logger};
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
mod link_previewer;
mod loudness_ws;
mod markdown_ws;
mod messages;
mod search;
mod secret;
mod websocket;

use filecache::FileCache;
use link_previewer::LinkPreviewer;
use loudness_ws::LoudnessService;
use markdown_ws::MarkdownService;
use secret::Secret;
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

pub struct State {
	logger: Logger,
	/// The list of all currently existing connections
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
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
}

#[derive(Debug, Eq, PartialEq, Hash, Copy, Clone, serde::Serialize, serde::Deserialize)]
pub enum Tristate {
	True,
	False,
	Toggle,
}

#[derive(Clone, Copy, Debug, Deserialize)]
enum WsFormat {
	Msgpack,
	Json,
}

#[derive(Clone, Debug, Deserialize)]
struct WsOptions {
	format: WsFormat,
}

pub struct App(Arc<State>);

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

impl juniper::Context for State {}

impl State {
	fn modify_settings<T: FnOnce(&mut Settings) -> Result<SettingsUpdate>>(
		state: &Arc<Self>, f: T,
	) -> (Result<SettingsUpdate>, Result<()>) {
		let mut settings = state.settings.write().unwrap();
		let old_loudness_threshold = settings.get_loudness_threshold();
		let old_global_volume = settings.get_global_volume();

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
		if let Some(v) = settings.get_loudness_threshold() {
			if Some(v) != old_loudness_threshold {
				let logger = state.logger.clone();
				if let Some(ad) = &state.audio_data {
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
		}

		if let Some(v) = settings.get_global_volume() {
			if Some(v) != old_global_volume {
				let logger = state.logger.clone();
				if let Some(ad) = &state.audio_data {
					actix::spawn(ad.ts2a.send(audio::ts_to_audio::SetGlobalVolumeMsg(v)).map(
						move |r| {
							if let Err(e) = r {
								error!(logger, "Failed to apply global volume"; "error" => %e);
							}
						},
					));
				}
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
}

impl Tristate {
	pub fn get_value(&self, old: bool) -> bool {
		match self {
			Tristate::True => true,
			Tristate::False => false,
			Tristate::Toggle => !old,
		}
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

	fn merge(&mut self, v: &Value) { merge_json(&mut self.0, v); }

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

#[get("/con/{id}/ws")]
async fn create_ws(
	state: web::Data<Arc<State>>, uuid: web::Path<Uuid>, options: web::Query<WsOptions>,
	req: HttpRequest, stream: web::Payload,
) -> impl Responder {
	let id = ConnectionId(*uuid);

	// Check that the id does not exist
	let mut cons = state.connections.lock().unwrap();
	if cons.contains_key(&id) || uuid.is_nil() {
		return Either::Left(
			HttpResponse::PreconditionFailed()
				.body("Connection id is already occupied".to_string()),
		);
	}

	let ws_con = Ws::new(state.logger.clone(), (**state).clone(), options.0, id);
	match ws::start_with_addr(ws_con, &req, stream) {
		Err(e) => {
			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
			Either::Left(HttpResponse::InternalServerError().body("Failed to start connection"))
		}
		Ok((addr, ws)) => {
			cons.insert(id, addr);
			Either::Right(ws)
		}
	}
}

#[post("/hotkey")]
async fn run_hotkey(
	state: web::Data<Arc<State>>, action: web::Json<hotkey::Action>,
) -> impl Responder {
	action.run(&state).await;
	HttpResponse::Ok()
}

#[post("/audio/reset")]
async fn audio_reset(state: web::Data<Arc<State>>) -> impl Responder {
	if let Some(ad) = &state.audio_data {
		if ad.a2ts.send(audio::audio_to_ts::ResetMsg).await.is_err() {
			error!(state.logger, "Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else if ad.ts2a.send(audio::ts_to_audio::ResetMsg).await.is_err() {
			error!(state.logger, "Failed to reset audio pipeline");
			HttpResponse::InternalServerError()
		} else {
			HttpResponse::Ok()
		}
	} else {
		HttpResponse::Ok()
	}
}

fn list_plugins_intern(state: &State) -> Vec<String> {
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
async fn list_plugins(state: web::Data<Arc<State>>) -> impl Responder {
	web::Json(list_plugins_intern(&**state))
}

#[get("/plugins/{name}")]
async fn get_plugin(state: web::Data<Arc<State>>, name: web::Path<String>) -> impl Responder {
	let path = state.launch_config.read().unwrap().plugin_path.join(&*name);
	fs::read_to_string(path)
		.with_header((http::header::CONTENT_TYPE, "application/javascript; charset=utf-8"))
}

#[derive(Deserialize)]
struct GetFileOptions {
	dl: Option<String>,
	return_code: Option<String>,
	#[serde(default)]
	cache: bool,
}

impl ResultDetails {
	fn gone() -> Self { Self::from_desc("gone".into()) }
}

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<Arc<State>>, path: web::Path<(Uuid, u64, String)>,
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
		if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await
		{
			let (stream, mut response) = guess_content_type(stream).await;
			response.no_chunking(len);
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
		response.no_chunking(len);
		if let Some(filename) = dl.as_ref() {
			response.insert_header((
				"Content-Disposition",
				format!("attachment; filename=\"{}\"", filename),
			));
		}

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
	state: web::Data<Arc<State>>, path: web::Path<(Uuid, u64, String)>,
	req: web::HttpRequest, body: web::Payload, query_opt: Query<PutFileOptions>,
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
	state: web::Data<Arc<State>>, path: web::Path<(String, u64, String)>,
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
async fn get_link_preview(state: web::Data<Arc<State>>, url: web::Path<String>) -> impl Responder {
	HttpResponse::Ok().json(state.link_previewer.decode_and_analyze_link(&url).await)
}

#[get("/loudness")]
async fn loudness_service(
	state: web::Data<Arc<State>>, req: HttpRequest, stream: web::Payload,
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
	state: web::Data<Arc<State>>, req: HttpRequest, stream: web::Payload,
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
async fn get_setting(state: web::Data<Arc<State>>) -> impl Responder {
	let values = state.settings.read().unwrap();
	HttpResponse::Ok().json(serde_json::to_value(&*values).unwrap())
}

#[put("/transient")]
async fn set_setting(state: web::Data<Arc<State>>, body: web::Json<Value>) -> impl Responder {
	let (r, res) = State::modify_settings(&state.into_inner(), |values| {
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

impl App {
	pub async fn new(logger: Logger, args: Args) -> Result<Self> {
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
			Some(audio::start(
				logger.clone(),
				connections.clone(),
				settings.get_global_volume().unwrap_or(1.0),
			)?)
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

		let state = Arc::new(State {
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

		Ok(Self(state))
	}

	pub async fn serve(self) -> Result<()> {
		let frontend_path = std::option_env!("FRONTEND_PATH").unwrap_or("../frontend/build/");
		let is_production = std::option_env!("FRONTEND_PATH").is_some();
		info!(self.0.logger, "Serving frontend"; "path" => frontend_path);
		let state2 = self.0.clone();
		let addr = self.get_listen_address();

		HttpServer::new(move || {
			let state = state2.clone();
			actix_web::App::new()
				//.wrap(middleware::Logger::default())
				// Return error messages
				.app_data(web::JsonConfig::default().error_handler(|err, _| {
					let err_string = err.to_string();
					error::InternalError::from_response(
						err, HttpResponse::BadRequest().body(err_string)).into()
				}))
				.wrap(Condition::new(!is_production, Cors::permissive().max_age(3600)))
				.data(state)
				.service(create_ws)
				.service(run_hotkey)
				.service(audio_reset)
				.service(list_plugins)
				.service(get_plugin)
				.service(download_file)
				.service(upload_file)
				.service(download_cache_file)
				.service(get_setting)
				.service(set_setting)
				.service(get_link_preview)
				.service(loudness_service)
				.service(render_md_service)
				.service(db::graphql::db_graphql)
				.service(db::graphql::graphiql)
				.service(Files::new("", frontend_path).index_file("index.html"))
				.wrap_fn(|req, srv| {
					let fut = srv.call(req);
					async {
						let mut res = fut.await?;
						let headers = res.headers_mut();
						if headers.contains_key(ETAG) {
							headers.insert(
								CACHE_CONTROL,
								HeaderValue::from_static("no-cache,must-revalidate"),
							);
						}
						Ok(res)
					}
				})
		})
		.bind(addr)?
		.run()
		.await?;

		// Quit all connections
		info!(self.0.logger, "Closing remaining connections");
		{
			let cons = self.0.connections.lock().unwrap();
			for con in cons.values() {
				actix::spawn(con.send(websocket::DisconnectMsg).map(|_| ()));
			}
		}

		// Wait at max a second and poll
		for _ in 0u8..10 {
			{
				let cons = self.0.connections.lock().unwrap();
				if cons.is_empty() {
					break;
				}
			}
			time::sleep(Duration::from_millis(10)).await;
		}

		Ok(())
	}

	pub fn get_listen_address(&self) -> SocketAddr {
		let settings = self.0.launch_config.read().unwrap();
		settings.listen_address
	}
}

/// Tests need a running TeamSpeak server on localhost. The default channel has to be channel 1,
/// this is used to access messages.
#[cfg(test)]
mod tests {
	use anyhow::format_err;
	use awc::ws;
	use rand::Rng;

	use juniper::http::GraphQLRequest;
	use slog::{o, Drain};
	use tsclientlib::ClientId;

	use super::*;
	use messages::{ConnectOptions, JsMessageTarget, MessageF2P, MessageP2F};

	struct TestProxy {
		logger: Logger,
		port: u16,
	}

	struct Connection {
		socket: actix_codec::Framed<awc::BoxedSocket, ws::Codec>,
	}

	#[derive(Deserialize)]
	struct GraphQLResponse<T> {
		data: T,
	}

	#[derive(Deserialize)]
	struct ClientServerKey {
		/// Public key of the server.
		server: Vec<u8>,
		/// Uid of the own identity.
		client: Vec<u8>,
	}

	impl TestProxy {
		fn new(logger: Logger) -> Self {
			let mut rng = rand::thread_rng();
			Self { logger, port: rng.gen_range(1025..=65535) }
		}

		async fn create_connection(&self) -> Result<Connection> {
			let client = awc::Client::default();
			let id = Uuid::new_v4();
			let url = format!("ws://127.0.0.1:{}/con/{}/ws?format=Json", self.port, id);
			info!(self.logger, "Connecting to proxy"; "url" => &url);
			let (_resp, socket) = client
				.ws(url)
				.connect()
				.await
				.map_err(|e| format_err!("Websocket client error: {:?}", e))?;
			Ok(Connection { socket })
		}

		async fn graphql<T>(&self, request: &GraphQLRequest) -> Result<T>
		where for<'a> T: Deserialize<'a> {
			let client = awc::Client::default();
			let url = format!("http://127.0.0.1:{}/db", self.port);
			debug!(self.logger, "GraphQL request"; "body" => serde_json::to_string(&request).unwrap());
			let mut resp = client
				.post(url)
				.send_json(request)
				.await
				.map_err(|_| format_err!("GraphQL failed"))?;
			if !resp.status().is_success() {
				let body = resp
					.body()
					.await
					.map_err(|e| format_err!("Failed to receive body: {:?}", e))?;
				bail!("GraphQL request failed: {}", String::from_utf8_lossy(body.as_ref()));
			}
			let resp: GraphQLResponse<T> =
				resp.json().await.map_err(|e| format_err!("Failed to decode json: {:?}", e))?;
			Ok(resp.data)
		}

		async fn get_client_server_key(&self) -> Result<ClientServerKey> {
			#[derive(Deserialize)]
			#[serde(rename_all = "camelCase")]
			struct Server {
				public_key: Vec<u8>,
			}
			#[derive(Deserialize)]
			struct Client {
				uid: Vec<u8>,
			}
			#[derive(Deserialize)]
			struct Identity {
				client: Client,
			}
			#[derive(Deserialize)]
			struct Bookmark {
				server: Server,
				identity: Identity,
			}
			#[derive(Deserialize)]
			#[serde(rename_all = "camelCase")]
			struct Query {
				most_recent_bookmark: Bookmark,
			}

			let resp: Query = self
				.graphql(&GraphQLRequest::new(
					"{
					mostRecentBookmark {
						server {
							publicKey
						}
						identity {
							client {
								uid
							}
						}
					}
				}"
					.into(),
					None,
					None,
				))
				.await?;
			Ok(ClientServerKey {
				client: resp.most_recent_bookmark.identity.client.uid,
				server: resp.most_recent_bookmark.server.public_key,
			})
		}

		/// Returns uid and name of the client and messages.
		async fn get_messages(
			&self, server: &[u8], type_s: &str, id: &str,
		) -> Result<Vec<(Vec<u8>, String, String)>> {
			#![allow(non_snake_case)]

			#[derive(Deserialize)]
			struct Client {
				uid: Vec<u8>,
				name: String,
			}
			#[derive(Deserialize)]
			struct Invoker {
				client: Client,
			}
			#[derive(Deserialize)]
			struct Message {
				invoker: Invoker,
				content: String,
			}
			#[derive(Deserialize)]
			struct Chat {
				messages: Vec<Message>,
			}
			#[derive(Deserialize)]
			struct Query {
				chat: Chat,
			}

			let vars = vec![("typ", type_s), ("id", id)];
			let vars = juniper::InputValue::Object({
				let mut vars: Vec<_> = vars
					.into_iter()
					.map(|(k, v)| {
						(
							juniper::parser::Spanning::unlocated(k.to_string()),
							juniper::parser::Spanning::unlocated(juniper::InputValue::scalar(v)),
						)
					})
					.collect();
				vars.push((
					juniper::parser::Spanning::unlocated("server".to_string()),
					juniper::parser::Spanning::unlocated(juniper::InputValue::list(
						server.iter().map(|b| juniper::InputValue::scalar(*b as i32)).collect(),
					)),
				));
				vars
			});
			let resp: Query = self
				.graphql(&GraphQLRequest::new(
					"query ($typ: GMessageTarget!, $server: [Int!]!, $id: ID!) {
					chat(typ: $typ, server: $server, id: $id) {
						messages {
							invoker {
								client {
									uid
									name
								}
							}
							content
						}
					}
				}"
					.into(),
					None,
					Some(vars),
				))
				.await?;
			Ok(resp
				.chat
				.messages
				.into_iter()
				.map(|m| (m.invoker.client.uid, m.invoker.client.name, m.content))
				.collect())
		}

		fn run(&self) -> impl Future<Output = Result<()>> {
			let logger = self.logger.clone();
			let port = self.port;
			async move {
				let dir = tempfile::Builder::new().prefix("qint-proxy").tempdir()?;
				info!(logger, "Using config directory"; "dir" => dir.path().display());
				let args = Args {
					listen_address: Some(format!("127.0.0.1:{}", port).parse().unwrap()),
					default_identity: None,
					config_path: Some(dir.path().join("config")),
					cache_path: Some(dir.path().join("cache")),
					plugin_path: None,
					no_audio: true,
					no_search: false,
					no_link_cache: false,
					verbosity: 1,
				};
				let app = App::new(logger, args).await?;
				app.serve().await?;
				dir.close()?;
				Ok(())
			}
		}

		fn run_log_errors(&self) -> impl Future<Output = ()> {
			let fut = self.run();
			let logger = self.logger.clone();
			async move {
				if let Err(e) = fut.await {
					error!(logger, "Proxy encountered an error"; "error" => %e);
				}
			}
		}
	}

	impl Connection {
		async fn connect(&mut self) -> Result<ClientId> {
			self.send(&MessageF2P::Connect(ConnectOptions {
				address: "localhost".to_string(),
				name: "Test".to_string(),
				..Default::default()
			}))
			.await?;
			loop {
				let msg = self.recv().await?;
				if let MessageP2F::Connected { own_client, .. } = msg {
					return Ok(ClientId(own_client.parse().unwrap()));
				} else if let MessageP2F::Error(e) = msg {
					bail!("Got proxy error: {}", e);
				}
			}
		}

		async fn send(&mut self, msg: &MessageF2P) -> Result<()> {
			println!("Sending message to proxy: {}", serde_json::to_string(msg).unwrap());
			self.socket
				.send(ws::Message::Text(serde_json::to_string(msg)?.into()))
				.await
				.map_err(|e| format_err!("Websocket client protocol error: {:?}", e))?;
			Ok(())
		}

		async fn recv(&mut self) -> Result<MessageP2F> {
			match self.socket.next().await {
				Some(Ok(ws::Frame::Binary(msg))) => Ok(rmp_serde::from_read_ref(msg.as_ref())?),
				Some(Ok(ws::Frame::Text(msg))) => {
					Ok(serde_json::from_str(std::str::from_utf8(&msg)?)?)
				}
				f => bail!("Websocket client received unexpected packet: {:?}", f),
			}
		}
	}

	fn create_logger() -> Logger {
		let decorator = slog_term::PlainDecorator::new(slog_term::TestStdoutWriter);
		let drain = Mutex::new(slog_term::FullFormat::new(decorator).build()).fuse();

		slog::Logger::root(drain, o!())
	}

	/// Check that connecting to a server adds this server to the recent connections and updates
	/// it when reconnecting.
	#[actix_rt::test]
	async fn test_save_server() -> Result<()> {
		let proxy = TestProxy::new(create_logger());
		actix::spawn(proxy.run_log_errors());
		// Wait for server to come up
		time::sleep(Duration::from_millis(100)).await;
		let mut con = proxy.create_connection().await?;
		con.connect().await?;
		// Wait for saving the connection in the database
		time::sleep(Duration::from_millis(100)).await;
		drop(con);

		#[derive(Deserialize)]
		#[serde(rename_all = "camelCase")]
		struct ServerServer {
			#[allow(dead_code)]
			public_key: Vec<u8>,
		}
		#[derive(Deserialize)]
		struct ServerBookmark {
			#[allow(dead_code)]
			server: ServerServer,
		}
		#[derive(Deserialize)]
		struct ServerResponse {
			bookmarks: Vec<ServerBookmark>,
		}

		// Check for the server in the database
		let response: ServerResponse = proxy
			.graphql(&GraphQLRequest::new(
				"{
				bookmarks {
					server {
						publicKey
					}
				}
			}"
				.into(),
				None,
				None,
			))
			.await?;
		assert_eq!(response.bookmarks.len(), 1, "Recent connection not saved in the database");
		Ok(())
	}

	/// Check that getting or sending a message from a client saves the other client and the
	/// message.
	#[actix_rt::test]
	async fn test_save_client() -> Result<()> {
		let logger = create_logger();
		let proxy0 = TestProxy::new(logger.clone());
		actix::spawn(proxy0.run_log_errors());
		let proxy1 = TestProxy::new(logger);
		actix::spawn(proxy1.run_log_errors());
		// Wait for server to come up
		time::sleep(Duration::from_millis(100)).await;
		let mut con0 = proxy0.create_connection().await?;
		con0.connect().await?;
		let mut con1 = proxy1.create_connection().await?;
		let con1_id = con1.connect().await?;

		// con0 sends a message to con1
		let msg = "Hello 1";
		con0.send(&MessageF2P::SendMessage {
			target: JsMessageTarget::Client(con1_id),
			message: msg.to_string(),
			return_code: None,
		})
		.await?;

		// Wait for saving the message in the database
		time::sleep(Duration::from_millis(100)).await;
		drop(con0);
		drop(con1);

		let key0 = proxy0.get_client_server_key().await?;
		let key1 = proxy1.get_client_server_key().await?;

		// Check for the message in the database of con0
		let msgs =
			proxy0.get_messages(&key0.server, "CLIENT", &base64::encode(&key1.client)).await?;
		assert_eq!(msgs.len(), 1, "Message not saved in the database");
		assert_eq!(msgs[0].0, key0.client, "Sender uid is wrong");
		assert_eq!(msgs[0].2, msg, "Message is wrong");
		assert!(msgs[0].1.starts_with("Test"), "Client name has to start with 'Test'");

		// Check for the message in the database of con1
		let msgs =
			proxy1.get_messages(&key0.server, "CLIENT", &base64::encode(&key0.client)).await?;
		assert_eq!(msgs.len(), 1, "Message not saved in the database");
		assert_eq!(msgs[0].0, key0.client, "Sender uid is wrong");
		assert_eq!(msgs[0].2, msg, "Message is wrong");
		assert!(msgs[0].1.starts_with("Test"), "Client name has to start with 'Test'");
		Ok(())
	}
}
