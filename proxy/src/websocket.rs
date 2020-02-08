use std::sync::Arc;
use std::thread;

use actix::fut::wrap_future;
use actix::*;
use actix_web_actors::ws;
use failure::{format_err, Error};
use futures::channel::oneshot;
use futures::prelude::*;
use futures01::sink::Sink as _;
use futures01::stream::Stream;
use futures01::Future as _;
use qint_shared::{ChatId, ChatType, ConnectOptions, InCommandMsg, MessageF2P, MessageP2F};
use slog::{debug, error, o, warn, Logger};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tsclientlib::{ChannelId, Connection, Identity, PHBox, PacketHandler, TextMessageTargetMode};
use tsclientlib::messages::c2s::{self, InMessageTrait};
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::{Direction, Flags, InAudio, InCommand, PacketType};

use crate::{audio, db, ConnectionId, State};

/// Define http actor
pub(crate) struct Ws {
	logger: Logger,
	state: Arc<State>,
	id: ConnectionId,
	connection: Option<Connection>,
	pk_recv: Option<oneshot::Receiver<()>>,
}

#[derive(Clone)]
struct ProxyPacketHandler {
	logger: Logger,
	con: ConnectionId,
	addr: Addr<audio::ts_to_audio::TsToAudio>,
}

#[derive(Clone)]
struct ProxyCommandObserver {
	logger: Logger,
	addr: Addr<Ws>,
}

struct WsCommandMsg(InCommandMsg);

pub(crate) struct DownloadFile {
	pub channel: ChannelId,
	pub path: String,
}

struct DisconnectMsg;

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl Drop for Ws {
	fn drop(&mut self) {
		debug!(self.logger, "Removing connection");
	}
}

impl Message for WsCommandMsg {
	type Result = Result<(), ()>;
}

impl Message for DisconnectMsg {
	type Result = ();
}

impl Message for DownloadFile {
	/// The size of the file and the stream
	type Result = Result<(u64, TcpStream), Error>;
}

impl Ws {
	fn close(&mut self, ctx: &mut <Self as Actor>::Context) {
		self.state.connections.lock().unwrap().remove(&self.id);
		ctx.stop();

		// Spawn disconnect here in a tokio compat environment
		if let Some(con) = self.connection.as_ref().cloned() {
			thread::spawn(|| {
				tokio_compat::runtime::run(futures01::future::lazy(move || {
					con.disconnect(None).map_err(|_| ())
				}))
			});
		}
	}

	pub(crate) fn new(state: Arc<State>, id: ConnectionId) -> Self {
		let logger = state.logger.new(o!("id" => id.0.to_string()));
		debug!(logger, "Creating connection");
		Self { logger, state, id, connection: None, pk_recv: None }
	}

	fn send_message(msg: &MessageP2F, ctx: &mut ws::WebsocketContext<Self>) {
		ctx.binary(rmp_serde::to_vec(&msg).unwrap());
	}

