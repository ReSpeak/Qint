#![feature(async_await)]
use std::sync::atomic::{AtomicU64, Ordering};

use actix::*;
use actix::fut::wrap_future;
use actix_web::*;
use actix_web::fs::StaticFiles;
use futures01::sink::Sink as _;
use futures01::stream::Stream;
use futures01::Future as _;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use slog::{error, o, Drain, Logger};
use structopt::clap::AppSettings;
use structopt::StructOpt;
use tsclientlib::{Connection, PacketHandler, PHBox};
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::{InAudio, InCommand};

mod audio;
use audio::webrtc;

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
		default_value = "127.0.0.1:4422",
		help = "The address where the server listenes"
	)]
	address: String,
	#[structopt(
		long = "webrtc",
		help = "Use webrtc for sound"
	)]
	webrtc: bool,
	#[structopt(
		short = "v",
		long = "verbose",
		help = "Print the content of all packets",
		parse(from_occurrences)
	)]
	verbose: u8,
	// 0. Print nothing
	// 1. Print command string
	// 2. Print packets
	// 3. Print udp packets
}

/// Define http actor
struct Ws {
	logger: Logger,
	id: ConnectionId,
	/// If the audio data is `None`, webrtc should be used
	audio_data: Option<audio::AudioData>,
	rtc: Option<Addr<webrtc::WebrtcHandler>>,
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
		audio_data: Option<audio::AudioData>,
	) -> Self
	{
		Self {
			logger,
			id: next_con_id(),
			audio_data,
			rtc: None,
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
						// Setup webrtc
						if self.audio_data.is_none() && self.rtc.is_none() {
							let (data, rtc) = match crate::audio::start(
								self.logger.clone(),
								Some(ctx.address()),
							) {
								Ok(r) => r,
								Err(e) => {
									error!(self.logger, "Failed to start audio"; "error" => ?e);
									ctx.terminate();
									return;
								}
							};
							self.audio_data = Some(data);
							self.rtc = rtc;
						}

						let ts2a = self.audio_data.as_ref().unwrap().ts2a.clone();
						let con = self.id;
						let logger = self.logger.clone();
						let addr = ctx.address();
						let con = Connection::new(tsclientlib::ConnectOptions::new(o.address)
							.name(o.name)
							.logger(logger.clone())
							.log_commands(o.log_commands)
							.log_packets(o.log_packets)
							.log_udp_packets(o.log_udp_packets)
							.prepare_client(Box::new(move |client| {
								client.lock().add_in_command_observer(
									"Qint".into(),
									Box::new(ProxyCommandObserver {
										addr: addr.clone(),
									}),
								);
							}))
							.handle_packets(Box::new(ProxyPacketHandler {
								logger,
								con,
								addr: ts2a.clone(),
							}))
						);

						let logger = self.logger.clone();
						let a2ts = self.audio_data.as_ref().unwrap().a2ts.clone();

						ctx.spawn(wrap_future(con).map_err(|_e, _actor, ctx| {
							Self::send_message(&MessageP2F::ConnectFailed(), ctx);
						}).map(move |con, actor: &mut Ws, _| {
							let c = con.clone();
							actor.connection = Some(con);

							// Activate audio
							// TODO Handle disconnect
							let logger2 = logger.clone();
							actix::spawn(a2ts.send(audio::audio_to_ts::SetListenerMsg {
								connection: c,
							}).map_err(move |e| {
								error!(logger2, "Failed to set listener"; "error" => ?e);
							}));
						}));
					}
					MessageF2P::SetTalking(talk) => {
						let logger = self.logger.clone();
						if let Some(audio_data) = &self.audio_data {
							actix::spawn(audio_data.a2ts.send(audio::audio_to_ts::SetPlayingMsg(talk))
								.then(move |r| {
									match r {
										Ok(Ok(())) => {}
										Err(e) => error!(logger, "Failed to set playing state"; "error" => ?e),
										Ok(Err(e)) => error!(logger, "Failed to set playing state"; "error" => ?e),
									}
									Ok(())
								}));
						}
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
					MessageF2P::Webrtc(msg) => {
						if let Some(rtc) = &self.rtc {
							let logger = self.logger.clone();
							actix::spawn(rtc.send(webrtc::SignallingMsg(msg))
								.then(move |r| {
									match r {
										Ok(()) => {}
										Err(e) => error!(logger, "Failed with webrtc"; "error" => ?e),
									}
									Ok(())
								}));
						} else {
							error!(self.logger, "Got webrtc message but it is disabled");
						}
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
			Stream<Item = InCommand, Error = tsproto::Error> + Send,
		>,
		audio_stream: Box<
			Stream<Item = InAudio, Error = tsproto::Error> + Send,
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

fn main() {
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

	let audio_data = if args.webrtc {
		None
	} else {
		Some(audio::start(logger.clone(), None).unwrap().0)
	};

	server::new(move || {
		let logger = logger.clone();
		let audio_data = audio_data.clone();
		App::new()
		.middleware(middleware::Logger::default())
		.resource("/ws", |r| r.f(move |req| ws::start(req, Ws::new(
			logger.clone(),
			audio_data.clone(),
		))))
		.handler("/", StaticFiles::new("../target/wasm32-unknown-unknown/release")
			.expect("static files not found")
			.default_handler(StaticFiles::new("../frontend/static/")
				.expect("Static files not found")
				.index_file("index.html"))
		)
		.finish()
	})
		.bind(args.address)
		.unwrap()
		.run();
}
