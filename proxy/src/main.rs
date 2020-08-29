#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex, RwLock};

use actix::*;
use actix_cors::Cors;
use actix_files::Files;
use actix_web::dev::Service;
use actix_web::*;
use actix_web_actors::ws;
use anyhow::{bail, Result};
use futures::prelude::*;
use http::{header::CACHE_CONTROL, header::ETAG, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use slog::{debug, error, info, o, warn, Drain, Logger};
use structopt::StructOpt;
use tokio::time::{self, Duration};
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::{ChannelId, Uid};
use uuid::Uuid;

mod audio;
mod book_events;
mod db;
mod filecache;
mod markdown;
mod messages;
mod secret;
mod shortcut;
mod websocket;

use filecache::FileCache;
use secret::Secret;
use websocket::Ws;
const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";
const SETTINGS_FILENAME: &str = "config.toml";
const TRANSIENT_SETTINGS_FILENAME: &str = "transient.toml";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConnectionId(pub Uuid);

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
struct Args {
	/// The address where the server listens
	#[structopt(short = "l", long)]
	listen_address: Option<String>,
	/// The id of the identity that is used by default
	#[structopt(short = "i", long)]
	default_identity: Option<u64>,
	/// The path for all the settings files. This makes only senses as a command
	/// line argument, it is ignored in the settings file.
	///
	/// If no value is given, the configuration path depends on the operating
	/// system.
	#[structopt(short = "c", long)]
	config_path: Option<PathBuf>,
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating
	/// system.
	#[structopt(long)]
	cache_path: Option<PathBuf>,
	/// The path for plugins.
	///
	/// If no value is given, this is the path of the config file plus
	/// `plugins/`.
	#[structopt(long)]
	plugin_path: Option<String>,
	/// Do not capture and play audio.
	// This is used for testing, which cannot initialize SDL.
	// SDL must only be initialized once per process, at the same time, it can only be used from a
	// single thread, which does not work well with parallel tests.
	#[structopt(long)]
	no_audio: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[structopt(short = "v", long, parse(from_occurrences))]
	verbosity: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct TransientSettings {
	#[serde(flatten, serialize_with = "toml::ser::tables_last")]
	fields: Map<String, Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Settings {
	#[serde(default = "default_listen_address")]
	listen_address: String,
	#[serde(skip)]
	config_path: PathBuf,
	#[serde(default = "default_cache_path")]
	cache_path: PathBuf,
	#[serde(default)]
	plugin_path: PathBuf,
	#[serde(default)]
	default_identity: u64,
	/// Do not capture and play audio.
	#[serde(default)]
	no_audio: bool,
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
	secret: Secret,
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

fn default_listen_address() -> String { "127.0.0.1:4422".into() }

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
			verbosity: Default::default(),
			shortcuts: Default::default(),
		}
	}
}

impl juniper::Context for State {}

impl State {
	fn modify_transient_settings<T: FnOnce(&mut TransientSettings)>(&self, f: T) {
		let mut settings = self.transient_settings.write().unwrap();
		f(&mut *settings);
		if let Err(e) = settings.save(&self.settings.read().unwrap().config_path) {
			error!(self.logger, "Failed to save transient settings"; "error" => %e);
		}
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

	let ws_con = Ws::new(state.logger.clone(), (**state).clone(), options.0, id);
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

#[get("/plugins")]
async fn list_plugins(state: web::Data<Arc<State>>) -> impl Responder {
	let path = &state.settings.read().unwrap().plugin_path;
	let mut res = Vec::new();
	let dir = match path.read_dir() {
		Ok(r) => r,
		Err(e) => {
			warn!(state.logger, "Failed to list plugins"; "dir" => ?path, "error" => %e);
			return std::io::Result::<_>::Ok(web::Json(Vec::new()));
		}
	};
	for p in dir {
		if let Ok(p) = p?.file_name().into_string() {
			res.push(p);
		}
	}
	Ok(web::Json(res))
}

#[get("/plugins/{name}")]
async fn get_plugin(state: web::Data<Arc<State>>, data: web::Path<String>) -> impl Responder {
	let path = state.settings.read().unwrap().plugin_path.join(&*data);
	fs::read_to_string(path)
		.with_header(http::header::CONTENT_TYPE, "application/javascript; charset=utf-8")
}

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<Arc<State>>, data: web::Path<(Uuid, u64, String)>,
) -> impl Responder {
	let channel = ChannelId(data.1);
	let cons = state.connections.lock().unwrap();
	if let Some(con) = cons.get(&ConnectionId(data.0)).cloned() {
		drop(cons);
		debug!(state.logger, "Downloading file"; "channel" => data.1,
			"path" => &data.2);
		let (len, file_stream, server) =
			match con.send(websocket::DownloadFile { channel, path: data.2.clone() }).await {
				Err(_) => {
					return HttpResponse::Gone().finish();
				}
				Ok(Err(e)) => {
					error!(state.logger, "File download failed"; "error" => %e);
					return HttpResponse::InternalServerError()
						.body(format!("Failed to download file: {}", e));
				}
				Ok(Ok(r)) => r,
			};

		let stream =
			FramedRead::new(file_stream, BytesCodec::new()).map(|r| r.map(web::BytesMut::freeze));
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

		// Cache icons and avatars for offline usage
		if channel.0 == 0 && (data.2.starts_with("icon_") || data.2.starts_with("avatar_")) {
			let stream = FileCache::cache_file(&*state, server, channel, &data.2, stream).await;
			response.content_length(len).streaming(stream)
		} else {
			response.content_length(len).streaming(stream)
		}
	} else {
		HttpResponse::Gone().finish()
	}
}

/// Get a cached file by server id, channel and path.
#[get("/filecache/{id}/{channel}/{path:.*}")]
async fn download_cache_file(
	state: web::Data<Arc<State>>, data: web::Path<(String, u64, String)>,
) -> impl Responder {
	let server = match base64::decode(&data.0) {
		Err(e) => {
			return HttpResponse::BadRequest().body(format!("Not a valid server uid: {}", e));
		}
		Ok(uid) => Uid(uid),
	};
	let channel = ChannelId(data.1);
	if let Some((len, stream)) = FileCache::get_cached_file(&*state, server, channel, &data.2).await
	{
		HttpResponse::Ok().content_length(len).streaming(stream)
	} else {
		HttpResponse::NotFound().finish()
	}
}

#[get("/transient/{key}")]
async fn get_transient_setting(
	state: web::Data<Arc<State>>, data: web::Path<String>,
) -> impl Responder {
	let transient_values = state.transient_settings.read().unwrap();
	let req = data.as_str();
	if req == "*" {
		HttpResponse::Ok().json(&*transient_values)
	} else if let Some(value) = transient_values.fields.get(req) {
		HttpResponse::Ok().json(value)
	} else {
		HttpResponse::NotFound().body("Unknown key".to_string())
	}
}

#[put("/transient/{key}")]
async fn set_transient_setting(
	state: web::Data<Arc<State>>, data: web::Path<String>, body: web::Json<Value>,
) -> impl Responder {
	let req = data.as_str();
	if req == "*" && !body.0.is_object() {
		HttpResponse::Forbidden().body("*-assign must be an object".to_string())
	} else {
		state.modify_transient_settings(|transient_values| {
			if req == "*" {
				if let Value::Object(obj) = body.0 {
					for (k, v) in obj.into_iter() {
						transient_values.set(k, v);
					}
				} else {
					panic!("Should be object (see 'if' check above)");
				}
			} else {
				transient_values.set(req.to_string(), body.0);
			}
		});
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

impl App {
	async fn run(logger: Logger, args: Args) -> Result<()> {
		let _scope_guard = slog_scope::set_global_logger(logger.clone());
		// Ignore errors if a logger has already been set
		let _ = slog_stdlog::init();

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
		if args.verbosity > settings.verbosity {
			settings.verbosity = args.verbosity;
		}

		if settings.plugin_path.to_str() == Some("") {
			settings.plugin_path = settings.config_path.join("plugins");
		}

		// Open database
		let database = db::DbHandler::new(logger.clone(), &settings, secret.clone())?.start();

		let connections = Arc::new(Mutex::new(HashMap::new()));

		// Start sound
		let audio_data = if settings.no_audio { None }
		else { Some(audio::start(logger.clone(), connections.clone())?) };
		let shortcut_config = settings.shortcuts.clone();
		let shortcuts = shortcut::Shortcuts::new(shortcut_config)?;
		let addr = settings.listen_address.clone();

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
		let state = Arc::new(State {
			logger,
			connections,
			audio_data,
			shortcuts,
			settings: RwLock::new(settings),
			transient_settings: RwLock::new(transient_settings),
			database,
			graphql_schema,
			secret,
		});

		state.shortcuts.apply_config(&state)?;

		let state2 = state.clone();
		HttpServer::new(move || {
			let state = state2.clone();
			actix_web::App::new()
				.wrap(Cors::new().max_age(3600).finish())
				.data(state)
				.service(create_ws)
				.service(run_shortcut)
				.service(audio_reset)
				.service(list_plugins)
				.service(get_plugin)
				.service(download_file)
				.service(download_cache_file)
				.service(get_transient_setting)
				.service(set_transient_setting)
				.service(db::graphql::db_graphql)
				.service(db::graphql::graphiql)
				.service(Files::new("", "../frontend/public/").index_file("index.html"))
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

/// Tests need a running TeamSpeak server on localhost.
#[cfg(test)]
mod tests {
	use anyhow::format_err;
	use awc::ws;
	use rand::Rng;

	use juniper::http::GraphQLRequest;
	use tsclientlib::Version;

	use super::*;
	use messages::{ConnectOptions, MessageF2P, MessageP2F};

	struct TestProxy {
		logger: Logger,
		port: u16,
	}

	struct Connection {
		logger: Logger,
		port: u16,
		id: Uuid,
		socket: actix_codec::Framed<awc::BoxedSocket, ws::Codec>,
	}

	#[derive(Deserialize)]
	struct GraphQLResponse<T> {
		data: T,
	}

	#[derive(Deserialize)]
	struct ClientServerKey {
		/// Public key of the server.
		server: String,
		/// Uid of the own identity.
		client: String,
	}

	impl TestProxy {
		fn new() -> Self {
			let logger = create_logger();
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
			Ok(Connection { logger: self.logger.clone(), port: self.port, id, socket })
		}

		async fn graphql<T>(&self, request: &GraphQLRequest) -> Result<T>
		where for<'a> T: Deserialize<'a> {
			let client = awc::Client::default();
			let url = format!("http://127.0.0.1:{}/db", self.port);
			let mut resp = client
				.post(url)
				.send_json(request)
				.await
				.map_err(|_| format_err!("GraphQL failed"))?;
			if !resp.status().is_success() {
				bail!("GraphQL request failed");
			}
			let resp: GraphQLResponse<T> =
				resp.json().await.map_err(|_| format_err!("Failed to decode json"))?;
			Ok(resp.data)
		}

		async fn get_client_server_key(&self) -> Result<ClientServerKey> {
			#![allow(non_snake_case)]

			#[derive(Deserialize)]
			struct Server {
				publicKey: String,
			}
			#[derive(Deserialize)]
			struct Client {
				uid: String,
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
			struct RecentBookmark {
				mostRecentBookmark: Bookmark,
			}

			let resp: RecentBookmark = self
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
				client: resp.mostRecentBookmark.identity.client.uid,
				server: resp.mostRecentBookmark.server.publicKey,
			})
		}

		fn run(&self) -> impl Future<Output = Result<()>> {
			let logger = self.logger.clone();
			let port = self.port;
			async move {
				let dir = tempfile::Builder::new().prefix("qint-proxy").tempdir()?;
				info!(logger, "Using config directory"; "dir" => dir.path().display());
				let args = Args {
					listen_address: Some(format!("127.0.0.1:{}", port)),
					default_identity: None,
					config_path: Some(dir.path().join("config")),
					cache_path: Some(dir.path().join("cache")),
					plugin_path: None,
					no_audio: true,
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
		async fn connect(&mut self) -> Result<()> {
			self.send(&MessageF2P::Connect(ConnectOptions {
				address: "localhost".to_string(),
				name: "Test".to_string(),
				version: Version::Linux_3_X_X,
				log_commands: false,
				log_packets: false,
				log_udp_packets: false,
			}))
			.await?;
			while {
				let msg = self.recv().await?;
				if let MessageP2F::Connected { .. } = msg { false } else { true }
			} {}
			Ok(())
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
		let proxy = TestProxy::new();
		actix::spawn(proxy.run_log_errors());
		// Wait for server to come up
		time::delay_for(Duration::from_millis(100)).await;
		let mut con = proxy.create_connection().await?;
		con.connect().await?;
		// Wait for saving the connection in the database
		time::delay_for(Duration::from_millis(100)).await;
		drop(con);

		#[derive(Deserialize)]
		struct ServerServer {
			#[allow(non_snake_case)]
			publicKey: String,
		}
		#[derive(Deserialize)]
		struct ServerBookmark {
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
		assert_eq!(response.bookmarks.len(), 1, "Should have one recent connection");
		Ok(())
	}

	/// Check that getting or sending a message from a client saves the other client and the
	/// message.
	#[actix_rt::test]
	async fn test_save_client() -> Result<()> {
		let proxy = TestProxy::new();
		actix::spawn(proxy.run_log_errors());
		// Wait for server to come up
		time::delay_for(Duration::from_millis(100)).await;
		let mut con = proxy.create_connection().await?;
		con.connect().await?;
		// Wait for saving the connection in the database
		time::delay_for(Duration::from_millis(100)).await;
		drop(con);

		#[derive(Deserialize)]
		struct ServerServer {
			#[allow(non_snake_case)]
			publicKey: String,
		}
		#[derive(Deserialize)]
		struct ServerBookmark {
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
		assert_eq!(response.bookmarks.len(), 1, "Should have one recent connection");
		Ok(())
	}
}
