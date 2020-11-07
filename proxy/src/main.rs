// Don't show terminal
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::net::{IpAddr, SocketAddr};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{mpsc, Arc, Mutex, RwLock};

use actix::*;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::{web::Bytes, middleware::Condition};
use actix_web::*;
use actix_web::{
	dev::{HttpResponseBuilder, Service},
	web::Query,
};
use actix_web_actors::ws;
use anyhow::{bail, Result};
use futures::prelude::*;
use futures::stream::Peekable;
use http::{header::CACHE_CONTROL, header::ETAG, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use slog::{debug, error, info, o, warn, Drain, Logger};
use structopt::StructOpt;
use tokio::time::{self, Duration};
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::ChannelId;
use tsclientlib::Error as TsError;
use tsproto_types::crypto::EccKeyPubP256;
use uuid::Uuid;

mod audio;
mod book_events;
mod db;
mod filecache;
mod find_url;
mod markdown;
mod markdown_ws;
mod messages;
mod search;
mod secret;
mod shortcut;
mod site_peek;
mod websocket;

use filecache::FileCache;
use markdown_ws::MarkdownService;
use secret::Secret;
use websocket::Ws;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";
const SETTINGS_FILENAME: &str = "config.toml";
const TRANSIENT_SETTINGS_FILENAME: &str = "transient.toml";

// The build environment of qint.
git_testament::git_testament!(TESTAMENT);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConnectionId(pub Uuid);

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
struct Args {
	/// The address where the server listens
	#[structopt(short = "l", long)]
	listen_address: Option<SocketAddr>,
	/// The id of the identity that is used by default
	#[structopt(short = "i", long)]
	default_identity: Option<u64>,
	/// The path for all the settings files. This makes only senses as a command line argument, it
	/// is ignored in the settings file.
	///
	/// If no value is given, the configuration path depends on the operating system.
	#[structopt(short = "c", long)]
	config_path: Option<PathBuf>,
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating system.
	#[structopt(long)]
	cache_path: Option<PathBuf>,
	/// The path for plugins.
	///
	/// If no value is given, this is the path of the config file plus `plugins/`.
	#[structopt(long)]
	plugin_path: Option<String>,
	/// Do not capture and play audio.
	// This is used for testing, which cannot initialize SDL.
	// SDL must only be initialized once per process, at the same time, it can only be used from a
	// single thread, which does not work well with parallel tests.
	#[structopt(long)]
	no_audio: bool,
	/// Open the frontend in the browser on start.
	#[structopt(long)]
	no_open: bool,
	/// Start in browser instead of in tauri.
	#[structopt(short, long)]
	browser: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[structopt(short = "v", long, parse(from_occurrences))]
	verbosity: u8,
}

/// The settings in this struct are saved to the transient settings file.
///
/// Settings in this struct are meant to be save the little convenient things like size of the
/// sidebar, which panes were last visible, the last entered, unsent text from the message field,
/// etc. In general, settings that change often.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct TransientSettings {
	#[serde(flatten, serialize_with = "toml::ser::tables_last")]
	fields: Map<String, Value>,
}

/// The settings in this struct are saved to the main settings file.
///
/// All settings here are meant to be edited by hand, e.g. for the case that a user wants to have
/// this settings file read-only.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Settings {
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
	no_open: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[serde(default)]
	verbosity: u8,

	shortcuts: shortcut::ShortcutConfig,
}

