use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};
use std::thread;

use anyhow::{format_err, Result};
use actix::fut::wrap_future;
use actix::*;
use actix_web::web::Bytes;
use actix_web_actors::ws;
use futures::prelude::*;
use slog::{debug, error, o, warn, Logger};
use tokio::net::TcpStream;
use tsclientlib::{
	events, ChannelId, ClientId, Connection, Identity, TextMessageTargetMode,
};
use tsclientlib::StreamItem as TsStreamItem;
use tsclientlib::events::Event as TsEvent;

use crate::{audio, db, ConnectionId, State, WsFormat, WsOptions};
use crate::messages::{self, MessageF2P, MessageP2F};

/// A websocket connection
pub(crate) struct Ws {
	logger: Logger,
	state: Arc<State>,
	options: WsOptions,
	id: ConnectionId,
	connection: Option<Connection>,
	connect_options: Option<messages::ConnectOptions>,

	talkers: Vec<ClientId>,
	self_talking: bool,
}

/// Polls the connection for events.
struct ConnectionPoller;

pub(crate) struct SendMessageMsg(pub MessageP2F);
pub(crate) struct SetTalkersMsg(pub Vec<ClientId>);
pub(crate) struct SetSelfTalkingMsg(pub bool);

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
		// TODO Wait until disconnected if still connected
		//Running::Continue
		Running::Stop
	}
}

impl Message for SendMessageMsg {
	type Result = ();
}

impl Message for SetTalkersMsg {
	type Result = ();
}

impl Message for SetSelfTalkingMsg {
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
		logger: Logger,
		state: Arc<State>,
		options: WsOptions,
		id: ConnectionId,
	) -> Self
	{
		let logger = logger.new(o!("id" => id.0.to_string()));
		Self {
			logger,
			state,
			options,
			id,
			connection: None,
			connect_options: None,

			talkers: Default::default(),
			self_talking: false,
		}
	}

	fn update_talkers(&self, ctx: &mut <Self as Actor>::Context) {
		if let Some(con) = &self.connection.as_ref().and_then(|c| c.get_state().ok()) {
			let mut talkers = self.talkers.clone();
			if self.self_talking {
				talkers.push(con.own_client);
			}
			self.send_message(&MessageP2F::TalkersChanged(talkers), ctx);
		}
		self.talkers.clear();
		self.send_message(&MessageP2F::TalkersChanged(Vec::new()), ctx);
	}

	fn handle_event(&mut self, event: TsStreamItem, ctx: &mut <Self as Actor>::Context) {
		let event = match event {
			TsStreamItem::ConEvents(events) => {
				for e in &events {
					if let TsEvent::PropertyAdded { id: events::PropertyId::Server, .. } = e {
						// Connected
						// Activate audio
						let logger = self.logger.clone();
						let a2ts = self.state.audio_data.a2ts.clone();
						actix::spawn(
							a2ts.send(
								audio::audio_to_ts::SetListenerMsg {
									connection: ctx.address(),
								},
							)
							.map(move |r| {
								if let Err(e) = r {
									error!(logger, "Failed to set listener"; "error" => ?e);
								}
							}),
						);

						match self.connection.and_then(|c| c.get_server_key().ok()) {
							Some(server_key) => {
								// Save in database
								let logger = self.logger.clone();
								let opts = self.connect_options.as_ref().unwrap();
								// TODO What if this one doesn't exist?
								let id = self.state.settings.default_identity;
								actix::spawn(
									self
										.state
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
						if let Err(e) = db::DbHandler::handle_events(&self.state.database, &self.logger, con, data, &events) {
							error!(self.logger, "Database failed to handle events"; "error" => ?e);
						}
					}
				}
				return;
			}
			TsStreamItem::IdentityLevelIncreased => {
				if let Some(con) = &self.connection {
					db::UpdateIdentityMsg(con.get_options().get_identity().unwrap().clone())
				} else {
					return;
				}
			}
			_ => return,
		};
		let logger = self.logger.clone();
		actix::spawn(self.state.database.send(event).map(move |r| match r {
			Ok(Ok(())) => {}
			Ok(Err(e)) => {
				error!(logger, "Failed to handle event in database"; "error" => ?e);
			}
			Err(_) => {
				error!(logger, "Failed to send event to database");
			}
		}));
	}

	fn handle_audio(&mut self) {
		// TODO
		/*let logger = self.logger.clone();
		self.audio.send(audio::ts_to_audio::PlayMsg(self.id, packet))
			.map(move |r| match r {
				Ok(Ok(())) => {}
				Ok(Err(e)) => {
					error!(logger, "Failed to play audio packet"; "error" => ?e)
				}
				Err(e) => {
					error!(logger, "Failed to play audio packet"; "error" => ?e)
				}
			});*/
	}

	fn disconnect(&mut self, ctx: &mut <Self as Actor>::Context) {
		if let Some(con) = &mut self.connection {
			// TODO Send disconnect packet
		} else { // if websocket connected
			// TODO Disconnect websocket
		} // Else stop()
	}

	fn handle_ws_message(&mut self, msg: MessageF2P, ctx: &mut <Self as Actor>::Context) {
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
							.map(|r| r.map_err(|e| e.into()).and_then(|r| r)),
					)
					.map(move |identity, actor: &mut Self, ctx| identity.and_then(|id| {
						let options = tsclientlib::ConnectOptions::new(o.address)
							.name(o.name)
							.identity(id)
							.logger(actor.logger.clone())
							.log_commands(o.log_commands || actor.state.settings.verbosity > 0)
							.log_packets(o.log_packets || actor.state.settings.verbosity > 1)
							.log_udp_packets(
								o.log_udp_packets || actor.state.settings.verbosity > 2,
							);

						actor.connection = Some(Connection::new(options)?);
						ctx.spawn(ConnectionPoller);
						Ok(())
					}))
					.map(move |r, actor: &mut Self, ctx| {
						if let Err(e) = r {
							actor.send_message(&MessageP2F::Error(
								"Failed to connect".to_string()), ctx);
						}
					}),
				);
			}
			MessageF2P::SendMessage { target, message } => {
				if let Some(con) = &self.connection {
					let server = match con.get_server_key() {
						Ok(key) => key,
						Err(e) => {
							// TODO Return as error
							error!(self.logger, "Failed to get server key"; "error" => ?e);
							return;
						}
					};
					let server = server.to_short().to_vec();
					let invoker_uid = {
						if let Some(uid) = con.get_state().ok().and_then(|s|
							s.clients.get(&s.own_client)).and_then(|c| c.uid.as_ref()) {
							uid.0.clone()
						} else {
							error!(self.logger, "Failed to get own client");
							return;
						}
					};

					// TODO
					/*let msg = db::WriteMessageMsg {
						message,
						invoker_uid,
						chat: ChatId { server, chat_type },
					};
					let logger = self.logger.clone();
					actix::spawn(db.send(msg).map(move |r| match r {
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
					}));*/
				} else {
					// TODO Respond with error
				}
			}
		}
	}

	fn send_message(&self, msg: &MessageP2F, ctx: &mut <Self as Actor>::Context) {
		match self.options.format {
			WsFormat::Msgpack => ctx.binary(rmp_serde::to_vec(msg).unwrap()),
			WsFormat::Json => ctx.text(serde_json::to_string(msg).unwrap()),
		}
	}
}

