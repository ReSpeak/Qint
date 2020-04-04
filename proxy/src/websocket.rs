use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};

use actix::fut::wrap_future;
use actix::*;
use actix_web_actors::ws;
use anyhow::{bail, format_err, Result};
use futures::prelude::*;
use slog::{debug, error, o, warn, Logger};
use tokio::net::TcpStream;
use tsclientlib::events::Event as TsEvent;
use tsclientlib::StreamItem as TsStreamItem;
use tsclientlib::{
	events, ChannelId, ClientId, Connection, DisconnectOptions, MessageTarget,
};
use tsproto::resend::PacketId;
use tsproto_packets::packets::{AudioData, OutPacket};

use crate::db::{ChatId, ChatType};
use crate::messages::{self, MessageF2P, MessageP2F};
use crate::{audio, db, ConnectionId, State, WsFormat, WsOptions};

/// A websocket connection
pub(crate) struct Ws {
	logger: Logger,
	state: Arc<State>,
	options: WsOptions,
	id: ConnectionId,
	connection: Option<Connection>,
	connect_options: Option<messages::ConnectOptions>,

	websocket_closed: bool,
	self_talking: bool,
	talkers: Vec<(ClientId, bool)>,
}

/// Polls the connection for events.
struct ConnectionPoller;

pub(crate) struct SendMessageMsg(pub MessageP2F);
pub(crate) struct SetSelfTalkingMsg(pub bool);
pub(crate) struct TalkersChangedMsg(pub Vec<(ClientId, bool)>);
pub(crate) struct SendPacketMsg(pub OutPacket);
pub(crate) struct DisconnectMsg;

pub(crate) struct DownloadFile {
	pub channel: ChannelId,
	pub path: String,
}

pub(crate) struct UploadFile {
	pub channel: ChannelId,
	pub path: String,
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;

	fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
		// Wait until disconnected if still connected
		if self.connection.is_some() {
			self.disconnect(ctx);
			Running::Continue
		} else {
			debug!(self.logger, "Stopping connection");
			Running::Stop
		}
	}

	fn stopped(&mut self, _: &mut Self::Context) {
		let mut cons = self.state.connections.lock().unwrap();
		cons.remove(&self.id);
	}
}

impl Message for SendMessageMsg {
	type Result = ();
}
impl Message for SetSelfTalkingMsg {
	type Result = ();
}
impl Message for TalkersChangedMsg {
	type Result = ();
}
impl Message for SendPacketMsg {
	type Result = Result<PacketId>;
}
impl Message for DisconnectMsg {
	type Result = ();
}
impl Message for DownloadFile {
	/// The size of the file and the stream
	type Result = Result<(u64, TcpStream)>;
}
impl Message for UploadFile {
	type Result = Result<TcpStream>;
}

impl Ws {
	pub fn new(
		logger: Logger, state: Arc<State>, options: WsOptions, id: ConnectionId,
	) -> Self {
		let logger = logger.new(o!("id" => id.0.to_string()));
		Self {
			logger,
			state,
			options,
			id,
			connection: None,
			connect_options: None,

			websocket_closed: false,
			self_talking: false,
			talkers: Default::default(),
		}
	}

	fn update_talkers(&mut self, ctx: &mut <Self as Actor>::Context) {
		if let Some(con) = &self.connection {
			if let Ok(state) = con.get_state() {
				let mut talkers = self.talkers.clone();
				if self.self_talking {
					talkers.push((state.own_client, false));
				}
				self.send_message(&MessageP2F::TalkersChanged(talkers), ctx);
				return;
			}
		}
		self.send_message(&MessageP2F::TalkersChanged(Vec::new()), ctx);
	}

