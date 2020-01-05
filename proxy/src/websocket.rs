use std::sync::Arc;
use std::thread;

use actix::fut::wrap_future;
use actix::*;
use actix_web_actors::ws;
use failure::{format_err, Error};
use futures01::sink::Sink as _;
use futures01::stream::Stream;
use futures01::Future as _;
use futures::prelude::*;
use futures::channel::oneshot;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use qint_shared::ConnectOptions;
use serde::{Deserialize, Serialize};
use slog::{error, warn, Logger};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::{InAudio, InCommand};
use tsclientlib::{ChannelId, Connection, Identity, PacketHandler, PHBox};

use crate::{audio, db, ConnectionId, State};

/// Define http actor
pub(crate) struct Ws {
	state: Arc<State>,
	id: ConnectionId,
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

pub(crate) struct DownloadFile {
	pub channel: ChannelId,
	pub path: String,
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl Drop for Ws {
	fn drop(&mut self) {
		self.state.connections.lock().unwrap().remove(&self.id);

		// Spawn disconnect here in a tokio compat environment
		if let Some(con) = self.connection.as_ref().map(|c| c.clone()) {
			thread::spawn(|| tokio_compat::runtime::run(futures01::future::lazy(move || {
				con.disconnect(None).map_err(|_| ())
			})));
		}
	}
}

impl Message for WsMessage {
	type Result = ();
}

impl Message for DownloadFile {
	/// The size of the file and the stream
	type Result = Result<(u64, TcpStream), Error>;
}

impl Ws {
	pub(crate) fn new(
		state: Arc<State>,
		id: ConnectionId,
	) -> Self
	{
		Self {
			state,
			id,
			connection: None,
		}
	}

	fn send_message(msg: &MessageP2F, ctx: &mut ws::WebsocketContext<Self>) {
		let mut buf = Vec::new();
		let mut ser = Serializer::new(&mut buf);
		msg.serialize(&mut ser).unwrap();
		ctx.binary(buf);
	}

