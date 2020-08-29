use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};

use actix::fut::wrap_future;
use actix::*;
use actix_web_actors::ws;
use anyhow::{bail, format_err, Error, Result};
use futures::prelude::*;
use slog::{debug, error, o, warn, Logger};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tsclientlib::events::Event as TsEvent;
use tsclientlib::prelude::*;
use tsclientlib::StreamItem as TsStreamItem;
use tsclientlib::{
	events, ChannelId, ClientId, Connection, DisconnectOptions, FileDownloadResult,
	FileTransferHandle, MessageTarget, Uid,
};
use tsproto::resend::PacketId;
use tsproto_packets::packets::{AudioData, OutPacket};

use crate::book_events::{ClientUpdate, JsM2B};
use crate::db::{ChatId, ChatType};
use crate::messages::{self, MessageF2P, MessageP2F};
use crate::{audio, book_events, db, ConnectionId, State, Tristate, WsFormat, WsOptions};

/// A websocket connection
pub(crate) struct Ws {
	logger: Logger,
	state: Arc<State>,
	options: WsOptions,
	id: ConnectionId,
	connection: Option<Connection>,
	connect_options: Option<messages::ConnectOptions>,
	file_downloads: HashMap<FileTransferHandle, oneshot::Sender<Result<FileDownloadResult>>>,

	websocket_closed: bool,
	self_talking: bool,
	talkers: Vec<(ClientId, bool)>,
}

/// Polls the connection for events.
struct ConnectionPoller;

pub(crate) struct GetClientVolumeMsg(pub ClientId);
/// Audio detection tells us if we are talking.
pub(crate) struct SetSelfTalkingMsg(pub bool);
pub(crate) struct TalkersChangedMsg(pub Vec<(ClientId, bool)>);
pub(crate) struct SaveClientMsg(pub Uid);
pub(crate) struct SendPacketMsg(pub OutPacket);
pub(crate) struct CaptureLoudnessMsg(pub f64);
pub(crate) struct SetVolumeMsg(pub Uid, pub f32);
#[derive(Clone)]
pub(crate) struct SetInputMutedMsg(pub Tristate);
#[derive(Clone)]
pub(crate) struct SetOutputMutedMsg(pub Tristate);
#[derive(Clone)]
pub(crate) struct SetAwayMsg(pub Tristate);
pub(crate) struct DisconnectMsg;