	fn handle_event(
		&mut self, event: TsStreamItem, ctx: &mut <Self as Actor>::Context,
	) {
		match event {
			TsStreamItem::ConEvents(events) => {
				for e in &events {
					if let TsEvent::PropertyAdded {
						id: events::PropertyId::Server,
						..
					} = e
					{
						// Connected
						// Activate audio
						let logger = self.logger.clone();
						let a2ts = self.state.audio_data.a2ts.clone();
						actix::spawn(
							a2ts.send(audio::audio_to_ts::SetListenerMsg {
								connection: ctx.address(),
							})
							.map(move |r| {
								if let Err(e) = r {
									error!(logger, "Failed to set listener"; "error" => ?e);
								}
							}),
						);

						match self
							.connection
							.as_ref()
							.and_then(|c| c.get_server_key().ok())
						{
							Some(server_key) => {
								// Save in database
								let logger = self.logger.clone();
								let opts =
									self.connect_options.as_ref().unwrap();
								// TODO What if this one doesn't exist?
								let id = self.state.settings.default_identity;
								actix::spawn(
									self.state
										.database
										.send(db::ConnectedMsg {
											bookmark: None,
											username: opts.name.clone(),
											address: opts.address.clone(),
											channel: None,
											identity: id as i64,
											server_key,
										})
										.map(move |r| match r {
											Ok(Err(e)) => {
												warn!(logger, "Failed to save connection in database"; "error" => ?e)
											}
											Err(e) => {
												warn!(logger, "Failed to save connection in database"; "error" => ?e)
											}
											_ => {}
										}),
								);
							}
							None => {
								error!(self.logger, "Failed to get server key")
							}
						}
					}
				}

				if let Some(con) = &self.connection {
					if let Ok(data) = con.get_state() {
						if let Err(e) = db::DbHandler::handle_events(
							&self.state.database,
							&self.logger,
							con,
							data,
							&events,
						) {
							error!(self.logger, "Database failed to handle events"; "error" => ?e);
						}
					}
				}
			}
			TsStreamItem::Audio(audio) => {
				let from = ClientId(match audio.data().data() {
					AudioData::S2C { from, .. } => *from,
					AudioData::S2CWhisper { from, .. } => *from,
					_ => panic!(
						"Can only handle S2C packets but got a C2S packet"
					),
				});
				let id = (self.id, from);
				let logger = self.logger.clone();
				actix::spawn(
					self.state
						.audio_data
						.ts2a
						.send(audio::ts_to_audio::PlayMsg(id, audio))
						.map(move |r| match r {
							Ok(Ok(())) => {}
							Ok(Err(e)) => {
								debug!(logger, "Audio error"; "error" => %e);
							}
							Err(_) => {
								warn!(
									logger,
									"Failed to send audio to handler"
								);
							}
						}),
				);
			}
			TsStreamItem::IdentityLevelIncreased => {
				if let Some(con) = &self.connection {
					let event = db::UpdateIdentityMsg(
						con.get_options().get_identity().unwrap().clone(),
					);
					let logger = self.logger.clone();
					actix::spawn(self.state.database.send(event).map(
						move |r| match r {
							Ok(Ok(())) => {}
							Ok(Err(e)) => {
								error!(logger, "Failed to handle event in database"; "error" => ?e);
							}
							Err(_) => {
								error!(
									logger,
									"Failed to send event to database"
								);
							}
						},
					));
				}
			}
			_ => {}
		}
	}

	fn disconnect(&mut self, ctx: &mut <Self as Actor>::Context) {
		if let Some(con) = &mut self.connection {
			if let Err(e) = con.disconnect(DisconnectOptions::new()) {
				warn!(self.logger, "Failed to disconnect properly";
					"error" => %e);
				self.connection = None;
				self.disconnect(ctx);
			}
		} else if !self.websocket_closed {
			debug!(self.logger, "Closing websocket");
			ctx.close(None);
		} else {
			ctx.stop();
		}
	}