	fn connect_intern(o: ConnectOptions, identity: Identity, actor: &mut Self, ctx: &mut ws::WebsocketContext<Self>)
		-> Box<dyn Future<Output=Result<tsclientlib::Connection, Error>> + Unpin> {
		let addr = ctx.address();
		let db_addr = actor.state.database.clone();
		let db_addr2 = db_addr.clone();
		let logger = actor.state.logger.clone();
		let server_addr = o.address.clone();
		let logger2 = actor.state.logger.clone();
		let options = tsclientlib::ConnectOptions::new(o.address)
			.name(o.name)
			.identity(identity)
			.logger(actor.state.logger.clone())
			.log_commands(o.log_commands || actor.state.settings.verbosity > 0)
			.log_packets(o.log_packets || actor.state.settings.verbosity > 1)
			.log_udp_packets(o.log_udp_packets || actor.state.settings.verbosity > 2)
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
				tokio::spawn(db_addr.send(event)
					.map(move |r| {
						match r {
							Ok(Ok(())) => {}
							Ok(Err(e)) => {
								error!(logger, "Failed to handle event in database"; "error" => ?e);
							}
							Err(_) => {
								error!(logger, "Failed to send event to database");
							}
						}
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
				logger: actor.state.logger.clone(),
				con: actor.id,
				addr: actor.state.audio_data.ts2a.clone(),
			}));

		let (send, recv) = oneshot::channel();

		thread::spawn(|| tokio_compat::runtime::run(futures01::future::lazy(move || {
				Connection::new(options).map(move |r| {
				let event = db::EventMsg::Connected(server_addr, r.clone());
				tokio::spawn(db_addr2.send(event)
					.map(move |r| {
						match r {
							Ok(Ok(())) => {}
							Ok(Err(e)) => {
								error!(logger2, "Failed to handle event in database"; "error" => ?e);
							}
							Err(_) => {
								error!(logger2, "Failed to send event to database");
							}
						}
					}));
				r
			}).from_err().then(|r| {
				let _ = send.send(r);
				Ok(())
			})
		})));

		Box::new(recv.map(|r| r.unwrap()))
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

impl Handler<DownloadFile> for Ws {
	type Result = ResponseFuture<Result<(u64, TcpStream), Error>>;
	fn handle(&mut self, msg: DownloadFile, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &self.connection {
			let con = con.clone();
			let (send, recv) = oneshot::channel();

			thread::spawn(|| tokio_compat::runtime::run(futures01::future::lazy(move || {
				con.download_file_token(msg.channel, &format!("/{}", msg.path), None, None)
			}).then(|r| {
				let _ = send.send(r);
				Ok(())
			})));

			Box::pin(recv.map(|r| r.unwrap().map_err(|e| e.into()))
				.then(|r| async {
					match r {
						Ok((token, size, addr)) => {
							let mut s = TcpStream::connect(&addr).await?;
							s.write_all(token.token.as_bytes()).await?;
							s.flush().await?;
							Ok((size, s))
						}
						Err(e) => Err(e),
					}
				}))
		} else {
			Box::pin(futures::future::err(format_err!("Connection does not exist")))
		}
	}
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>,
		ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(text)) => ctx.text(text),
			Ok(ws::Message::Binary(bin)) => {
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
						let id = self.state.settings.default_identity;
						let address = o.address.clone();
						let username = o.name.clone();
						ctx.spawn(wrap_future(self.state.database.send(db::GetIdentityMsg(id, true))
								.map(|r| r.map_err(|e| e.into()).and_then(|r| r)))
							.then(move |identity, actor: &mut Self, ctx| {
								match identity {
									Ok(id) => wrap_future(Self::connect_intern(o, id, actor, ctx)),
									Err(e) => {
										let fut: Box<dyn Future<Output=Result<tsclientlib::Connection, Error>> + Unpin> = Box::new(futures::future::err(e));
										wrap_future(fut)
									}
								}
							}).map(|r, _, ctx| {
								if r.is_err() {
									Self::send_message(&MessageP2F::ConnectFailed(), ctx);
								}
								r
							}).map(move |con, actor: &mut Self, _| {
								match con {
									Ok(con) => {
										let c = con.clone();
										let c2 = con.clone();
										actor.connection = Some(con);

										// Activate audio
										// TODO Handle disconnect
										let logger = actor.state.logger.clone();
										let a2ts = actor.state.audio_data.a2ts.clone();
										actix::spawn(a2ts.send(audio::audio_to_ts::SetListenerMsg {
											connection: c,
										}).map(move |r| if let Err(e) = r {
											error!(logger, "Failed to set listener"; "error" => ?e);
										}));

										match c2.get_server_key() {
											Ok(server_key) => {
												// Save in database
												let logger = actor.state.logger.clone();
												actix::spawn(actor.state.database.send(db::ConnectedMsg {
													bookmark: None,
													username,
													address,
													channel: None,
													identity: id as i64,
													server_key,
												}).map(move |r| match r {
													Ok(Err(e)) => warn!(logger, "Failed to save connection in database"; "error" => ?e),
													Err(e) => warn!(logger, "Failed to save connection in database"; "error" => ?e),
													_ => {}
												}));
											}
											Err(e) => error!(actor.state.logger, "Failed to get server key"; "error" => ?e),
										}
									}
									Err(e) => error!(actor.state.logger,
										"Failed to get identity for conection";
										"error" => ?e),
								}
							}));
					}
					MessageF2P::SetTalking(talk) => {
						let logger = self.state.logger.clone();
						actix::spawn(self.state.audio_data.a2ts.send(audio::audio_to_ts::SetPlayingMsg(talk))
							.map(move |r| {
								match r {
									Ok(()) => {}
									Err(e) => error!(logger, "Failed to set playing state"; "error" => ?e),
								}
							}));
					}
					MessageF2P::Packet(packet) => {
						if let Some(con) = &mut self.connection {
							let sink = con.get_packet_sink();

							let (send, recv) = oneshot::channel();

							thread::spawn(|| tokio_compat::runtime::run(futures01::future::lazy(move || {
								sink.send(packet)
							}).then(|r| {
								let _ = send.send(r);
								Ok(())
							})));

							ctx.spawn(wrap_future(recv.map(|r| if let Err(e) = r.unwrap() {
								// TODO Return
								eprintln!("Failed to send packet: {:?}", e);
							})));
						}
					}
					MessageF2P::Webrtc(_) => {
						// No webrtc
						error!(self.state.logger, "Got unsupported webrtc message, ignore it");
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
		thread::spawn(|| tokio_compat::runtime::run(command_stream.for_each(|_| Ok(()))
			.then(move |r| {
				if let Err(e) = r {
					error!(logger, "Failed to handle packets"; "error" => ?e);
				}
				Ok(())
			})));

		let logger = self.logger.clone();
		let logger2 = self.logger.clone();
		let con = self.con;
		let addr = self.addr.clone();
		thread::spawn(move || {
			tokio_compat::runtime::current_thread::run(audio_stream.for_each(move |packet| {
				let logger = logger.clone();

				tokio::task::spawn_local(addr.send(audio::ts_to_audio::PlayMsg(con, packet))
					.map(move |r| {
						match r {
							Ok(Ok(())) => {}
							Ok(Err(e)) => error!(logger, "Failed to play audio packet"; "error" => ?e),
							Err(e) => error!(logger, "Failed to play audio packet"; "error" => ?e),
						}
					})).compat().map(|_| ()).map_err(|e| format_err!("Failed to spawn local future {:?}", e).into())
			}).then(move |r| {
				if let Err(e) = r {
					error!(logger2, "Failed to handle packets"; "error" => ?e);
				}
				Ok(())
			}))
		});
	}

	/// Clone into a box.
	fn clone(&self) -> PHBox {
		Box::new(Clone::clone(self))
	}
}

impl<T> InCommandObserver<T> for ProxyCommandObserver {
	fn observe(&self, _: &mut (T, tsproto::connection::Connection), cmd: &InCommand) {
		tokio::spawn(self.addr.send(WsMessage::Packet(cmd.into())).map(|r| {
			if r.is_err() {
				eprintln!("Failed to redirect packet to websocket connection");
			}
		}));
	}
}