pub(crate) struct DownloadFile {
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

impl Message for GetClientVolumeMsg {
	type Result = Result<f32>;
}
impl Message for SetSelfTalkingMsg {
	type Result = ();
}
impl Message for TalkersChangedMsg {
	type Result = ();
}
impl Message for SaveClientMsg {
	type Result = Result<()>;
}
impl Message for SendPacketMsg {
	type Result = Result<PacketId>;
}
impl Message for CaptureLoudnessMsg {
	type Result = ();
}
impl Message for SetVolumeMsg {
	type Result = ();
}
impl Message for SetInputMutedMsg {
	type Result = Result<()>;
}
impl Message for SetOutputMutedMsg {
	type Result = Result<()>;
}
impl Message for SetAwayMsg {
	type Result = Result<()>;
}
impl Message for DisconnectMsg {
	type Result = ();
}
impl Message for DownloadFile {
	/// The size of the file and the stream
	type Result = Result<(u64, TcpStream, Uid)>;
}

impl Ws {
	pub fn new(logger: Logger, state: Arc<State>, options: WsOptions, id: ConnectionId) -> Self {
		let logger = logger.new(o!("id" => id.0.to_string()));
		Self {
			logger,
			state,
			options,
			id,
			connection: None,
			connect_options: None,
			file_downloads: Default::default(),

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

	fn send_to_audio<T: Message<Result = Result<()>> + Send + 'static>(&self, msg: T)
	where audio::ts_to_audio::TsToAudio: Handler<T> {
		if let Some(ad) = &self.state.audio_data {
			let logger = self.logger.clone();
			actix::spawn(ad.ts2a.send(msg).map(move |r| match r {
				Ok(Ok(())) => {}
				Ok(Err(e)) => {
					debug!(logger, "Audio error"; "error" => %e);
				}
				Err(_) => {
					warn!(logger, "Failed to send audio to handler");
				}
			}));
		}
	}

	fn handle_event(&mut self, event: TsStreamItem, ctx: &mut <Self as Actor>::Context) {
		match event {
			TsStreamItem::ConEvents(events) => {
				let mut connected_msg = None;
				for e in &events {
					if let TsEvent::PropertyAdded { id: events::PropertyId::Server, .. } = e {
						// Connected
						self.activate_audio(ctx);

						match self.connection.as_ref().and_then(|c| {
							c.get_server_key()
								.ok()
								.and_then(|s| c.get_state().map(|c| (s, c.own_client)).ok())
						}) {
							Some((server_key, own_client)) => {
								// Send server id
								let server = base64::encode(&server_key.to_short());
								self.send_message(
									&MessageP2F::Connected { server, own_client },
									ctx,
								);

								// Save in database
								let logger = self.logger.clone();
								let opts = self.connect_options.as_ref().unwrap();
								let id = self.state.settings.read().unwrap().default_identity;
								connected_msg = Some(db::ConnectedMsg {
									bookmark: None,
									username: opts.name.clone(),
									address: opts.address.clone(),
									channel: None,
									identity: id as i64,
									server_key,
								});
							}
							None => error!(self.logger, "Failed to get server key"),
						}
					} else if let TsEvent::ChannelListFinished = e {
						// Subscribe to all channels
						if let Some(con) = &mut self.connection {
							if let Ok(data) = con.get_state() {
								if let Err(e) = data.server.set_subscribed(true).send(con) {
									error!(self.logger, "Failed to subscribe \
										to server"; "error" => %e);
								}
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
							connected_msg,
						) {
							error!(self.logger, "Database failed to handle \
								events"; "error" => %e);
						}

						self.send_message(
							&MessageP2F::Events(
								events
									.into_iter()
									.filter_map(|e| book_events::convert_event(data, &e))
									.collect(),
							),
							ctx,
						);
					}
				}
			}
			TsStreamItem::Audio(audio) => {
				let from = ClientId(match audio.data().data() {
					AudioData::S2C { from, .. } => *from,
					AudioData::S2CWhisper { from, .. } => *from,
					_ => panic!("Can only handle S2C packets but got a C2S packet"),
				});
				let id = (self.id, from);
				self.send_to_audio(audio::ts_to_audio::PlayMsg(id, audio));
			}
			TsStreamItem::IdentityLevelIncreased => {
				if let Some(con) = &self.connection {
					let event =
						db::UpdateIdentityMsg(con.get_options().get_identity().unwrap().clone());
					let logger = self.logger.clone();
					actix::spawn(self.state.database.send(event).map(move |r| match r {
						Ok(Ok(())) => {}
						Ok(Err(e)) => {
							error!(logger, "Failed to handle event in database"; "error" => %e);
						}
						Err(_) => {
							error!(logger, "Failed to send event to database");
						}
					}));
				}
			}
			TsStreamItem::DisconnectedTemporarily => {
				self.deactivate_audio(ctx);
				self.send_message(&MessageP2F::DisconnectedTemporarily(), ctx);
				self.talkers.clear();
				self.update_talkers(ctx);
			}
			TsStreamItem::FileDownload(handle, file) => {
				if let Some(transfer) = self.file_downloads.remove(&handle) {
					let _ = transfer.send(Ok(file));
				}
			}
			TsStreamItem::FileTransferFailed(handle, e) => {
				if let Some(transfer) = self.file_downloads.remove(&handle) {
					let _ = transfer.send(Err(e.into()));
				}
			}
			_ => {}
		}
	}

	fn set_audio_active(active: bool, logger: Logger, a2ts: Addr<audio::audio_to_ts::AudioToTs>,
		ctx: &mut <Self as Actor>::Context) {
		if active {
			actix::spawn(
				a2ts.send(audio::audio_to_ts::AddListenerMsg(ctx.address()))
				.map(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to add audio listener"; "error" => %e);
					}
				})
			);
		} else {
			actix::spawn(
				a2ts.send(audio::audio_to_ts::RemoveListenerMsg(ctx.address()))
				.map(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to remove audio listener"; "error" => %e);
					}
				}),
			);
		}
	}