impl Handler<DownloadFile> for Ws {
	type Result = ResponseFuture<Result<(u64, TcpStream)>>;
	fn handle(
		&mut self,
		msg: DownloadFile,
		_: &mut Self::Context,
	) -> Self::Result
	{
		if let Some(con) = &self.connection {
			let con = con.clone();
			let handle = con.download_file(
				msg.channel,
				&format!("/{}", msg.path),
				None,
				None,
			);
			Box::pin(futures::future::err(format_err!(
				"TODO"
			)))
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
		&mut self,
		msg: UploadFile,
		_: &mut Self::Context,
	) -> Self::Result
	{
		if let Some(con) = &self.connection {
			Box::pin(futures::future::err(format_err!(
				"TODO, not implemented"
			)))
		} else {
			Box::pin(futures::future::err(format_err!(
				"Connection does not exist"
			)))
		}
	}
}

impl Handler<SetTalkersMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self,
		SetTalkersMsg(talkers): SetTalkersMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		self.talkers = talkers;
		self.update_talkers(ctx);
	}
}

impl Handler<SetSelfTalkingMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self,
		SetSelfTalkingMsg(talking): SetSelfTalkingMsg,
		ctx: &mut Self::Context,
	) -> Self::Result
	{
		self.self_talking = talking;
		self.update_talkers(ctx);
	}
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(
		&mut self,
		msg: std::result::Result<ws::Message, ws::ProtocolError>,
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
				let msg: MessageF2P = match rmp_serde::from_read_ref(msg.as_ref()) {
					Ok(r) => r,
					Err(e) => {
						error!(self.logger, "Error msgpack deserializing message"; "error" => ?e);
						return;
					}
				};
				self.handle_ws_message(msg, ctx);
			}
			Ok(ws::Message::Close(_)) => self.disconnect(ctx),
			_ => {}
		}
	}
}

impl ActorFuture for ConnectionPoller {
	type Output = ();
	type Actor = Ws;
	fn poll(
		self: Pin<&mut Self>,
		actor: &mut Self::Actor,
		ctx: &mut <Self::Actor as Actor>::Context,
		task: &mut task::Context,
	) -> Poll<Self::Output> {
		if let Some(con) = &mut actor.connection {
			match con.events().poll_next_unpin(task) {
				Poll::Pending => Poll::Pending,
				Poll::Ready(None) => {
					actor.connection = None;
					actor.disconnect(ctx);
					Poll::Ready(())
				}
				Poll::Ready(Some(Err(e))) => {
					actor.connection = None;
					actor.send_message(&MessageP2F::Error(
						"Connection failed".to_string()), ctx);
					Poll::Ready(())
				}
				Poll::Ready(Some(Ok(item))) => {
					actor.handle_event(item, ctx);
					Poll::Pending
				}
			}
		} else {
			Poll::Ready(())
		}
	}
}