	fn handle_ws_message(
		&mut self, msg: MessageF2P, ctx: &mut <Self as Actor>::Context,
	) {
		match msg {
			MessageF2P::Connect(o) => {
				let id = self.state.settings.default_identity;
				ctx.spawn(
					wrap_future(
						self.state
							.database
							.send(db::GetIdentityMsg(id, true))
							.map(|r| r.map_err(|e| e.into()).and_then(|r| r)),
					)
					.map(move |identity, actor: &mut Self, ctx| {
						identity.and_then(|id| {
							let options = tsclientlib::ConnectOptions::new(
								o.address.clone(),
							)
							.name(o.name.clone())
							.identity(id)
							.version(o.version.clone())
							.logger(actor.logger.clone())
							.log_commands(
								o.log_commands
									|| actor.state.settings.verbosity > 0,
							)
							.log_packets(
								o.log_packets
									|| actor.state.settings.verbosity > 1,
							)
							.log_udp_packets(
								o.log_udp_packets
									|| actor.state.settings.verbosity > 2,
							);

							actor.connect_options = Some(o);
							actor.connection = Some(Connection::new(options)?);
							ctx.spawn(ConnectionPoller);
							Ok(())
						})
					})
					.map(move |r, actor: &mut Self, ctx| {
						if let Err(_) = r {
							actor.send_message(
								&MessageP2F::Error(
									"Failed to connect".to_string(),
								),
								ctx,
							);
						}
					}),
				);
			}
			MessageF2P::SendMessage { target, message } => {
				if let Some(con) = &mut self.connection {
					match con.get_mut_state() {
						Err(e) => {
							error!(self.logger, "Failed to get state"; "error" => ?e);
						}
						Ok(mut state) => {
							if let Err(e) = state.send_message(target, &message)
							{
								error!(self.logger, "Failed to send message"; "error" => ?e);
							}
						}
					}

					let server = match con.get_server_key() {
						Ok(key) => key,
						Err(e) => {
							// TODO Return as error
							error!(self.logger, "Failed to get server key"; "error" => ?e);
							return;
						}
					};

					if let Ok(state) = con.get_state() {
						let server = server.to_short().to_vec();
						let own_channel;
						let invoker_uid = {
							if let Some(own_client) =
								state.clients.get(&state.own_client)
							{
								own_channel = own_client.channel.0;
								if let Some(uid) = own_client.uid.as_ref() {
									uid.0.clone()
								} else {
									error!(
										self.logger,
										"Failed to get own client uid"
									);
									return;
								}
							} else {
								error!(self.logger, "Failed to get own client");
								return;
							}
						};

						let chat_type = match target {
							MessageTarget::Server => ChatType::Server,
							MessageTarget::Channel => {
								ChatType::Channel(own_channel)
							}
							MessageTarget::Client(id)
							| MessageTarget::Poke(id) => {
								let uid = &state
									.clients
									.get(&id)
									.and_then(|c| c.uid.as_ref());
								if let Some(uid) = uid {
									if let MessageTarget::Client(_) = target {
										ChatType::Client(uid.0.clone())
									} else {
										ChatType::Poke(uid.0.clone())
									}
								} else {
									error!(self.logger, "Failed to get uid of client"; "client" => ?id);
									return;
								}
							}
						};

						let msg = db::WriteMessageMsg {
							message,
							invoker_uid,
							chat: ChatId { server, chat_type },
						};
						let logger = self.logger.clone();
						actix::spawn(self.state.database.send(msg).map(
							move |r| match r {
								Ok(Ok(())) => {}
								Ok(Err(e)) => {
									error!(logger, "Failed to handle event in database"; "error" => ?e);
								}
								Err(_) => {
									error!(
										logger,
										"Failed to send event to database"
									);
								}
							},
						));
					} else {
						error!(self.logger, "Failed to get connection state");
					}
				} else {
					// TODO Respond with error
				}
			}
		}
	}