	fn activate_audio(&mut self, ctx: &mut <Self as Actor>::Context) {
		if let Some(ad) = &self.state.audio_data {
			Self::set_audio_active(true, self.logger.clone(), ad.a2ts.clone(), ctx);
		}
	}

	fn deactivate_audio(&mut self, ctx: &mut <Self as Actor>::Context) {
		if let Some(ad) = &self.state.audio_data {
			Self::set_audio_active(false, self.logger.clone(), ad.a2ts.clone(), ctx);
		}
	}

	fn disconnect(&mut self, ctx: &mut <Self as Actor>::Context) {
		self.deactivate_audio(ctx);
		self.talkers.clear();
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

	fn handle_ws_message(&mut self, msg: MessageF2P, ctx: &mut <Self as Actor>::Context) {
		match msg {
			MessageF2P::Connect(o) => {
				let id = self.state.settings.read().unwrap().default_identity;
				ctx.spawn(
					wrap_future(
						self.state
							.database
							.send(db::GetIdentityMsg(id, true))
							.map(|r| r.map_err(|e| e.into()).and_then(|r| r)),
					)
					.map(move |identity, actor: &mut Self, ctx| {
						identity.and_then(|id| {
							let settings = actor.state.settings.read().unwrap();
							let options = tsclientlib::ConnectOptions::new(o.address.clone())
								.name(o.name.clone())
								.identity(id)
								.version(o.version.clone())
								.logger(actor.logger.clone())
								.log_commands(o.log_commands || settings.verbosity > 0)
								.log_packets(o.log_packets || settings.verbosity > 1)
								.log_udp_packets(
									o.log_udp_packets || settings.verbosity > 2,
								);

							actor.connect_options = Some(o);
							actor.connection = Some(Connection::new(options)?);
							ctx.spawn(ConnectionPoller);
							Ok(())
						})
					})
					.map(move |r, actor: &mut Self, ctx| {
						if r.is_err() {
							actor.send_message(
								&MessageP2F::Error("Failed to connect".to_string()),
								ctx,
							);
						}
					}),
				);
			}
			MessageF2P::Disconnect(o) => {
				if let Some(con) = &mut self.connection {
					if let Err(e) = con.disconnect(o) {
						error!(self.logger, "Failed to disconnect";
							"error" => %e);
					}
				}
			}
			MessageF2P::SetLoudnessThreshold(threshold) => {
				// Save in settings
				self.state.modify_transient_settings(|settings| {
					settings.set_loudness_threshold(Some(threshold));
				});

				let logger = self.logger.clone();
				if let Some(ad) = &self.state.audio_data {
					actix::spawn(
						ad.a2ts.send(
							audio::audio_to_ts::SetLoudnessThresholdMsg(threshold))
						.map(move |r| {
							if let Err(e) = r {
								error!(logger, "Failed to apply loudness threshold"; "error" => %e);
							}
						})
					);
				}
			}
			MessageF2P::SubscribeLoudness(subscribe) => {
				if let Some(ad) = &self.state.audio_data {
					let logger = self.logger.clone();
					if subscribe {
						actix::spawn(
							ad.a2ts.send(
								audio::audio_to_ts::AddLoudnessListenerMsg(ctx.address()))
							.map(move |r| {
								if let Err(e) = r {
									error!(logger, "Failed to add loudness listener"; "error" => %e);
								}
							})
						);
					} else {
						actix::spawn(
							ad.a2ts.send(
								audio::audio_to_ts::RemoveLoudnessListenerMsg(ctx.address()))
							.map(move |r| {
								if let Err(e) = r {
									error!(logger, "Failed to remove loudness listener"; "error" => %e);
								}
							})
						);
					}
				}
			}
			MessageF2P::SendMessage { target, message } => {
				self.send_chat_message(target, message);
			}
			MessageF2P::Change(change) => {
				if let Some(con) = &mut self.connection {
					match con.get_state() {
						Err(e) => {
							error!(self.logger, "Failed to get state"; "error" => %e);
						}
						Ok(state) => {
							if let JsM2B::ClientUpdate(ClientUpdate { input_muted, output_muted, away, .. }) = &change {
								if input_muted.is_some() || output_muted.is_some() || away.is_some() {
									if let Some(client) = state.clients.get(&state.own_client) {

										let input_muted = input_muted.unwrap_or_else(|| client.input_muted);
										let output_muted = output_muted.unwrap_or_else(|| client.output_muted);
										let is_away = away.as_ref().map(|a| a.is_some())
											.unwrap_or_else(|| client.away_message.is_some());

										let audio_active = !input_muted && !output_muted && !is_away;
										if let Some(ad) = &self.state.audio_data {
											Self::set_audio_active(audio_active,
												self.logger.clone(),
												ad.a2ts.clone(),
												ctx);
										}
									}
								}
							}

							if let Err(e) = change.to_packet(state)
								.and_then(|p| p.send(con).map_err(|e| e.into())) {
								error!(self.logger, "Failed to send change"; "error" => %e);
							}
						}
					}
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

	fn send_chat_message(&mut self, target: MessageTarget, message: String) {
		if let Some(con) = &mut self.connection {
			match con.get_state() {
				Err(e) => {
					error!(self.logger, "Failed to get state"; "error" => %e);
				}
				Ok(state) => {
					if let Err(e) = state.send_message(target, &message).send(con) {
						error!(self.logger, "Failed to send message"; "error" => %e);
					}
				}
			}

			let server = match con.get_server_key() {
				Ok(key) => key,
				Err(e) => {
					// TODO Return as error
					error!(self.logger, "Failed to get server key"; "error" => %e);
					return;
				}
			};

			if let Ok(state) = con.get_state() {
				let server = server.to_short().to_vec();
				let own_channel;
				let invoker_uid = {
					if let Some(own_client) = state.clients.get(&state.own_client) {
						own_channel = own_client.channel.0;
						if let Some(uid) = own_client.uid.as_ref() {
							uid.0.clone()
						} else {
							error!(self.logger, "Failed to get own client uid");
							return;
						}
					} else {
						error!(self.logger, "Failed to get own client");
						return;
					}
				};

				let chat_type = match target {
					MessageTarget::Server => ChatType::Server,
					MessageTarget::Channel => ChatType::Channel(own_channel),
					MessageTarget::Client(id) | MessageTarget::Poke(id) => {
						let uid = &state.clients.get(&id).and_then(|c| c.uid.as_ref());
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
				actix::spawn(self.state.database.send(msg).map(move |r| match r {
					Ok(Ok(())) => {}
					Ok(Err(e)) => {
						error!(logger, "Failed to handle event in database"; "error" => %e);
					}
					Err(_) => {
						error!(logger, "Failed to send event to database");
					}
				}));
			} else {
				error!(self.logger, "Failed to get connection state");
			}
		} else {
			// TODO Respond with error
		}
	}
}

impl Handler<DownloadFile> for Ws {
	type Result = ResponseFuture<Result<(u64, TcpStream, Uid)>>;
	fn handle(&mut self, msg: DownloadFile, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let uid = match con
				.get_server_key()
				.map_err(Error::from)
				.and_then(|k| k.get_uid_no_base64().map_err(|e| e.into()))
			{
				Ok(k) => Uid(k),
				Err(e) => {
					return Box::pin(futures::future::err(format_err!("Failed to get uid: {}", e)));
				}
			};

			let handle = match con.download_file(msg.channel, &format!("/{}", msg.path), None, None)
			{
				Ok(r) => r,
				Err(e) => {
					return Box::pin(futures::future::err(format_err!(
						"Failed to download file: {}",
						e
					)));
				}
			};
			let (send, recv) = oneshot::channel();
			self.file_downloads.insert(handle, send);
			Box::pin(recv.map(|r| match r {
				Ok(Ok(r)) => Ok((r.size, r.stream, uid)),
				Ok(Err(e)) => Err(e),
				Err(e) => Err(e.into()),
			}))
		} else {
			Box::pin(futures::future::err(format_err!("Connection does not exist")))
		}
	}
}

impl Handler<GetClientVolumeMsg> for Ws {
	type Result = ActorResponse<Self, f32, Error>;
	fn handle(
		&mut self, GetClientVolumeMsg(client): GetClientVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &self.connection {
			match con.get_state() {
				Ok(state) => {
					let uid_fut: Box<dyn ActorFuture<Actor = Self, Output = Result<Vec<u8>>>>;
					if let Some(client) = state.clients.get(&client) {
						uid_fut = Box::new(wrap_future(future::ready(
							client
								.uid
								.as_ref()
								.map(|u| u.0.clone())
								.ok_or_else(|| format_err!("Client has no uid")),
						)));
					} else {
						// TODO Get uid from server
						uid_fut =
							Box::new(wrap_future(future::err(format_err!("Not yet implemented"))));
					}
					// Get volume from db
					ActorResponse::r#async(
						uid_fut
							.then(|uid, this, _| match uid {
								Ok(uid) => wrap_future(
									this.state
										.database
										.send(db::GetClientVolumeMsg(uid))
										.map_err(|e| e.into())
										.left_future(),
								),
								Err(e) => wrap_future(future::err(e).right_future()),
							})
							.map(|res, _, _| match res {
								Ok(Ok(Some(v))) => Ok(v),
								Ok(Ok(None)) => Ok(1.0),
								Ok(Err(e)) => Err(e),
								Err(e) => Err(e),
							}),
					)
				}
				Err(e) => ActorResponse::r#async(wrap_future(future::err(e.into()))),
			}
		} else {
			ActorResponse::r#async(wrap_future(future::err(format_err!(
				"Connection does not exist"
			))))
		}
	}
}

impl Handler<SetSelfTalkingMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, SetSelfTalkingMsg(talking): SetSelfTalkingMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		if self.self_talking != talking {
			self.self_talking = talking;
			self.update_talkers(ctx);
		}
	}
}

