#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use actix::*;
use actix_files::Files;
use actix_web::*;
use actix_web_actors::ws;
use bytes::BytesMut;
use failure::{format_err, Error};
use futures::prelude::*;
use serde::Deserialize;
use slog::{error, info, o, warn, Drain, Logger};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tokio::net::TcpStream;
use tokio_util::codec::{BytesCodec, FramedRead};
//use tokio01::codec::{BytesCodec, FramedRead};
use tsclientlib::ChannelId;
use uuid::Uuid;

mod audio;
mod db;
mod secret;
mod websocket;

use secret::Secret;
use websocket::Ws;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConnectionId(pub Uuid);

#[derive(StructOpt, Debug)]
#[structopt(raw(global_settings = "&[AppSettings::ColoredHelp, \
	AppSettings::VersionlessSubcommands]"))]
struct Args {
	#[structopt(
		short = "a",
		long = "address",
		help = "The address where the server listens"
	)]
	listen_address: Option<String>,
	#[structopt(
		short = "i",
		long = "identity",
		help = "The id of the identity that is used by default"
	)]
	default_identity: Option<u64>,
	/// The path for all the settings files. This makes only senses as a command
	/// line argument, it is ignored in the settings file.
	///
	/// If no value is given, the configuration path depends on the operating
	/// system.
	#[structopt(
		short = "c",
		long = "config-path",
		help = "The folder that contains all the configuration files"
	)]
	config_path: Option<String>,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[structopt(
		short = "v",
		long = "verbose",
		help = "Print the content of all packets",
		parse(from_occurrences)
	)]
	verbosity: u8,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Settings {
	#[serde(default = "default_listen_address")]
	listen_address: String,
	#[serde(skip)]
	config_path: PathBuf,
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

fn default_listen_address() -> String { "127.0.0.1:4422".into() }

impl Default for Settings {
	fn default() -> Self {
		Self {
			listen_address: default_listen_address(),
			config_path: Default::default(),
			default_identity: Default::default(),
			verbosity: Default::default(),
		}
	}
}

#[get("/ws/{id}")]
async fn create_ws(state: web::Data<State>, uuid: web::Path<Uuid>,
	req: HttpRequest, stream: web::Payload) -> impl Responder {
	let id = ConnectionId(*uuid);

	// Check that the id does not exist
	let mut cons = state.connections.lock().unwrap();
	if cons.contains_key(&id) {
		return Ok(HttpResponse::PreconditionFailed().finish());
	}

	let ws_con = Ws::new(
		state.deref().clone(),
		id,
	);

	ws::start_with_addr(ws_con, &req, stream).map(|(addr, resp)| {
		cons.insert(id, addr);
		resp
	})
}

#[post("/audiosend/true")]
async fn audiosend_true(state: web::Data<State>) -> impl Responder {
	if state.audio_data.a2ts.send(audio::audio_to_ts::SetPlayingMsg(true)).await.is_err() {
		error!(state.logger, "Failed to set playing state");
		HttpResponse::InternalServerError()
	} else {
		HttpResponse::Ok()
	}
}

#[post("/audiosend/false")]
async fn audiosend_false(state: web::Data<State>) -> impl Responder {
	if state.audio_data.a2ts.send(audio::audio_to_ts::SetPlayingMsg(false)).await.is_err() {
		error!(state.logger, "Failed to set playing state");
		HttpResponse::InternalServerError()
	} else {
		HttpResponse::Ok()
	}
}

#[get("/file/{id}/{channel}/{path:.*}")]
async fn download_file(state: web::Data<State>, data: web::Path<(Uuid, u64, PathBuf)>)
	-> Result<HttpResponse, Error> {
	let channel = ChannelId(data.1);
	let cons = state.connections.lock().unwrap();
	if let Some(con) = cons.get(&ConnectionId(data.0)) {
		let (len, file_stream): (u64, TcpStream) = con.send(websocket::DownloadFile {
			channel,
			path: data.2.clone(),
		}).await??;
		let stream = FramedRead::new(file_stream, BytesCodec::new())
			.map(|r| r.map(BytesMut::freeze));
		println!("Streaming {} from {:?}", len, stream);
		//Ok(HttpResponse::Ok().streaming(stream))
		Ok(HttpResponse::Ok().content_length(len).streaming(stream))
	} else {
		Ok(HttpResponse::Gone().finish())
	}
}

#[actix_rt::main]
async fn main() -> Result<(), Error> {
	let logger = {
		let decorator = slog_term::TermDecorator::new().build();
		let drain = slog_term::CompactFormat::new(decorator).build().fuse();
		let drain = slog_async::Async::new(drain).build().fuse();

		slog::Logger::root(drain, o!())
	};
	let _scope_guard = slog_scope::set_global_logger(logger.clone());
	let _log_guard = slog_stdlog::init().unwrap();

	// Parse command line options
	let args = Args::from_args();

	let config_path: PathBuf = if let Some(p) = args.config_path {
		p.into()
	} else {
		let proj_dirs = match directories::ProjectDirs::from("",
			DIR_ORGANIZATION, DIR_PROJECT) {
			Some(r) => r,
			None => {
				return Err(format_err!("Failed to get project directory"));
			}
		};
		proj_dirs.config_dir().into()
	};

	// Load settings
	let mut settings = match fs::read_to_string(&config_path.join("config.toml")) {
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
	if let Some(a) = args.listen_address {
		settings.listen_address = a;
	}
	if let Some(a) = args.default_identity {
		settings.default_identity = a;
	}
	if args.verbosity > settings.verbosity {
		settings.verbosity = args.verbosity;
	}

	// Open database
	let database = db::DbHandler::new(logger.clone(), &settings, key)?.start();

	// Start sound
	let audio_data = audio::start(logger.clone())?;

	let addr = settings.listen_address.clone();

	let state = State {
		logger,
		connections: Arc::new(Mutex::new(HashMap::new())),
		audio_data,
		settings,
		database,
	};

	Ok(HttpServer::new(move || {
		let state = state.clone();
		App::new()
			.wrap(middleware::Logger::default())
			.data(state)
			.service(create_ws)
			.service(audiosend_true)
			.service(audiosend_false)
			.service(download_file)
			.service(Files::new("", "../frontend/static/")
				.index_file("index.html")
				.default_handler(Files::new("",
						"../frontend/target/wasm32-unknown-unknown/debug/")))
	})
	.bind(addr)?
	.run()
	.await?)
}
