#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use actix::*;
use actix::fut::wrap_future;
use actix_web::*;
use actix_web::fs::StaticFiles;
use actix_web::http::Method;
use failure::{format_err, Error};
use futures01::sink::Sink as _;
use futures01::stream::Stream;
use futures01::Future as _;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use slog::{error, info, o, warn, Drain, Logger};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tsclientlib::{Connection, PacketHandler, PHBox};
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::{InAudio, InCommand};

mod audio;
mod db;
mod files;
mod secret;

use secret::Secret;

const DIR_ORGANIZATION: &str = "ReSpeak";
const DIR_PROJECT: &str = "Qint";

static NEXT_CON_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ConnectionId(u64);

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

/// Define http actor
struct Ws {
	logger: Logger,
	settings: Settings,
	database: Addr<db::DbHandler>,
	file_cache: Addr<files::FileCache>,
	id: ConnectionId,
	audio_data: audio::AudioData,
	connection: Option<Connection>,
}

#[derive(Clone)]
struct ProxyPacketHandler {
	logger: Logger,
	con: ConnectionId,
	addr: Addr<audio::ts_to_audio::TsToAudio>,
}

#[derive(Clone)]
struct ProxyCommandObserver {
	addr: Addr<Ws>,
}

// TODO Should not be an enum but two different messages
enum WsMessage {
	Packet(InCommandMsg),
	Message(MessageP2F),
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

fn next_con_id() -> ConnectionId {
	ConnectionId(NEXT_CON_ID.fetch_add(1, Ordering::Relaxed))
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl Message for WsMessage {
	type Result = ();
}

impl Ws {
	fn new(
		logger: Logger,
		audio_data: audio::AudioData,
		settings: Settings,
		database: Addr<db::DbHandler>,
		file_cache: Addr<files::FileCache>,
	) -> Self
	{
		Self {
			logger,
			settings,
			database,
			file_cache,
			id: next_con_id(),
			audio_data,
			connection: None,
		}
	}

	fn send_message(msg: &MessageP2F, ctx: &mut ws::WebsocketContext<Self>) {
		let mut buf = Vec::new();
		let mut ser = Serializer::new(&mut buf);
		msg.serialize(&mut ser).unwrap();
		ctx.binary(buf);
	}
}

impl Handler<WsMessage> for Ws {
	type Result = ();
	fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) -> Self::Result {
		match msg {
			WsMessage::Packet(packet) =>
				Self::send_message(&MessageP2F::Packet(packet), ctx),
			WsMessage::Message(msg) => Self::send_message(&msg, ctx),
		}
	}
}

/// Handler for ws::Message message
impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
	fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
		match msg {
			ws::Message::Ping(msg) => ctx.pong(&msg),
			ws::Message::Text(text) => ctx.text(text),
			ws::Message::Binary(bin) => {
				let mut de = Deserializer::new(bin.as_ref());
				let msg: MessageF2P = match Deserialize::deserialize(&mut de) {
					Ok(r) => r,
					Err(e) => {
						// TODO log
						eprintln!("Error deserializing message: {:?}", e);
						return;
					}
				};

				match msg {
					MessageF2P::Connect(o) => {
						let ts2a = self.audio_data.ts2a.clone();
						let id = self.settings.default_identity;
						ctx.spawn(wrap_future(self.database.send(db::GetIdentityMsg(id, true)).from_err().and_then(|r| r))
							.and_then(move |identity, actor: &mut Self, ctx| {
								let addr = ctx.address();
								let db_addr = actor.database.clone();
								let db_addr2 = db_addr.clone();
								let logger = actor.logger.clone();
								let server_addr = o.address.clone();
								let con = Connection::new(tsclientlib::ConnectOptions::new(o.address)
									.name(o.name)
									.identity(identity)
									.logger(actor.logger.clone())
									.log_commands(o.log_commands || actor.settings.verbosity > 0)
									.log_packets(o.log_packets || actor.settings.verbosity > 1)
									.log_udp_packets(o.log_udp_packets || actor.settings.verbosity > 2)
									.add_event_listener("Qint".into(), Box::new(move |e| {
										let event = match e {
											tsclientlib::Event::ConEvents(con, events) => {
												db::EventMsg::Events(con.get_locked(), events.iter().map(|e| e.clone()).collect())
											}
											tsclientlib::Event::IdentityLevelIncreased(id) => {
												db::EventMsg::UpdateIdentity((*id).clone())
											}
											_ => return,
										};
										let logger = logger.clone();
										actix::spawn(db_addr.send(event)
											.then(move |r| {
												match r {
													Ok(Ok(())) => {}
													Ok(Err(e)) => {
														error!(logger, "Failed to handle event in database"; "error" => ?e);
													}
													Err(_) => {
														error!(logger, "Failed to send event to database");
													}
												}
												Ok(())
											}));
									}))
									.prepare_client(Box::new(move |client| {
										client.lock().add_in_command_observer(
											"Qint".into(),
											Box::new(ProxyCommandObserver {
												addr: addr.clone(),
											}),
										);
									}))
									.handle_packets(Box::new(ProxyPacketHandler {
										logger: actor.logger.clone(),
										con: actor.id,
										addr: ts2a.clone(),
									}))
								);

								let logger = actor.logger.clone();
								wrap_future(con.from_err().map(move |r| {
									let event = db::EventMsg::Connected(server_addr, r.clone());
									actix::spawn(db_addr2.send(event)
										.then(move |r| {
											match r {
												Ok(Ok(())) => {}
												Ok(Err(e)) => {
													error!(logger, "Failed to handle event in database"; "error" => ?e);
												}
												Err(_) => {
													error!(logger, "Failed to send event to database");
												}
											}
											Ok(())
										}));
									r
								}))
							}).map_err(|_e, _actor, ctx| {
								Self::send_message(&MessageP2F::ConnectFailed(), ctx);
							}).map(move |con, actor: &mut Self, _| {
									let c = con.clone();
									actor.connection = Some(con);

									// Activate audio
									// TODO Handle disconnect
									let logger = actor.logger.clone();
									let a2ts = actor.audio_data.a2ts.clone();
									actix::spawn(a2ts.send(audio::audio_to_ts::SetListenerMsg {
										connection: c,
									}).map_err(move |e| {
										error!(logger, "Failed to set listener"; "error" => ?e);
									}));
							})
							.map_err(move |e, actor, _ctx| {
								error!(actor.logger, "Failed to get identity for conection";
									"error" => ?e);
							}));
					}
					MessageF2P::SetTalking(talk) => {
						let logger = self.logger.clone();
						actix::spawn(self.audio_data.a2ts.send(audio::audio_to_ts::SetPlayingMsg(talk))
							.then(move |r| {
								match r {
									Ok(()) => {}
									Err(e) => error!(logger, "Failed to set playing state"; "error" => ?e),
								}
								Ok(())
							}));
					}
					MessageF2P::Packet(packet) => {
						if let Some(con) = &mut self.connection {
							let sink = con.get_packet_sink();
							ctx.spawn(wrap_future(futures01::future::lazy(move || {
								sink.send(packet).map(|_| ())
							})).map_err(|e, _actor: &mut Ws, _ctx| {
								// TODO Return
								eprintln!("Failed to send packet: {:?}", e);
							}));
						}
					}
					MessageF2P::Webrtc(_) => {
						// No webrtc
						error!(self.logger, "Got unsupported webrtc message, ignore it");
					}
				}
			}
			_ => {}
		}
	}
}