pub struct State {
	logger: Logger,
	/// The list of all currently existing connections
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	audio_data: Option<audio::AudioData>,
	shortcuts: shortcut::Shortcuts,
	settings: RwLock<Settings>,
	transient_settings: RwLock<TransientSettings>,
	database: Addr<db::DbHandler>,
	graphql_schema: Arc<db::graphql::Schema>,
	file_cache: Arc<FileCache>,
	site_peek_cache: site_peek::SitePeekCache,
	secret: Secret,
	search: Arc<search::Search>,
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

struct App;

fn default_listen_address() -> SocketAddr { "127.0.0.1:4422".parse().unwrap() }

fn default_cache_path() -> PathBuf {
	let proj_dirs = match directories::ProjectDirs::from("", DIR_ORGANIZATION, DIR_PROJECT) {
		Some(r) => r,
		None => {
			return Default::default();
		}
	};
	proj_dirs.cache_dir().into()
}

impl Default for Settings {
	fn default() -> Self {
		Self {
			listen_address: default_listen_address(),
			config_path: Default::default(),
			cache_path: default_cache_path(),
			plugin_path: Default::default(),
			default_identity: Default::default(),
			no_audio: Default::default(),
			no_open: Default::default(),
			verbosity: Default::default(),
			shortcuts: Default::default(),
		}
	}
}

impl juniper::Context for State {}

impl State {
	fn modify_transient_settings<R, T: FnOnce(&mut TransientSettings) -> R>(&self, f: T) -> R {
		let mut settings = self.transient_settings.write().unwrap();
		let r = f(&mut *settings);
		if let Err(e) = settings.save(&self.settings.read().unwrap().config_path) {
			error!(self.logger, "Failed to save transient settings"; "error" => %e);
		}
		r
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

impl TransientSettings {
	fn save(&self, config_path: &Path) -> Result<()> {
		// TODO Could also be msgpack
		let data = toml::to_string(self)?;
		fs::write(&config_path.join(TRANSIENT_SETTINGS_FILENAME), data)?;
		Ok(())
	}

	fn set(&mut self, k: String, v: Value) {
		if let Value::Null = v {
			self.fields.remove(&k);
		} else if let Some(value) = self.fields.get_mut(&k) {
			merge_json(value, &v);
		} else {
			let mut new_obj = Value::Object(Map::<_, _>::new());
			merge_json(&mut new_obj, &v);
			self.fields.insert(k, new_obj);
		}
	}

	// TODO Should be part of the settings, not transient settings
	fn get_loudness_threshold(&self) -> Option<f64> {
		self.fields.get("loudness_threshold").and_then(|v| v.as_f64())
	}

	fn set_loudness_threshold(&mut self, value: Option<f64>) {
		if let Some(value) = value {
			self.fields.insert("loudness_threshold".into(), value.into());
		} else {
			self.fields.remove("loudness_threshold");
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
		}
	}
	(stream, response)
}

#[get("/con/{id}/ws")]
async fn create_ws(
	state: web::Data<Arc<State>>, uuid: web::Path<Uuid>, options: web::Query<WsOptions>,
	req: HttpRequest, stream: web::Payload,
) -> impl Responder
{
	let id = ConnectionId(*uuid);

	// Check that the id does not exist
	let mut cons = state.connections.lock().unwrap();
	if cons.contains_key(&id) || uuid.is_nil() {
		return Either::A(
			HttpResponse::PreconditionFailed()
				.body("Connection id is already occupied".to_string()),
		);
	}

	let ws_con = Ws::new(state.logger.clone(), (**state).clone(), options.0, id, None);
	match ws::start_with_addr(ws_con, &req, stream) {
		Err(e) => {
			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
			Either::A(HttpResponse::InternalServerError().body("Failed to start connection"))
		}
		Ok((addr, ws)) => {
			cons.insert(id, addr);
			Either::B(ws)
		}
	}
}

#[post("/shortcut")]
async fn run_shortcut(
	state: web::Data<Arc<State>>, action: web::Json<shortcut::Action>,
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
	let path = &state.settings.read().unwrap().plugin_path;
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
	let path = state.settings.read().unwrap().plugin_path.join(&*name);
	fs::read_to_string(path)
		.with_header(http::header::CONTENT_TYPE, "application/javascript; charset=utf-8")
}

#[derive(Deserialize)]
struct GetFileOptions {
	dl: Option<String>,
}

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<Arc<State>>, web::Path((id, channel, path)): web::Path<(Uuid, u64, String)>,
	query_opt: Query<GetFileOptions>,
) -> impl Responder
{
	let channel = ChannelId(channel);
	let cons = state.connections.lock().unwrap();
	if let Some(con) = cons.get(&ConnectionId(id)).cloned() {
		drop(cons);

		// Lookup in cache
		let server = match con.send(websocket::GetPublicKeyMsg).await {
			Ok(Ok(r)) => r,
			Ok(Err(e)) => {
				error!(state.logger, "Failed to get server public key"; "error" => %e);
				return HttpResponse::Gone().finish();
			}
			Err(_) => {
				return HttpResponse::Gone().finish();
			}
		};
		if let Some((len, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await
		{
			let (stream, mut response) = guess_content_type(stream).await;
			response.no_chunking(len);
			return response.streaming(stream);
		}

		debug!(state.logger, "Downloading file"; "channel" => channel.0, "path" => &path);
		let (len, file_stream, server) =
			match con.send(websocket::DownloadFile { channel, path: path.clone() }).await {
				Err(_) => {
					return HttpResponse::Gone().finish();
				}
				Ok(Err(e)) => {
					if let Some(TsError::CommandError(err)) = e.downcast_ref::<TsError>() {
						if err.error == tsclientlib::TsError::FileInvalidPath {
							debug!(state.logger, "File not found"; "path" => &path);
							return HttpResponse::NotFound().finish();
						}
					}
					error!(state.logger, "File download failed"; "error" => %e, "path" => &path);
					return HttpResponse::InternalServerError()
						.body(format!("Failed to download file: {}", e));
				}
				Ok(Ok(r)) => r,
			};

		let stream =
			FramedRead::new(file_stream, BytesCodec::new()).map(|r| r.map(web::BytesMut::freeze));
		let (stream, mut response) = guess_content_type(stream).await;
		response.no_chunking(len);
		if let Some(filename) = query_opt.dl.as_ref() {
			response.set_header(
				"Content-Disposition",
				format!("attachment; filename=\"{}\"", filename),
			);
		}

		// Cache icons and avatars for offline usage
		if channel.0 == 0 && (path.starts_with("icon_") || path.starts_with("avatar_")) {
			let stream = state.file_cache.cache_file(&server, channel, &path, stream).await;
			response.streaming(stream)
		} else {
			response.streaming(stream)
		}
	} else {
		HttpResponse::Gone().finish()
	}
}

#[put("/con/{id}/file/{channel}/{path:.*}")]
async fn upload_file(
	state: web::Data<Arc<State>>, web::Path((id, channel, path)): web::Path<(Uuid, u64, String)>,
	req: web::HttpRequest, body: web::Payload,
) -> impl Responder
{
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
			})
			.await
		{
			Err(_) => {
				return HttpResponse::Gone().finish();
			}
			Ok(Err(e)) => {
				error!(state.logger, "File upload failed"; "error" => %e, "path" => &path);
				return HttpResponse::InternalServerError()
					.body(format!("Failed to upload file: {}", e));
			}
			Ok(Ok(r)) => r,
		};
		// Upload
		let mut body_reader = tokio::io::stream_reader(body.map_err(|e| {
			std::io::Error::new(std::io::ErrorKind::Other, format!("Payload error {}", e))
		}));
		if let Err(e) = tokio::io::copy(&mut body_reader, &mut file_stream).await {
			warn!(state.logger, "File upload aborted"; "error" => %e);
			return HttpResponse::BadGateway().body(format!("Upload failed: {}", e));
		}
		HttpResponse::Ok().finish()
	} else {
		HttpResponse::Gone().finish()
	}
}

/// Get a cached file by server id, channel and path.
#[get("/filecache/{id}/{channel}/{path:.*}")]
async fn download_cache_file(
	state: web::Data<Arc<State>>, web::Path((id, channel, path)): web::Path<(String, u64, String)>,
) -> impl Responder {
	let server = match hex::decode(&id) {
		Err(e) => {
			return HttpResponse::BadRequest().body(format!("Not a valid server id: {}", e));
		}
		Ok(id) => EccKeyPubP256::from_short(id),
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
	HttpResponse::Ok().json(state.site_peek_cache.decode_and_analyze_link(&url).await)
}

#[get("/render_md_service")]
async fn render_md_service(
	state: web::Data<Arc<State>>, req: HttpRequest, stream: web::Payload,
) -> impl Responder {
	let ws = MarkdownService::new();
	match ws::start_with_addr(ws, &req, stream) {
		Err(e) => {
			error!(state.logger, "Failed to create websocket actor"; "error" => %e);
			Either::A(HttpResponse::InternalServerError().body("Failed to start connection"))
		}
		Ok((_, ws)) => Either::B(ws),
	}
}

fn get_transient_setting_internal(state: &State, req: &str) -> Option<Value> {
	let transient_values = state.transient_settings.read().unwrap();
	if req == "*" {
		Some(serde_json::to_value(&*transient_values).unwrap())
	} else if let Some(value) = transient_values.fields.get(req) {
		Some(value.clone())
	} else {
		None
	}
}

#[get("/transient/{key}")]
async fn get_transient_setting(
	state: web::Data<Arc<State>>, data: web::Path<String>,
) -> impl Responder {
	if let Some(res) = get_transient_setting_internal(&**state, data.as_str()) {
		HttpResponse::Ok().json(res)
	} else {
		HttpResponse::NotFound().body("Unknown key")
	}
}

fn set_transient_setting_internal(state: &State, req: &str, body: Value) -> Result<()> {
	state.modify_transient_settings(|transient_values| {
		if req == "*" {
			if let Value::Object(obj) = body {
				for (k, v) in obj.into_iter() {
					transient_values.set(k, v);
				}
			} else {
				bail!("*-assign must be an object");
			}
		} else {
			transient_values.set(req.to_string(), body);
		}
		Ok(())
	})
}

#[put("/transient/{key}")]
async fn set_transient_setting(
	state: web::Data<Arc<State>>, data: web::Path<String>, body: web::Json<Value>,
) -> impl Responder {
	if let Err(e) = set_transient_setting_internal(&**state, data.as_str(), body.0) {
		HttpResponse::BadRequest().body(e.to_string())
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
		(a, b) => {
			if b.is_object() {
				let mut new_a = Value::Object(Map::<_, _>::new());
				merge_json(&mut new_a, b);
				*a = new_a;
			} else {
				*a = b.clone();
			}
		}
	}
}

/// Handle http requests made through the tauri interface.
async fn handle_tauri_request(
	state: &Arc<State>, req: messages::TauriHttpRequest,
) -> Result<messages::TauriHttpResponse> {
	use messages::{TauriHttpRequest, TauriHttpResponse};

	match req {
		TauriHttpRequest::RunShortcut(action) => {
			action.run(state).await;
			Ok(TauriHttpResponse::Void())
		}
		TauriHttpRequest::ListPlugins() => {
			Ok(TauriHttpResponse::PluginList(list_plugins_intern(&*state)))
		}
		TauriHttpRequest::GetPlugin(name) => {
			let path = state.settings.read().unwrap().plugin_path.join(&name);
			Ok(TauriHttpResponse::Plugin(fs::read_to_string(path)?))
		}
		TauriHttpRequest::DownloadFile { connection, channel, path } => {
			let _ = (connection, channel, path);
			Ok(TauriHttpResponse::Void())
		}
		TauriHttpRequest::DownloadCacheFile { server, channel, path } => {
			let _ = (server, channel, path);
			Ok(TauriHttpResponse::Void())
		}
		TauriHttpRequest::GetTransientSetting(name) => {
			Ok(TauriHttpResponse::TransientSetting(get_transient_setting_internal(state, &name)))
		}
		TauriHttpRequest::SetTransientSetting(name, content) => {
			set_transient_setting_internal(state, &name, content)?;
			Ok(TauriHttpResponse::Void())
		}
		TauriHttpRequest::Graphql(req) => Ok(TauriHttpResponse::Graphql(serde_json::to_value(
			db::graphql::db_graphql_intern(&*state, &req).await,
		)?)),
	}
}

impl App {
	async fn run(logger: Logger, args: Args) -> Result<()> {
		let _scope_guard = slog_scope::set_global_logger(logger.clone());
		// Ignore errors if a logger has already been set
		let _ = slog_stdlog::init();

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
			let proj_dirs = match directories::ProjectDirs::from("", DIR_ORGANIZATION, DIR_PROJECT)
			{
				Some(r) => r,
				None => bail!("Failed to get project directory"),
			};
			proj_dirs.config_dir().into()
		};

		// Load settings
		let mut settings = match fs::read_to_string(&config_path.join(SETTINGS_FILENAME)) {
			Ok(r) => toml::from_str(&r)?,
			Err(e) => {
				// Only a soft error
				info!(logger, "Failed to read settings, using defaults"; "error" => %e);
				// Create settings directory
				fs::create_dir_all(&config_path)?;

				Settings::default()
			}
		};

		let transient_settings =
			match fs::read_to_string(&config_path.join(TRANSIENT_SETTINGS_FILENAME)) {
				Ok(r) => toml::from_str(&r)?,
				Err(e) => {
					info!(logger, "Failed to read transient settings, using defaults"; "error" => %e);
					TransientSettings::default()
				}
			};

		// Load secret key
		let key_path = config_path.join("secret.key");
		let secret = match fs::read(&key_path) {
			Ok(r) => Secret(r),
			Err(e) => {
				warn!(logger, "Failed to read secret key, all your current \
					identities cannot be used anymore, creating new secret";
					"error" => %e);

				let secret = Secret::new()?;
				fs::write(&key_path, &secret.0)?;

				secret
			}
		};

		settings.config_path = config_path;
		// Override settings with args
		if let Some(a) = args.cache_path {
			settings.cache_path = a;
		}
		if let Some(a) = args.plugin_path {
			settings.plugin_path = a.into();
		}
		if let Some(a) = args.listen_address {
			settings.listen_address = a;
		}
		if let Some(a) = args.default_identity {
			settings.default_identity = a;
		}
		if args.no_audio {
			settings.no_audio = true;
		}
		if args.no_open {
			settings.no_open = true;
		}
		if args.verbosity > settings.verbosity {
			settings.verbosity = args.verbosity;
		}

		if settings.plugin_path.to_str() == Some("") {
			settings.plugin_path = settings.config_path.join("plugins");
		}

		let file_cache = Arc::new(FileCache::new(logger.clone(), settings.cache_path.clone()));

		// Open search database
		let (search, search_is_new) =
			search::Search::new(logger.clone(), &settings.cache_path.join("search.db"))?;
		let search = Arc::new(search);

		// Open database
		let database = db::DbHandler::new(
			logger.clone(),
			file_cache.clone(),
			search.clone(),
			&settings,
			secret.clone(),
		)?
		.start();

		let connections = Arc::new(Mutex::new(HashMap::new()));

		// Start sound
		let audio_data = if settings.no_audio {
			None
		} else {
			Some(audio::start(logger.clone(), connections.clone())?)
		};
		let shortcut_config = settings.shortcuts.clone();
		let shortcuts = shortcut::Shortcuts::new(shortcut_config)?;
		let addr = settings.listen_address;
		let no_open = settings.no_open;

		if let Some(threshold) = transient_settings.get_loudness_threshold() {
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

		let graphql_schema = db::graphql::create_schema();
		let site_peek_cache = Default::default();
		let state = Arc::new(State {
			logger,
			connections,
			audio_data,
			shortcuts,
			settings: RwLock::new(settings),
			transient_settings: RwLock::new(transient_settings),
			database,
			graphql_schema,
			file_cache,
			site_peek_cache,
			secret,
			search,
		});

		state.shortcuts.apply_config(&state)?;

		if search_is_new {
			search::Search::start_setup(&state);
		}

		if !no_open {
			// Open browser
			let port = addr.port();
			let logger = state.logger.clone();
			actix::spawn(async move {
				// Connect to localhost if == 0.0.0.0 or ::
				let url = if addr.ip() == "0.0.0.0".parse::<IpAddr>().unwrap()
					|| addr.ip() == "::".parse::<IpAddr>().unwrap()
				{
					format!("http://localhost:{}", port)
				} else {
					format!("http://{}", addr)
				};
				debug!(logger, "Opening url"; "url" => &url);
				if let Err(e) = open::that(url) {
					error!(logger, "Failed to open frontend in browser"; "error" => %e);
				}
			});
		}

		if !args.browser {
			let state2 = state.clone();
			let state3 = state.clone();
			tauri::AppBuilder::new()
				.setup(move |webview, _source| {
					let webview = webview.as_mut();
					let state = state2.clone();
					tauri::event::listen("websocket", move |msg| {
						let msg = if let Some(msg) = msg {
							msg
						} else {
							error!(state.logger, "No message for websocket event");
							return;
						};
						let msg: messages::TauriWsEventF2P = match serde_json::from_str(&msg) {
							Ok(r) => r,
							Err(e) => {
								error!(state.logger, "Failed to parse websocket event";
									"error" => %e);
								return;
							}
						};
						if msg.connection.is_nil() {
							error!(state.logger, "Nil uuid is not allowed as connection id");
							return;
						}
						let id = ConnectionId(msg.connection);

						let con;
						{
							let mut cons = state.connections.lock().unwrap();
							let entry = cons.entry(id.clone());
							con = entry
								.or_insert_with(|| {
									// Create connection
									let options = WsOptions { format: WsFormat::Json };
									let _con = Ws::new(
										state.logger.clone(),
										state.clone(),
										options,
										id,
										Some(webview.clone()),
									);
									panic!()
									//con.start() TODO
								})
								.clone();
						}
						let logger = state.logger.clone();
						actix::spawn(con.send(websocket::HandleWsMessageMsg(msg.msg)).map(
							move |r| match r {
								Err(e) => {
									error!(logger, "Failed to handle websocket message";
										"error" => %e);
								}
								Ok(()) => {}
							},
						));
					});
				})
				.invoke_handler(move |webview, arg| {
					match serde_json::from_str::<messages::TauriHttpRequestWrapper>(arg) {
						Err(e) => Err(format!("Failed to parse message {}", e)),
						Ok(command) => {
							let req = command.req;
							let state = state3.clone();
							tauri::execute_promise(
								webview,
								move || {
									let (send, recv) = mpsc::channel();
									actix::spawn(async move {
										let r = handle_tauri_request(&state, req).await;
										let _ = send.send(r);
									});
									Ok(recv.recv()??)
								},
								command.callback,
								command.error,
							);
							Ok(())
						}
					}
				})
				.build()
				.run();
		} else {
			let frontend_path = std::option_env!("FRONTEND_PATH").unwrap_or("../frontend/build/");
			let is_production = std::option_env!("FRONTEND_PATH").is_some();
			let state2 = state.clone();
			HttpServer::new(move || {
				let state = state2.clone();
				actix_web::App::new()
					.wrap(Condition::new(!is_production, Cors::permissive().max_age(3600)))
					.data(state)
					.service(create_ws)
					.service(run_shortcut)
					.service(audio_reset)
					.service(list_plugins)
					.service(get_plugin)
					.service(download_file)
					.service(upload_file)
					.service(download_cache_file)
					.service(get_transient_setting)
					.service(set_transient_setting)
					.service(get_link_preview)
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
		}

		// Quit all connections
		info!(state.logger, "Closing remaining connections");
		{
			let cons = state.connections.lock().unwrap();
			for con in cons.values() {
				actix::spawn(con.send(websocket::DisconnectMsg).map(|_| ()));
			}
		}

		// Wait at max a second and poll
		for _ in 0u8..10 {
			{
				let cons = state.connections.lock().unwrap();
				if cons.is_empty() {
					break;
				}
			}
			time::delay_for(Duration::from_millis(10)).await;
		}

		Ok(())
	}
}

#[actix_rt::main]
async fn main() -> Result<()> {
	let logger = {
		let decorator = slog_term::TermDecorator::new().build();
		let drain = slog_term::CompactFormat::new(decorator).build();
		let drain = slog_envlogger::new(drain).fuse();
		let drain = slog_async::Async::new(drain).build().fuse();

		slog::Logger::root(drain, o!())
	};

	// Parse command line options
	let args = Args::from_args();

	App::run(logger, args).await
}

/// Tests need a running TeamSpeak server on localhost. The default channel has to be channel 1,
/// this is used to access messages.
#[cfg(test)]
mod tests {
	use anyhow::format_err;
	use awc::ws;
	use rand::Rng;

	use juniper::http::GraphQLRequest;
	use tsclientlib::{ClientId, Version};

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
			Self { logger, port: rng.gen_range(1025, 65535) }
		}

		async fn create_connection(&self) -> Result<Connection> {
			let client = awc::Client::default();
			let id = Uuid::new_v4();
			let url = format!("ws://127.0.0.1:{}/con/{}/ws?format=Msgpack", self.port, id);
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
					no_open: true,
					browser: true,
					verbosity: 1,
				};
				App::run(logger, args).await?;
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
				version: Version::Linux_3_X_X,
				bookmark: None,
				channel: None,
				ignore_identity_mismatch: false,
				log_commands: false,
				log_packets: false,
				log_udp_packets: false,
			}))
			.await?;
			loop {
				if let MessageP2F::Connected { own_client, .. } = self.recv().await? {
					return Ok(ClientId(own_client.parse().unwrap()));
				}
			}
		}

		async fn send(&mut self, msg: &MessageF2P) -> Result<()> {
			self.socket
				.send(ws::Message::Binary(rmp_serde::to_vec(msg)?.into()))
				.await
				.map_err(|e| format_err!("Websocket client protocol error: {:?}", e))?;
			Ok(())
		}

		async fn recv(&mut self) -> Result<MessageP2F> {
			match self.socket.next().await {
				Some(Ok(ws::Frame::Binary(msg))) => Ok(rmp_serde::from_read_ref(msg.as_ref())?),
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
		time::delay_for(Duration::from_millis(100)).await;
		let mut con = proxy.create_connection().await?;
		con.connect().await?;
		// Wait for saving the connection in the database
		time::delay_for(Duration::from_millis(100)).await;
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
		time::delay_for(Duration::from_millis(100)).await;
		let mut con0 = proxy0.create_connection().await?;
		con0.connect().await?;
		let mut con1 = proxy1.create_connection().await?;
		let con1_id = con1.connect().await?;

		// con0 sends a message to con1
		let msg = "Hello 1";
		con0.send(&MessageF2P::SendMessage {
			target: JsMessageTarget::Client(con1_id),
			message: msg.to_string(),
		})
		.await?;

		// Wait for saving the message in the database
		time::delay_for(Duration::from_millis(100)).await;
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