	fn send_message(
		&self, msg: &MessageP2F, ctx: &mut <Self as Actor>::Context,
	) {
		match self.options.format {
			WsFormat::Msgpack => ctx.binary(rmp_serde::to_vec(msg).unwrap()),
			WsFormat::Json => ctx.text(serde_json::to_string(msg).unwrap()),
		}
	}
}

impl Handler<DownloadFile> for Ws {
	type Result = ResponseFuture<Result<(u64, TcpStream)>>;
	fn handle(
		&mut self, msg: DownloadFile, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let handle = con.download_file(
				msg.channel,
				&format!("/{}", msg.path),
				None,
				None,
			);
			Box::pin(futures::future::err(format_err!("TODO")))
		} else {
			Box::pin(futures::future::err(format_err!(
				"Connection does not exist"
			)))
		}
	}
}

impl Handler<UploadFile> for Ws {
	type Result = ResponseFuture<Result<TcpStream>>;
	fn handle(
		&mut self, _msg: UploadFile, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(_con) = &self.connection {
			Box::pin(futures::future::err(format_err!("TODO, not implemented")))
		} else {
			Box::pin(futures::future::err(format_err!(
				"Connection does not exist"
			)))
		}
	}
}

impl Handler<SetSelfTalkingMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, SetSelfTalkingMsg(talking): SetSelfTalkingMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		if self.self_talking != talking {
			self.self_talking = talking;
			self.update_talkers(ctx);
		}
	}
}

impl Handler<TalkersChangedMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, TalkersChangedMsg(talkers): TalkersChangedMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		if self.talkers != talkers {
			self.talkers = talkers;
			self.update_talkers(ctx);
		}
	}
}

impl Handler<SendPacketMsg> for Ws {
	type Result = Result<PacketId>;
	fn handle(
		&mut self, SendPacketMsg(packet): SendPacketMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &mut self.connection {
			con.get_tsproto_client_mut()?.send_packet(packet)
		} else {
			bail!("Connection does not exist")
		}
	}
}

impl Handler<DisconnectMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, _: DisconnectMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		self.disconnect(ctx);
	}
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(
		&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>,
		ctx: &mut Self::Context,
	)
	{
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(msg)) => {
				let msg: MessageF2P = match serde_json::from_str(&msg) {
					Ok(r) => r,
					Err(e) => {
						error!(self.logger, "Error json deserializing message"; "error" => ?e);
						return;
					}
				};
				self.handle_ws_message(msg, ctx);
			}
			Ok(ws::Message::Binary(msg)) => {
				let msg: MessageF2P =
					match rmp_serde::from_read_ref(msg.as_ref()) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Error msgpack deserializing message"; "error" => ?e);
							return;
						}
					};
				self.handle_ws_message(msg, ctx);
			}
			Ok(ws::Message::Close(_)) => {
				debug!(self.logger, "Websocket closed");
				self.websocket_closed = true;
				self.disconnect(ctx);
			}
			_ => {}
		}
	}
}

impl ActorFuture for ConnectionPoller {
	type Output = ();
	type Actor = Ws;
	fn poll(
		self: Pin<&mut Self>, actor: &mut Self::Actor,
		ctx: &mut <Self::Actor as Actor>::Context, task: &mut task::Context,
	) -> Poll<Self::Output>
	{
		loop {
			let res = if let Some(con) = &mut actor.connection {
				con.events().poll_next_unpin(task)
			} else {
				break Poll::Ready(());
			};

			match res {
				Poll::Pending => break Poll::Pending,
				Poll::Ready(None) => {
					actor.connection = None;
					actor.disconnect(ctx);
					break Poll::Ready(());
				}
				Poll::Ready(Some(Err(e))) => {
					error!(actor.state.logger, "Connection failed"; "error" => ?e);
					actor.connection = None;
					actor.send_message(
						&MessageP2F::Error("Connection failed".to_string()),
						ctx,
					);
					break Poll::Ready(());
				}
				Poll::Ready(Some(Ok(item))) => {
					actor.handle_event(item, ctx);
				}
			}
		}
	}
}