impl PacketHandler for ProxyPacketHandler {
	fn new_connection(
		&mut self,
		command_stream: Box<
			dyn Stream<Item = InCommand, Error = tsproto::Error> + Send,
		>,
		audio_stream: Box<
			dyn Stream<Item = InAudio, Error = tsproto::Error> + Send,
		>,
	) {
		let logger = self.logger.clone();
		actix::spawn(command_stream.for_each(|_| Ok(())).map_err(move |e| {
			error!(logger, "Failed to handle packets"; "error" => ?e);
		}));

		let logger = self.logger.clone();
		let logger2 = self.logger.clone();
		let con = self.con;
		let addr = self.addr.clone();
		actix::spawn(audio_stream.for_each(move |packet| {
			let logger = logger.clone();
			addr.send(audio::ts_to_audio::PlayMsg(con, packet)).then(move |r| {
				match r {
					Ok(Ok(())) => {}
					Ok(Err(e)) => error!(logger, "Failed to play audio packet"; "error" => ?e),
					Err(e) => error!(logger, "Failed to play audio packet"; "error" => ?e),
				}
				Ok(())
			})
		}).map_err(move |e| {
			error!(logger2, "Failed to handle packets"; "error" => ?e);
		}));
	}

	/// Clone into a box.
	fn clone(&self) -> PHBox {
		Box::new(Clone::clone(self))
	}
}

impl<T> InCommandObserver<T> for ProxyCommandObserver {
	fn observe(&self, _: &mut (T, tsproto::connection::Connection), cmd: &InCommand) {
		actix::spawn(self.addr.send(WsMessage::Packet(cmd.into())).map_err(|_| {
			eprintln!("Failed to redirect packet to websocket connection");
		}));
	}
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

	// Open database
	let database = db::DbHandler::new(logger.clone(), &settings, key)?.start();

	// Open cache
	let file_cache = files::FileCache::new(settings.cache_path.clone())?.start();

	// Start sound
	let audio_data = audio::start(logger.clone())?;

	let addr = settings.listen_address.clone();
	server::new(move || {
		let logger2 = logger.clone();
		let audio_data2 = audio_data.clone();
		let settings = settings.clone();
		let database = database.clone();
		let file_cache = file_cache.clone();
		let mut app = App::new()
			.middleware(middleware::Logger::default())
			.resource("/ws", |r| r.f(move |req| ws::start(req, Ws::new(
				logger2.clone(),
				audio_data2.clone(),
				settings.clone(),
				database.clone(),
				file_cache.clone(),
			))));

		let addr = audio_data.a2ts.clone();
		let addr2 = audio_data.a2ts.clone();
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
		});
		app = app.handler("/", StaticFiles::new("../frontend/target/wasm32-unknown-unknown/release/")
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
