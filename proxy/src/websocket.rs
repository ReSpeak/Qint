use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::sync::mpsc::SyncSender;

use actix::fut::wrap_future;
use actix::*;
use actix_web::*;
use futures01::sink::Sink as _;
use futures01::stream::Stream;
use futures01::Future as _;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use slog::{error, Logger};
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::{InAudio, InCommand};
use tsclientlib::{Connection, PacketHandler, PHBox};

use crate::{audio, db, files, ConnectionId, Settings};

/// Define http actor
pub(crate) struct Ws {
	logger: Logger,
	settings: Settings,
	database: Addr<db::DbHandler>,
	file_cache: Addr<files::FileCache>,
	id: ConnectionId,
	audio_data: audio::AudioData,
	connection: Option<Connection>,
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
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

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl Drop for Ws {
	fn drop(&mut self) {
		self.connections.lock().unwrap().remove(&self.id);
	}
}

impl Message for WsMessage {
	type Result = ();
}

impl Ws {
	pub(crate) fn new(
		id: ConnectionId,
		logger: Logger,
		audio_data: audio::AudioData,
		settings: Settings,
		database: Addr<db::DbHandler>,
		file_cache: Addr<files::FileCache>,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<Ws>>>>,
	) -> Self
	{
		Self {
			logger,
			settings,
			database,
			file_cache,
			id,
			audio_data,
			connection: None,
			connections,
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