impl Handler<TalkersChangedMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, TalkersChangedMsg(talkers): TalkersChangedMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		if self.talkers != talkers {
			self.talkers = talkers;
			self.update_talkers(ctx);
		}
	}
}

impl Handler<SaveClientMsg> for Ws {
	type Result = Result<()>;
	fn handle(&mut self, SaveClientMsg(uid): SaveClientMsg, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let state = con.get_state()?;
			if let Some(client) = state.clients.values().find(|c| c.uid.as_ref() == Some(&uid)) {
				db::DbHandler::create_client(&self.state.database, &self.logger, con, state, client)
			} else {
				bail!("Client not found")
			}
		} else {
			bail!("Connection does not exist")
		}
	}
}

impl Handler<CaptureLoudnessMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, CaptureLoudnessMsg(loudness): CaptureLoudnessMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		self.send_message(&MessageP2F::Loudness(loudness), ctx);
	}
}

impl Handler<SendPacketMsg> for Ws {
	type Result = Result<PacketId>;
	fn handle(
		&mut self, SendPacketMsg(packet): SendPacketMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &mut self.connection {
			Ok(con.get_tsproto_client_mut()?.send_packet(packet)?)
		} else {
			bail!("Connection does not exist")
		}
	}
}

impl Handler<SetVolumeMsg> for Ws {
	type Result = ();
	fn handle(
		&mut self, SetVolumeMsg(client, volume): SetVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &self.connection {
			if let Ok(state) = con.get_state() {
				for c in state.clients.values() {
					if c.uid.as_ref() == Some(&client) {
						let id = (self.id, c.id);
						self.send_to_audio(audio::ts_to_audio::SetVolumeMsg(id, volume));
					}
				}
			} else {
				error!(self.logger, "Connection is not connected")
			}
		} else {
			error!(self.logger, "Connection does not exist")
		}
	}
}