	fn connect_intern(
		o: ConnectOptions,
		identity: Identity,
		actor: &mut Self,
		ctx: &mut ws::WebsocketContext<Self>,
	) -> Box<dyn Future<Output = Result<tsclientlib::Connection, Error>> + Unpin>
	{
		let addr = ctx.address();
		let addr2 = ctx.address();
		let db_addr = actor.state.database.clone();
		let db_addr2 = db_addr.clone();
		let logger = actor.logger.clone();
		let server_addr = o.address.clone();
		let logger2 = actor.logger.clone();
		let logger3 = actor.logger.clone();
		let options = tsclientlib::ConnectOptions::new(o.address)
			.name(o.name)
			.identity(identity)
			.logger(actor.logger.clone())
			.log_commands(o.log_commands || actor.state.settings.verbosity > 0)
			.log_packets(o.log_packets || actor.state.settings.verbosity > 1)
			.log_udp_packets(
				o.log_udp_packets || actor.state.settings.verbosity > 2,
			)
			.add_event_listener(
				"Qint".into(),
				Box::new(move |e| {
					let event = match e {
						tsclientlib::Event::ConEvents(con, events) => {
							db::EventMsg::Events(
								con.get_locked(),
								events.to_vec(),
							)
						}
						tsclientlib::Event::IdentityLevelIncreased(id) => {
							db::EventMsg::UpdateIdentity((*id).clone())
						}
						_ => return,
					};
					let logger = logger.clone();
					tokio::spawn(db_addr.send(event).map(move |r| match r {
						Ok(Ok(())) => {}
						Ok(Err(e)) => {
							error!(logger, "Failed to handle event in database"; "error" => ?e);
						}
						Err(_) => {
							error!(logger, "Failed to send event to database");
						}
					}));
				}),
			)
			.prepare_client(Box::new(move |client| {
				client.lock().add_in_command_observer(
					"Qint".into(),
					Box::new(ProxyCommandObserver {
						logger: logger3.clone(),
						addr: addr.clone(),
					}),
				);
			}))
			.handle_packets(Box::new(ProxyPacketHandler {
				logger: actor.logger.clone(),
				con: actor.id,
				addr: actor.state.audio_data.ts2a.clone(),
			}));

		let (send, recv) = oneshot::channel();

		thread::spawn(|| {
			tokio_compat::runtime::run(futures01::future::lazy(move || {
				Connection::new(options)
					.map(move |r| {
						r.add_on_disconnect(Box::new(move || {
							tokio::spawn(addr2.send(DisconnectMsg));
						}));

						let event =
							db::EventMsg::Connected(server_addr, r.clone());
						tokio::spawn(db_addr2.send(event).map(
							move |r| match r {
								Ok(Ok(())) => {}
								Ok(Err(e)) => {
									error!(logger2, "Failed to handle event in database"; "error" => ?e);
								}
								Err(_) => {
									error!(
										logger2,
										"Failed to send event to database"
									);
								}
							},
						));
						r
					})
					.from_err()
					.then(|r| {
						let _ = send.send(r);
						Ok(())
					})
			}))
		});

		let (pk_send, pk_recv) = oneshot::channel();
		actor.pk_recv = Some(pk_recv);

		let (send2, recv2) = oneshot::channel();
		ctx.spawn(wrap_future(recv.map(|r| r.unwrap()))
			.map(|r, _, ctx| {
				if let Ok(con) = &r {
					Self::send_message(&MessageP2F::ServerKey(con
						.get_server_key().unwrap().to_short().to_vec()), ctx);
					let _ = pk_send.send(());
				}
				let _ = send2.send(r);
			}));


		Box::new(recv2.map(Result::unwrap))
	}
}

impl Handler<WsCommandMsg> for Ws {
	type Result = Box<dyn ActorFuture<Output = Result<(), ()>, Actor = Self>>;
	fn handle(
		&mut self,
		WsCommandMsg(packet): WsCommandMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		// Block sending the initserver until the public key is sent
		if let Some(recv) = self.pk_recv.take() {
			let (send2, recv2) = oneshot::channel();
			self.pk_recv = Some(recv2);
			return Box::new(wrap_future(recv).map(|_, _, ctx| {
				Self::send_message(&MessageP2F::Packet(packet), ctx);
				let _ = send2.send(());
				Ok(())
			}));
		} else {
			Self::send_message(&MessageP2F::Packet(packet), ctx);
		}
		Box::new(wrap_future(future::ok(())))
	}
}

impl Handler<DisconnectMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self,
		_: DisconnectMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		self.close(ctx);
	}
}

impl Handler<DownloadFile> for Ws {
	type Result = ResponseFuture<Result<(u64, TcpStream), Error>>;
	fn handle(
		&mut self,
		msg: DownloadFile,
		_: &mut Self::Context,
	) -> Self::Result
	{
		if let Some(con) = &self.connection {
			let con = con.clone();
			let (send, recv) = oneshot::channel();

			thread::spawn(|| {
				tokio_compat::runtime::run(
					futures01::future::lazy(move || {
						con.download_file_token(
							msg.channel,
							&format!("/{}", msg.path),
							None,
							None,
						)
					})
					.then(|r| {
						let _ = send.send(r);
						Ok(())
					}),
				)
			});

			Box::pin(recv.map(|r| r.unwrap().map_err(|e| e.into())).then(
				|r| async {
					match r {
						Ok((token, size, addr)) => {
							let mut s = TcpStream::connect(&addr).await?;
							s.write_all(token.token.as_bytes()).await?;
							s.flush().await?;
							Ok((size, s))
						}
						Err(e) => Err(e),
					}
				},
			))
		} else {
			Box::pin(futures::future::err(format_err!(
				"Connection does not exist"
			)))
		}
	}
}

