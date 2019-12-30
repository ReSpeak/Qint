#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};

use actix::*;
use actix_web::*;
use actix_web::fs::StaticFiles;
use actix_web::http::Method;
use failure::{format_err, Error};
use futures01::Future as _;
use serde::Deserialize;
use slog::{error, info, o, warn, Drain};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tsclientlib::Uid;
use uuid::Uuid;

mod audio;
mod db;
mod files;
mod secret;
mod websocket;

use secret::Secret;
use websocket::Ws;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";

type BoxFuture<T, E=Error> = Box<dyn futures01::Future<Item=T, Error=E>>;

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
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating
	/// system.
	#[structopt(
		long = "cache-path",
		help = "The folder that contains cached files"
	)]
	cache_path: Option<String>,
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
	#[serde(default = "default_cache_path")]
	cache_path: PathBuf,
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

fn default_listen_address() -> String { "127.0.0.1:4422".into() }

fn default_cache_path() -> PathBuf {
	let proj_dirs = match directories::ProjectDirs::from("", DIR_ORGANIZATION,
		DIR_PROJECT) {
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
			default_identity: Default::default(),
			verbosity: Default::default(),
		}
	}
}

fn main() -> Result<(), Error> {
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
	if let Some(a) = args.cache_path {
		settings.cache_path = a.into();
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

	// The list of all currently existing connections
	let connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>> =
		Arc::new(Mutex::new(HashMap::new()));

	// Open database
	let database = db::DbHandler::new(logger.clone(), &settings, key)?.start();

	// Open cache
	let file_cache = files::FileCache::new(settings.cache_path.clone())?.start();

	// Start sound
	let audio_data = audio::start(logger.clone())?;

	let addr = settings.listen_address.clone();
	server::new(move || {
		let logger2 = logger.clone();
		let connections2 = connections.clone();
		let audio_data2 = audio_data.clone();
		let settings = settings.clone();
		let database = database.clone();
		let file_cache2 = file_cache.clone();
		let mut app = App::new()
			.middleware(middleware::Logger::default())
			.resource("/ws/{id}", |r| r.f(move |req| {
				let id = req.match_info().get("id").unwrap();
				let uuid = match Uuid::parse_str(id) {
					Ok(r) => r,
					Err(e) => {
						error!(logger2, "Failed to parse uuid"; "uuid" => id,
							"error" => ?e);
						return Ok(HttpResponse::BadRequest().finish());
					}
				};
				let id = ConnectionId(uuid);

				// Check that the id does not exist
				let mut cons = connections2.lock().unwrap();
				if cons.contains_key(&id) {
					return Ok(HttpResponse::PreconditionFailed().finish());
				}

				let ws_con = Ws::new(
					id,
					logger2.clone(),
					audio_data2.clone(),
					settings.clone(),
					database.clone(),
					file_cache2.clone(),
					connections2.clone(),
				);

				let res = ws::start(req, ws_con);
				if res.is_ok() {
					//let addr = recv.recv().unwrap();
					//cons.insert(id, addr);
				}
				res
			}));

		let addr = audio_data.a2ts.clone();
		let addr2 = audio_data.a2ts.clone();
		let file_cache_addr = file_cache.clone();
		let logger = logger.clone();
		let logger2 = logger.clone();
		app = app.route("/audiosend/true", Method::POST, move |_: HttpRequest| {
			let logger = logger.clone();
			actix::spawn(addr.send(audio::audio_to_ts::SetPlayingMsg(true))
				.then(move |r| {
					match r {
						Ok(()) => {}
						Err(_) => {
							error!(logger, "Failed to set playing state");
						}
					}
					Ok(())
				}));
			HttpResponse::Ok()
		}).route("/audiosend/false", Method::POST, move |_: HttpRequest| {
			let logger = logger2.clone();
			actix::spawn(addr2.send(audio::audio_to_ts::SetPlayingMsg(false))
				.then(move |r| {
					match r {
						Ok(()) => {}
						Err(_) => {
							error!(logger, "Failed to set playing state");
						}
					}
					Ok(())
				}));
			HttpResponse::Ok()
		}).resource("/files/{server}/{type}/{name}", |r| {
			r.get().a(move |req| -> BoxFuture<HttpResponse> {
				let info = req.match_info();
				let server = Uid(info.get("server").unwrap().into());
				let name = info.get("name").unwrap();
				let file = match info.get("type").unwrap() {
					"avatar" => files::CachedFile::Avatar {
						server,
						client: Uid(name.into()),
					},
					"icon" => files::CachedFile::Icon {
						server,
						name: name.into(),
					},
					_ => return Box::new(futures01::future::ok(HttpResponse::BadRequest().finish())),
				};
				files::handle_request(file, &file_cache_addr)
			});
		});
		app = app.handler("/", StaticFiles::new("../frontend/target/wasm32-unknown-unknown/debug/")
			.expect("static files not found")
			.default_handler(StaticFiles::new("../frontend/static/")
				.expect("Static files not found")
				.index_file("index.html"))
		);

		app.finish()
	})
		.bind(addr)
		.unwrap()
		.run();
	Ok(())
}