impl Handler<SetInputMutedMsg> for Ws {
	type Result = Result<()>;
	fn handle(&mut self, SetInputMutedMsg(new): SetInputMutedMsg, ctx: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let state = con.get_state()?;
			let own_client = state.own_client;
			let old: bool = if let Some(own_client) = state.clients.get(&own_client) {
				own_client.input_muted
			} else {
				bail!("Failed to get own client");
			};
			let new_input_muted = new.get_value(old);
			state.client_update().set_input_muted(new_input_muted).send(con)?;
			if let Some(ad) = &self.state.audio_data {
				Self::set_audio_active(!new_input_muted,
					self.logger.clone(),
					ad.a2ts.clone(),
					ctx);
			}
		} else {
			bail!("Connection does not exist");
		}
		Ok(())
	}
}

impl Handler<SetOutputMutedMsg> for Ws {
	type Result = Result<()>;
	fn handle(&mut self, SetOutputMutedMsg(new): SetOutputMutedMsg, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let state = con.get_state()?;
			let own_client = state.own_client;
			let old: bool = if let Some(own_client) = state.clients.get(&own_client) {
				own_client.output_muted
			} else {
				bail!("Failed to get own client");
			};
			state.client_update().set_output_muted(new.get_value(old)).send(con)?;
		} else {
			bail!("Connection does not exist");
		}
		Ok(())
	}
}