/// Handler for ws::Message message
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(
		&mut self,
		msg: Result<ws::Message, ws::ProtocolError>,
		ctx: &mut Self::Context,
	)
	{
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(text)) => ctx.text(text),
			Ok(ws::Message::Binary(bin)) => {
				let msg: MessageF2P =
					match rmp_serde::from_read_ref(bin.as_ref()) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Error deserializing message"; "error" => ?e);
							return;
						}
					};

				match msg {
					MessageF2P::Connect(o) => {
						let id = self.state.settings.default_identity;
						let address = o.address.clone();
						let username = o.name.clone();
						ctx.spawn(
							wrap_future(
								self.state
									.database
									.send(db::GetIdentityMsg(id, true))
									.map(|r| {
										r.map_err(|e| e.into()).and_then(|r| r)
									}),
							)
							.then(move |identity, actor: &mut Self, ctx| {
								match identity {
									Ok(id) => wrap_future(
										Self::connect_intern(o, id, actor, ctx),
									),
									Err(e) => {
										let fut: Box<
											dyn Future<
													Output = Result<
														tsclientlib::Connection,
														Error,
													>,
												> + Unpin,
										> = Box::new(futures::future::err(e));
										wrap_future(fut)
									}
								}
							})
							.map(|r, _, ctx| {
								if r.is_err() {
									Self::send_message(
										&MessageP2F::ConnectFailed(),
										ctx,
									);
								}
								r
							})
							.map(move |con, actor: &mut Self, _| {
								match con {
									Ok(con) => {
										let c = con.clone();
										let c2 = con.clone();
										actor.connection = Some(con);

										// Activate audio
										let logger = actor.logger.clone();
										let a2ts =
											actor.state.audio_data.a2ts.clone();
										actix::spawn(a2ts.send(audio::audio_to_ts::SetListenerMsg {
											connection: c,
										}).map(move |r| if let Err(e) = r {
											error!(logger, "Failed to set listener"; "error" => ?e);
										}));

										match c2.get_server_key() {
											Ok(server_key) => {
												// Save in database
												let logger =
													actor.logger.clone();
												actix::spawn(
													actor
														.state
														.database
														.send(
															db::ConnectedMsg {
																bookmark: None,
																username,
																address,
																channel: None,
																identity: id
																	as i64,
																server_key,
															},
														)
														.map(
															move |r| match r {
																Ok(Err(e)) => {
																	warn!(logger, "Failed to save connection in database"; "error" => ?e)
																}
																Err(e) => {
																	warn!(logger, "Failed to save connection in database"; "error" => ?e)
																}
																_ => {}
															},
														),
												);
											}
											Err(e) => {
												error!(actor.logger, "Failed to get server key"; "error" => ?e)
											}
										}
									}
									Err(e) => error!(actor.logger,
										"Failed to get identity for conection";
										"error" => ?e),
								}
							}),
						);
					}
					MessageF2P::Packet(packet) => {
						if let Some(con) = &mut self.connection {
							let sink = con.get_packet_sink();

							let (send, recv) = oneshot::channel();
							if packet.header().packet_type() == PacketType::Command && (packet.content().starts_with(b"sendtextmessage ") || packet.content().starts_with(b"clientpoke ")) {
								let logger = self.state.logger.clone();
								let command = InCommand::new(packet.content().to_vec(),
									packet.header().packet_type(), packet.header().flags().contains(Flags::NEWPROTOCOL), Direction::C2S).unwrap();

								let server = match con.get_server_key() {
									Ok(key) => key,
									Err(e) => {
										error!(logger, "Failed to get server key"; "error" => ?e);
										return;
									}
								};
								let server = server.to_short().to_vec();
								let invoker_uid = {
									let con = con.lock();
									if let Some(client) = con.clients.get(&con.own_client) {
										base64::decode(&client.uid.0).unwrap()
									} else {
										error!(logger, "Failed to get own client");
										return;
									}
								};

								let chat_type;
								let message;
								if packet.content().starts_with(b"sendtextmessage ") {
									let msg = c2s::InSendTextMessage::new(&command).unwrap();
									let msg = msg.iter().next().unwrap();
									message = msg.message.into();
									chat_type = match msg.target {
										TextMessageTargetMode::Server => ChatType::Server,
										TextMessageTargetMode::Channel => {
											let con = con.lock();
											let own_client = &con.clients[&con.own_client];
											ChatType::Channel(own_client.channel.0)
										}
										TextMessageTargetMode::Client => {
											let id = if let Some(id) = msg.target_client_id {
												id
											} else {
												error!(logger, "Invalid sendtextmessage to a client without client id");
												return;
											};
											let con = con.lock();
											let client = &con.clients[&id];
											ChatType::Client(base64::decode(&client.uid.0).unwrap())
										}
										TextMessageTargetMode::Unknown => {
											error!(logger, "Invalid sendtextmessage to a client with unknown target mode");
											return;
										}
									};
								} else {
									// Poke
									let msg = c2s::InClientPokeRequest::new(&command).unwrap();
									let msg = msg.iter().next().unwrap();
									message = msg.message.into();
									let con = con.lock();
									let client = &con.clients[&msg.client_id];
									chat_type = ChatType::Poke(base64::decode(&client.uid.0).unwrap());
								}

								let msg = db::WriteMessageMsg {
									message,
									invoker_uid,
									chat: ChatId {
										server,
										chat_type,
									},
								};
								tokio::spawn(self.state.database.send(msg).map(move |r| match r {
									Ok(Ok(())) => {}
									Ok(Err(e)) => {
										error!(logger, "Failed to handle event in database"; "error" => ?e);
									}
									Err(_) => {
										error!(logger, "Failed to send event to database");
									}
								}));
							}

							thread::spawn(|| {
								tokio_compat::runtime::run(
									futures01::future::lazy(move || {
										sink.send(packet)
									})
									.then(|r| {
										let _ = send.send(r);
										Ok(())
									}),
								)
							});

							ctx.spawn(wrap_future(recv.map(|r| {
								if let Err(e) = r.unwrap() {
									// TODO Return
									eprintln!("Failed to send packet: {:?}", e);
								}
							})));
						}
					}
					MessageF2P::Webrtc(_) => {
						// No webrtc
						error!(
							self.logger,
							"Got unsupported webrtc message, ignore it"
						);
					}
				}
			}
			Ok(ws::Message::Close(_)) => self.close(ctx),
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
	)
	{
		let logger = self.logger.clone();
		thread::spawn(|| {
			tokio_compat::runtime::run(
				command_stream.for_each(|_| Ok(())).then(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to handle packets"; "error" => ?e);
					}
					Ok(())
				}),
			)
		});

		let logger = self.logger.clone();
		let logger2 = self.logger.clone();
		let con = self.con;
		let addr = self.addr.clone();
		thread::spawn(move || {
			tokio_compat::runtime::current_thread::run(
				audio_stream
					.for_each(move |packet| {
						let logger = logger.clone();

						tokio::task::spawn_local(
							addr.send(audio::ts_to_audio::PlayMsg(con, packet))
								.map(move |r| match r {
									Ok(Ok(())) => {}
									Ok(Err(e)) => {
										error!(logger, "Failed to play audio packet"; "error" => ?e)
									}
									Err(e) => {
										error!(logger, "Failed to play audio packet"; "error" => ?e)
									}
								}),
						)
						.compat()
						.map(|_| ())
						.map_err(|e| {
							format_err!("Failed to spawn local future {:?}", e)
								.into()
						})
					})
					.then(move |r| {
						if let Err(e) = r {
							error!(logger2, "Failed to handle packets"; "error" => ?e);
						}
						Ok(())
					}),
			)
		});
	}

	/// Clone into a box.
	fn clone(&self) -> PHBox { Box::new(Clone::clone(self)) }
}

impl<T> InCommandObserver<T> for ProxyCommandObserver {
	fn observe(
		&self,
		_: &mut (T, tsproto::connection::Connection),
		cmd: &InCommand,
	)
	{
		let logger = self.logger.clone();
		tokio::spawn(self.addr.send(WsCommandMsg(cmd.into())).map(
			move |r| {
				if let Err(e) = r {
					error!(logger, "Failed to redirect packet to websocket connection";
					"error" => ?e);
				}
			},
		));
	}
}
