#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use actix::*;
use actix_files::Files;
use actix_web::*;
use actix_web_actors::ws;
use anyhow::{bail, Result};
use futures::prelude::*;
use serde::Deserialize;
use slog::{debug, error, info, o, warn, Drain, Logger};
use structopt::StructOpt;
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::ChannelId;
use uuid::Uuid;

mod audio;
mod db;
mod messages;
mod secret;
mod websocket;

use secret::Secret;
use websocket::Ws;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";

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
	config_path: Option<String>,
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating
	/// system.
	#[structopt(long)]
	cache_path: Option<String>,
	/// The path for plugins.
	///
	/// If no value is given, this is the path of the config file plus
	/// `plugins/`.
	#[structopt(long)]
	plugin_path: Option<String>,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[structopt(short = "v", long, parse(from_occurrences))]
	verbosity: u8,
}

#[derive(Clone, Debug, Deserialize)]
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
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[serde(default)]
	verbosity: u8,
}

#[derive(Clone)]
struct State {
	logger: Logger,
	/// The list of all currently existing connections
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	audio_data: audio::AudioData,
	settings: Settings,
	database: Addr<db::DbHandler>,
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

fn default_listen_address() -> String { "127.0.0.1:4422".into() }

fn default_cache_path() -> PathBuf {
	let proj_dirs =
		match directories::ProjectDirs::from("", DIR_ORGANIZATION, DIR_PROJECT)
		{
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
			verbosity: Default::default(),
		}
	}
}

#[get("/con/{id}/ws")]
async fn create_ws(
	state: web::Data<State>, uuid: web::Path<Uuid>,
	options: web::Query<WsOptions>, req: HttpRequest, stream: web::Payload,
) -> impl Responder
{
	let id = ConnectionId(*uuid);

	// Check that the id does not exist
	let cons = state.connections.lock().unwrap();
	if cons.contains_key(&id) {
		return Either::A(
			HttpResponse::PreconditionFailed()
				.body("Connection id is already occupied".to_string()),
		);
	}

	let ws_con = Ws::new(state.logger.clone(), (*state).clone(), options.0, id);
	Either::B(ws::start(ws_con, &req, stream))
}

#[post("/audiosend/true")]
async fn audiosend_true(state: web::Data<State>) -> impl Responder {
	if state
		.audio_data
		.a2ts
		.send(audio::audio_to_ts::SetPlayingMsg(true))
		.await
		.is_err()
	{
		error!(state.logger, "Failed to set playing state");
		HttpResponse::InternalServerError()
	} else {
		HttpResponse::Ok()
	}
}

#[post("/audiosend/false")]
async fn audiosend_false(state: web::Data<State>) -> impl Responder {
	if state
		.audio_data
		.a2ts
		.send(audio::audio_to_ts::SetPlayingMsg(false))
		.await
		.is_err()
	{
		error!(state.logger, "Failed to set playing state");
		HttpResponse::InternalServerError()
	} else {
		HttpResponse::Ok()
	}
}

#[get("/plugins")]
async fn list_plugins(state: web::Data<State>) -> impl Responder {
	let path = &state.settings.plugin_path;
	let mut res = Vec::new();
	let dir = match path.read_dir() {
		Ok(r) => r,
		Err(e) => {
			warn!(state.logger, "Failed to list plugins"; "dir" => ?path, "error" => ?e);
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
async fn get_plugin(
	state: web::Data<State>, data: web::Path<String>,
) -> impl Responder {
	let path = state.settings.plugin_path.join(&*data);
	fs::read_to_string(path).with_header(
		http::header::CONTENT_TYPE,
		"application/javascript; charset=utf-8",
	)
}

#[get("/con/{id}/file/{channel}/{path:.*}")]
async fn download_file(
	state: web::Data<State>, data: web::Path<(Uuid, u64, String)>,
) -> impl Responder {
	let channel = ChannelId(data.1);
	let cons = state.connections.lock().unwrap();
	if let Some(con) = cons.get(&ConnectionId(data.0)).cloned() {
		drop(cons);
		debug!(state.logger, "Downloading file"; "channel" => data.1, "path" => &data.2);
		let (len, file_stream): (u64, TcpStream) = match con
			.send(websocket::DownloadFile { channel, path: data.2.clone() })
			.await
		{
			Err(_) => {
				return HttpResponse::Gone().finish();
			}
			Ok(Err(e)) => {
				error!(state.logger, "File download failed"; "error" => ?e);
				return HttpResponse::InternalServerError()
					.body("Failed to download file");
			}
			Ok(Ok(r)) => r,
		};

		// TODO Cache icons and avatars for offline usage
		// Use a general file cache (e.g. also for sent images) by TS-Server

		// Icons: There can be collisions as only CRC-32 is used, we may update
		// them at some time when the modification time on the server is newer
		// than on the cached file.
		let stream = FramedRead::new(file_stream, BytesCodec::new())
			.map(|r| r.map(web::BytesMut::freeze));
		HttpResponse::Ok().content_length(len).streaming(stream)
	} else {
		HttpResponse::Gone().finish()
	}
}

#[actix_rt::main]
async fn main() -> Result<()> {
	let logger = {
		let decorator = slog_term::TermDecorator::new().build();
		let drain = slog_term::CompactFormat::new(decorator).build().fuse();
		let drain = slog_async::Async::new(drain).build().fuse();

		slog::Logger::root(drain, o!())
	};
	let _scope_guard = slog_scope::set_global_logger(logger.clone());
	slog_stdlog::init().unwrap();

	// Parse command line options
	let args = Args::from_args();

	let config_path: PathBuf = if let Some(p) = args.config_path {
		p.into()
	} else {
		let proj_dirs = match directories::ProjectDirs::from(
			"",
			DIR_ORGANIZATION,
			DIR_PROJECT,
		) {
			Some(r) => r,
			None => bail!("Failed to get project directory"),
		};
		proj_dirs.config_dir().into()
	};

	// Load settings
	let mut settings =
		match fs::read_to_string(&config_path.join("config.toml")) {
			Ok(r) => toml::from_str(&r)?,
			Err(e) => {
				// Only a soft error
				info!(logger, "Failed to read settings, using defaults";
				"error" => %e);
				// Create settings directory
				fs::create_dir_all(&config_path)?;

				Settings::default()
			}
		};

	// Load secret key
	let key_path = config_path.join("secret.key");
	let key = match fs::read(&key_path) {
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
		settings.cache_path = a.into();
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
	if args.verbosity > settings.verbosity {
		settings.verbosity = args.verbosity;
	}

	if settings.plugin_path.to_str() == Some("") {
		settings.plugin_path = settings.config_path.join("plugins");
	}

	// Open database
	let database = db::DbHandler::new(logger.clone(), &settings, key)?.start();

	let connections = Arc::new(Mutex::new(HashMap::new()));

	// Start sound
	let audio_data = audio::start(logger.clone(), connections.clone())?;

	let addr = settings.listen_address.clone();

	let state = State { logger, connections, audio_data, settings, database };

	Ok(HttpServer::new(move || {
		let state = state.clone();
		App::new()
			//.wrap(middleware::Logger::default())
			.data(state)
			.service(create_ws)
			.service(audiosend_true)
			.service(audiosend_false)
			.service(list_plugins)
			.service(get_plugin)
			.service(download_file)
			.service(
				Files::new("", "../frontend/static/")
					.index_file("index.html")
					.default_handler(Files::new(
						"",
						"../frontend/dist/",
					)),
			)
	})
	.bind(addr)?
	.run()
	.await?)
}