impl Handler<SetAwayMsg> for Ws {
	type Result = Result<()>;
	fn handle(&mut self, SetAwayMsg(new): SetAwayMsg, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let state = con.get_state()?;
			let own_client = state.own_client;
			let old: bool = if let Some(own_client) = state.clients.get(&own_client) {
				own_client.away_message.is_some()
			} else {
				bail!("Failed to get own client");
			};
			state.client_update().set_away(if new.get_value(old) { Some("") } else { None }).send(con)?;
		} else {
			bail!("Connection does not exist");
		}
		Ok(())
	}
}

impl Handler<DisconnectMsg> for Ws {
	type Result = ();
	fn handle(&mut self, _: DisconnectMsg, ctx: &mut Self::Context) -> Self::Result {
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
						error!(self.logger, "Error json deserializing message"; "error" => %e);
						return;
					}
				};
				self.handle_ws_message(msg, ctx);
			}
			Ok(ws::Message::Binary(msg)) => {
				let msg: MessageF2P = match rmp_serde::from_read_ref(msg.as_ref()) {
					Ok(r) => r,
					Err(e) => {
						error!(self.logger, "Error msgpack deserializing message"; "error" => %e);
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
		self: Pin<&mut Self>, actor: &mut Self::Actor, ctx: &mut <Self::Actor as Actor>::Context,
		task: &mut task::Context,
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
					error!(actor.state.logger, "Connection failed"; "error" => %e);
					actor.connection = None;
					actor.send_message(&MessageP2F::Error("Connection failed".to_string()), ctx);
					break Poll::Ready(());
				}
				Poll::Ready(Some(Ok(item))) => {
					actor.handle_event(item, ctx);
				}
			}
		}
	}
}
