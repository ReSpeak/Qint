use std::collections::{HashMap, VecDeque};
use std::convert::TryInto;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{self, Poll};

use actix::fut::wrap_future;
use actix::*;
use anyhow::{bail, format_err, Result};
use futures::prelude::*;
use proxy_codegen::book_events::{self, JsM2B};
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tracing::{debug, error, info_span, warn, Span};
use tsclientlib::events::Event as TsEvent;
use tsclientlib::messages::s2c::InCommandErrorPart;
use tsclientlib::prelude::*;
use tsclientlib::StreamItem as TsStreamItem;
use tsclientlib::{
	events, AudioEvent, ChannelId, ClientId, Connection, ConnectionStats, DisconnectOptions,
	Error as TsclError, FileDownloadResult, FileUploadResult, FiletransferHandle, InMessage,
	MessageTarget, TsError, UidBuf,
};
use tsproto_packets::packets::{AudioData, CodecType, OutAudio, OutCommand, OutPacket};
use tsproto_types::crypto::EccKeyPubP256;

use crate::db::{ChannelListMsg, ChannelListTask, ChatId, ChatType, SetClientVolumeMsg};
use crate::messages::{self, MessageF2P, MessageP2F, ResultDetails, ResultStruct, WhisperData};
use crate::{audio, db, with_log, ConnectionId, FrontBridge, QintState};

type ReturnCodeListener = Box<dyn FnOnce(&mut QintConnection, &InCommandErrorPart)>;

const RETURN_CODE_PREFIX: &str = "proxy:";

/// A websocket connection
pub struct QintConnection {
	span: Span,
	pub id: ConnectionId,
	state: Arc<QintState>,
	sender: FrontBridge,
	connection: Option<Connection>,
	connect_options: Option<messages::ConnectOptions>,
	channel_list_finished_task: Option<ChannelListTask>,
	file_downloads: HashMap<FiletransferHandle, oneshot::Sender<Result<FileDownloadResult, Error>>>,
	file_uploads: HashMap<FiletransferHandle, oneshot::Sender<Result<FileUploadResult, Error>>>,
	return_codes: HashMap<String, ReturnCodeListener>,
	cur_return_code: u16,

	self_talking: bool,
	own_loudness: VecDeque<f64>,
	talkers: Vec<(ClientId, bool)>,
	whisper_list: Option<WhisperData>,
}

/// Polls the connection for events.
struct ConnectionPoller;

pub struct MessageF2PWrapper(pub MessageF2P);
pub struct GetPublicKeyMsg;
pub struct GetClientVolumeMsg(pub ClientId);
///detection tells us if we are talking.
pub struct SetSelfTalkingMsg(pub bool);
pub struct TalkersChangedMsg(pub Vec<(ClientId, bool)>);
pub struct LoudnessesMsg(pub HashMap<ClientId, f64>);
pub struct SendPacketMsg(pub OutPacket);
pub struct SendAudioMsg(pub CodecType, pub Vec<u8>);
pub struct CaptureLoudnessMsg(pub f64, pub f32); // (Loudness, Vad)
pub struct DisconnectMsg;
pub struct SetChannelListTaskMsg(pub ChannelListTask);
pub struct RunOnConMsg<R: 'static, F: FnOnce(&mut QintConnection) -> R>(pub F);

pub struct DownloadFile {
	pub channel: ChannelId,
	pub path: String,
	pub channel_password: Option<String>,
	pub resume: bool,
	pub return_code: Option<String>,
}
pub struct DownloadFileContext {
	pub size: u64,
	pub stream: TcpStream,
}

pub struct UploadFileContext {
	pub stream: TcpStream,
}

pub struct UploadFile {
	pub channel: ChannelId,
	pub path: String,
	pub channel_password: Option<String>,
	pub resume: bool,
	pub overwrite: bool,
	pub size: u64,
	pub return_code: Option<String>,
}

#[derive(Debug, Error)]
pub enum Error {
	#[error(transparent)]
	TsError(#[from] TsclError),
	#[error(transparent)]
	RecvError(#[from] tokio::sync::oneshot::error::RecvError),
	#[error("Connection does not exist")]
	NoConnection,
	#[error("Failed to get uid: {0}")]
	NoUid(#[source] TsclError),
}

impl Actor for QintConnection {
	type Context = actix::Context<Self>;

	fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
		// Wait until disconnected if still connected
		if self.connection.is_some() {
			self.disconnect(ctx);
			Running::Continue
		} else {
			debug!(parent: &self.span, "Stopping QintConnection");
			Running::Stop
		}
	}

	fn stopped(&mut self, _: &mut Self::Context) {
		let mut cons = self.state.connections.lock().unwrap();
		cons.remove(&self.id);
	}
}

impl Message for MessageF2PWrapper {
	type Result = ();
}
impl Message for GetPublicKeyMsg {
	type Result = Result<EccKeyPubP256>;
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
impl Message for LoudnessesMsg {
	type Result = ();
}
impl Message for SendPacketMsg {
	type Result = Result<()>;
}
impl Message for SendAudioMsg {
	type Result = Result<()>;
}
impl Message for CaptureLoudnessMsg {
	type Result = ();
}
impl Message for DisconnectMsg {
	type Result = ();
}
impl Message for SetChannelListTaskMsg {
	type Result = ();
}
impl Message for DownloadFile {
	/// The size of the file and the stream.
	type Result = Result<DownloadFileContext, Error>;
}
impl Message for UploadFile {
	type Result = Result<UploadFileContext, Error>;
}
impl<R: 'static, F: FnOnce(&mut QintConnection) -> R> Message for RunOnConMsg<R, F> {
	type Result = R;
}

impl QintConnection {
	pub fn new(state: Arc<QintState>, id: ConnectionId, sender: FrontBridge) -> Self {
		Self {
			span: info_span!("QintConnection", id = %id.0),
			state,
			id,
			sender,
			connection: None,
			connect_options: None,
			channel_list_finished_task: None,
			file_downloads: Default::default(),
			file_uploads: Default::default(),
			return_codes: Default::default(),
			cur_return_code: 0,

			self_talking: false,
			own_loudness: Default::default(),
			talkers: Default::default(),
			whisper_list: Default::default(),
		}
	}

	pub fn get_mut_connection(&mut self) -> Option<&mut tsclientlib::Connection> {
		self.connection.as_mut()
	}

	pub fn get_book(&self) -> Option<&tsclientlib::data::Connection> {
		self.connection.as_ref().and_then(|c| c.get_state().ok())
	}

	pub fn get_own_client(&self) -> Option<&tsclientlib::data::Client> {
		self.get_book().and_then(|b| b.clients.get(&b.own_client))
	}

	fn update_talkers(&mut self) {
		if let Some(state) = self.get_book() {
			let mut talkers = self.talkers.clone();
			if self.self_talking {
				talkers.push((state.own_client, false));
			}
			let talkers = talkers.into_iter().map(|(i, t)| (i.to_string(), t)).collect();
			self.send_message(&MessageP2F::TalkersChanged(talkers));
			return;
		}
		self.send_message(&MessageP2F::TalkersChanged(Vec::new()));
	}

	fn send_to_ts2a<T: Message<Result = Result<()>> + Send + 'static>(&self, msg: T)
	where audio::TsToAudio: Handler<T> {
		if let Some(ad) = &self.state.audio_data {
			actix::spawn(ad.ts2a.send(msg).map(move |r| match r {
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					debug!(%error, "Audio output error");
				}
				Err(_) => {
					warn!("Failed to send message to audio output handler");
				}
			}));
		}
	}

	fn send_to_a2ts<T: Message<Result = ()> + Send + 'static>(&self, msg: T)
	where audio::AudioToTs: Handler<T> {
		let _span = self.span.enter();
		if let Some(ad) = &self.state.audio_data {
			actix::spawn(with_log!(ad.a2ts.send(msg), "Failed to send audio to handler"));
		}
	}

	fn send_to_a2ts_r<R: Send + 'static, T: Message<Result = R> + Send + 'static>(&self, msg: T)
	where audio::AudioToTs: Handler<T> {
		let _span = self.span.enter();
		if let Some(ad) = &self.state.audio_data {
			actix::spawn(with_log!(
				ad.a2ts.send(msg),
				"Failed to send message to audio input handler"
			));
		}
	}

	fn handle_event(&mut self, event: TsStreamItem, ctx: &mut <Self as Actor>::Context) {
		let _span = self.span.clone().entered();
		match event {
			TsStreamItem::BookEvents(events) => {
				let mut connected_msg = None;
				for e in &events {
					if let TsEvent::PropertyAdded { id: events::PropertyId::Server, .. } = e {
						// Connected
						if let Some(return_code) =
							self.connect_options.as_mut().and_then(|o| o.return_code.take())
						{
							self.send_message(&MessageP2F::Result(ResultStruct {
								return_code,
								details: ResultDetails {
									ts_result: None,
									missing_permission: None,
									description: None,
								},
							}));
						}

						match self.connection.as_ref().and_then(|c| {
							c.get_server_key()
								.ok()
								.and_then(|s| c.get_state().map(|c| (s, c.own_client)).ok())
						}) {
							Some((server_key, own_client)) => {
								// Send server uid and own client id
								self.send_message(&MessageP2F::Connected {
									server: server_key.get_uid_no_base64(),
									own_client: own_client.to_string(),
								});

								// Save in database
								let opts = self.connect_options.as_ref().unwrap();
								let id = opts.identity_id.unwrap_or_else(|| {
									self.state.launch_config.read().unwrap().default_identity
								});
								connected_msg = Some(db::ConnectedMsg {
									bookmark: opts.bookmark.map(|i| i as i64),
									username: opts.name.clone(),
									address: opts.address.clone(),
									channel: opts.channel.clone(),
									password: opts.password.clone(),
									channel_password: opts.channel_password.clone(),
									identity: id as i64,
									server_key,
								});
							}
							None => error!("Failed to get server key"),
						}
					}
				}

				if let Some(con) = &self.connection {
					if let Ok(data) = con.get_state() {
						// Send to database
						if let Err(error) = db::DbHandler::handle_events(
							&self.state,
							con,
							data,
							&events,
							connected_msg,
							ctx.address(),
						) {
							error!(%error, "Database failed to handle events");
						}

						// Extend connection info packet for own client
						let msg = &MessageP2F::Events(
							events
								.into_iter()
								.filter_map(|e| {
									if let Some(mut e) = book_events::convert_event(data, &e) {
										use book_events::{JsEvent, JsProperty, JsPropertyId};

										if let JsEvent::PropertyChanged {
											id: JsPropertyId::ConnectionClientData(id),
											prop: JsProperty::ConnectionClientData(info),
											..
										} = &mut e
										{
											if let Some(state) = self.get_book() {
												if let Ok(stats) = con.get_network_stats() {
													if *id == state.own_client {
														Self::fill_connection_info(info, stats);
													}
												}
											}
										}

										Some(e)
									} else {
										warn!(event = ?e, "Event could not be converted for \
											frontend");
										None
									}
								})
								.collect(),
						);
						self.send_message(msg);
					}
				}
			}
			TsStreamItem::MessageEvent(msg) => {
				if let InMessage::ChannelListFinished(_) = msg {
					if let Some(con) = &mut self.connection {
						if let Ok(data) = con.get_state() {
							// Tell the database that all channels are now available
							if let Some(task) = self.channel_list_finished_task.take() {
								let msg = ChannelListMsg {
									current_channel: data
										.clients
										.get(&data.own_client)
										.map(|c| c.channel),
									task,
								};
								actix::spawn(self.state.database.send(msg).map(move |r| match r {
									Ok(Ok(())) => {}
									Ok(Err(error)) => {
										debug!(%error, "Failed to update bookmark");
									}
									Err(_) => {
										warn!("Failed to send message to database");
									}
								}));
							}

							if let Err(error) =
								db::DbHandler::handle_message(&self.state, con, data, &msg)
							{
								error!(%error, "Database failed to handle message");
							}

							// Subscribe to all channels
							if let Err(error) = data.server.set_subscribed(true).send(con) {
								error!(%error, "Failed to subscribe to server");
							}
						}
					}
				}

				// Convert errors by hand
				if let InMessage::CommandError(error) = &msg {
					for e in error.iter() {
						if let Some(return_code) = &e.return_code {
							if !return_code.starts_with(RETURN_CODE_PREFIX) {
								self.send_message(&MessageP2F::Result(ResultStruct {
									return_code: return_code.clone(),
									details: ResultDetails {
										ts_result: Some(e.id),
										missing_permission: e.missing_permission_id,
										description: None,
									},
								}));
							}

							if let Some(handler) = self.return_codes.remove(return_code) {
								handler(self, e);
							}
						}
					}
				} else if let Some(m) = book_events::convert_message(&msg) {
					self.send_message(&MessageP2F::Message(m));
				} else if !matches!(msg, InMessage::ClientNeededPermissions(_)) {
					warn!(message = ?msg, "Message could not be converted for frontend");
				}
			}
			TsStreamItem::Audio(audio) => {
				let from = ClientId(match audio.data().data() {
					AudioData::S2C { from, .. } => *from,
					AudioData::S2CWhisper { from, .. } => *from,
					_ => panic!("Can only handle S2C packets but got a C2S packet"),
				});
				let id = (self.id, from);
				self.send_to_ts2a(audio::ts_to_audio::PlayMsg(id, audio));
			}
			TsStreamItem::AudioChange(change) => match change {
				AudioEvent::CanSendAudio(can) => self.set_audio_input_active(ctx, can),
				AudioEvent::CanReceiveAudio(can) => self.set_audio_output_active(ctx, can),
			},
			TsStreamItem::IdentityLevelIncreased => {
				if let Some(con) = &self.connection {
					let mut update_identity = db::models::UpdateIdentity::default();
					let find_key = update_identity
						.from_identity_with_find(con.get_options().get_identity().unwrap());
					let event = db::UpdateIdentityMsg(find_key, update_identity);
					actix::spawn(self.state.database.send(event).map(move |r| match r {
						Ok(Ok(())) => {}
						Ok(Err(error)) => {
							error!(%error, "Failed to handle event in database");
						}
						Err(_) => {
							error!("Failed to send event to database");
						}
					}));
				}
			}
			TsStreamItem::DisconnectedTemporarily(_) => {
				self.send_message(&MessageP2F::DisconnectedTemporarily());
				self.talkers.clear();
				self.update_talkers();
			}
			TsStreamItem::FileDownload(handle, file) => {
				if let Some(transfer) = self.file_downloads.remove(&handle) {
					let _ = transfer.send(Ok(file));
				}
			}
			TsStreamItem::FileUpload(handle, file) => {
				if let Some(transfer) = self.file_uploads.remove(&handle) {
					let _ = transfer.send(Ok(file));
				}
			}
			TsStreamItem::FiletransferFailed(handle, e) => {
				if let Some(transfer) = self.file_downloads.remove(&handle) {
					let _ = transfer.send(Err(e.into()));
				} else if let Some(transfer) = self.file_uploads.remove(&handle) {
					let _ = transfer.send(Err(e.into()));
				}
			}
			TsStreamItem::NetworkStatsUpdated => {
				if let Some(con) = &self.connection {
					if let Ok(stats) = con.get_network_stats() {
						self.send_to_a2ts(audio::audio_to_ts::SetPacketlossMsg(
							stats.get_packetloss(),
						));
					}
				}
			}
			TsStreamItem::MessageResult(handle, res) => {
				self.send_message(&MessageP2F::Result(ResultStruct {
					return_code: handle.0.to_string(),
					details: res.into(),
				}));
			}
			_ => {}
		}
	}

	fn set_audio_input_active(&mut self, ctx: &mut <Self as Actor>::Context, active: bool) {
		if active {
			self.send_to_a2ts(audio::audio_to_ts::AddListenerMsg(ctx.address()))
		} else {
			self.send_to_a2ts_r(audio::audio_to_ts::RemoveListenerMsg(ctx.address()))
		}
	}

	fn set_audio_output_active(&mut self, _ctx: &mut <Self as Actor>::Context, _active: bool) {
		// TODO
	}

	fn fill_connection_info(
		info: &mut book_events::js_structs::ConnectionClientData, stats: &ConnectionStats,
	) {
		use tsclientlib::PacketStat;

		// connected_time is missing, we do not know that

		info.ping = Some(stats.rtt.try_into().ok());
		info.ping_deviation = Some(stats.rtt_dev.try_into().ok());

		info.packets_sent_speech =
			Some(Some(u64::from(stats.total_packets[PacketStat::OutSpeech as usize])));
		info.packets_sent_keepalive =
			Some(Some(u64::from(stats.total_packets[PacketStat::OutKeepalive as usize])));
		info.packets_sent_control =
			Some(Some(u64::from(stats.total_packets[PacketStat::OutControl as usize])));
		info.bytes_sent_speech =
			Some(Some(u64::from(stats.total_bytes[PacketStat::OutSpeech as usize])));
		info.bytes_sent_keepalive =
			Some(Some(u64::from(stats.total_bytes[PacketStat::OutKeepalive as usize])));
		info.bytes_sent_control =
			Some(Some(u64::from(stats.total_bytes[PacketStat::OutControl as usize])));

		info.packets_received_speech =
			Some(Some(u64::from(stats.total_packets[PacketStat::InSpeech as usize])));
		info.packets_received_keepalive =
			Some(Some(u64::from(stats.total_packets[PacketStat::InKeepalive as usize])));
		info.packets_received_control =
			Some(Some(u64::from(stats.total_packets[PacketStat::InControl as usize])));
		info.bytes_received_speech =
			Some(Some(u64::from(stats.total_bytes[PacketStat::InSpeech as usize])));
		info.bytes_received_keepalive =
			Some(Some(u64::from(stats.total_bytes[PacketStat::InKeepalive as usize])));
		info.bytes_received_control =
			Some(Some(u64::from(stats.total_bytes[PacketStat::InControl as usize])));

		let bandwidth_last_second = stats.get_last_second_bytes();
		info.bandwidth_sent_last_second_speech =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::OutSpeech as usize])));
		info.bandwidth_sent_last_second_keepalive =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::OutKeepalive as usize])));
		info.bandwidth_sent_last_second_control =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::OutControl as usize])));
		info.bandwidth_received_last_second_speech =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::InSpeech as usize])));
		info.bandwidth_received_last_second_keepalive =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::InKeepalive as usize])));
		info.bandwidth_received_last_second_control =
			Some(Some(u64::from(bandwidth_last_second[PacketStat::InControl as usize])));

		let bandwidth_last_minute = stats.get_last_second_bytes();
		info.bandwidth_sent_last_minute_speech =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::OutSpeech as usize])));
		info.bandwidth_sent_last_minute_keepalive =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::OutKeepalive as usize])));
		info.bandwidth_sent_last_minute_control =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::OutControl as usize])));
		info.bandwidth_received_last_minute_speech =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::InSpeech as usize])));
		info.bandwidth_received_last_minute_keepalive =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::InKeepalive as usize])));
		info.bandwidth_received_last_minute_control =
			Some(Some(u64::from(bandwidth_last_minute[PacketStat::InControl as usize])));

		info.server_to_client_packetloss_speech = Some(Some(stats.get_packetloss_s2c_speech()));
		info.server_to_client_packetloss_keepalive =
			Some(Some(stats.get_packetloss_s2c_keepalive()));
		info.server_to_client_packetloss_control = Some(Some(stats.get_packetloss_s2c_control()));
		info.server_to_client_packetloss_total = Some(Some(stats.get_packetloss_s2c_total()));
	}

	fn reset_connection(&mut self) {
		self.connection = None;
		self.channel_list_finished_task = None;
		self.file_downloads.clear();
		self.file_uploads.clear();
		self.return_codes.clear();
		self.cur_return_code = 0;

		self.self_talking = false;
		self.own_loudness.clear();
		self.talkers.clear();
		self.whisper_list = None;
		self.update_talkers();
	}

	fn disconnect(&mut self, ctx: &mut <Self as Actor>::Context) {
		let _span = self.span.clone().entered();
		self.set_audio_input_active(ctx, false);
		self.set_audio_output_active(ctx, false);
		self.talkers.clear();
		if let Some(con) = &mut self.connection {
			if con.get_state().is_ok() {
				debug!("Sending disconnect packet");
				if let Err(error) = con.disconnect(DisconnectOptions::new()) {
					warn!(%error, "Failed to disconnect properly");
					self.reset_connection();
				} else {
					// Wait until disconnected
					return;
				}
			} else {
				self.reset_connection();
			}
		}
		debug!("Disconnecting QintConnection");
		self.sender.close();
		ctx.stop();
	}

	/// Update channel password after successfully switching channels.
	fn handle_channel_move_result(&mut self, channel: ChannelId, password: Option<String>) {
		let _span = self.span.clone().entered();
		if let Some(state) = self.get_book() {
			let server = state.server.public_key.to_short();
			actix::spawn(
				self.state
					.database
					.send(db::RunOnDbMsg(move |db| {
						use diesel::prelude::*;

						use db::schema::channels;

						diesel::update(channels::table.filter(
							channels::server.eq(&server).and(channels::id.eq(channel.0 as i64)),
						))
						.set(channels::password.eq(password))
						.execute(&db.con)
					}))
					.map(move |r| match r {
						Ok(Ok(1)) => {}
						Ok(Ok(_)) => {
							error!(
								"Failed to update channel password in database, channel not found"
							);
						}
						Ok(Err(error)) => {
							error!(%error, "Failed to update channel password in database");
						}
						Err(error) => {
							error!(%error, "Failed to send channel password update to database");
						}
					}),
			);
		}
	}

	fn handle_ws_message(&mut self, msg: MessageF2P, ctx: &mut <Self as Actor>::Context) {
		let _span = self.span.clone().entered();
		match msg {
			MessageF2P::Connect(o) => {
				let identity_id = o
					.identity_id
					.unwrap_or_else(|| self.state.launch_config.read().unwrap().default_identity);
				ctx.spawn(
					wrap_future(
						self.state
							.database
							.send(db::GetIdentityAndServerMsg {
								id: identity_id,
								create: true,
								address: o.address.clone(),
							})
							.map(|r| r.map_err(|e| e.into()).and_then(|r| r)),
					)
					.map(move |res, actor: &mut Self, ctx| {
						res.and_then(|(identity, server)| {
							let launch_config = actor.state.launch_config.read().unwrap();
							let mut options = tsclientlib::Connection::build(o.address.clone())
								.name(o.name.clone())
								.identity(identity)
								.log_commands(o.log_commands || launch_config.verbosity > 0)
								.log_packets(o.log_packets || launch_config.verbosity > 1)
								.log_udp_packets(o.log_udp_packets || launch_config.verbosity > 2);

							macro_rules! opt {
								($name:ident) => {
									if let Some($name) = o.$name {
										options = options.$name($name);
									}
								};
								($name:ident, clone) => {
									if let Some($name) = &o.$name {
										options = options.$name($name.clone());
									}
								};
							}

							opt!(input_muted);
							opt!(input_hardware_enabled);
							opt!(output_muted);
							opt!(output_hardware_enabled);

							opt!(away, clone);
							opt!(version, clone);
							opt!(channel, clone);
							opt!(channel_password, clone);
							opt!(password, clone);
							if !o.ignore_identity_mismatch {
								if let Some(server) = server {
									options = options.server(server);
								}
							}

							actor.connect_options = Some(o);
							actor.connection = Some(options.connect()?);
							ctx.spawn(ConnectionPoller);
							Ok(())
						})
					})
					.map(move |r, actor: &mut Self, _| {
						if let Err(error) = r {
							warn!(%error, "Failed to connect");
							actor.send_message(&MessageP2F::Error(format!(
								"Failed to connect ({})",
								error
							)));
						}
					}),
				);
			}
			MessageF2P::Disconnect(o) => {
				if let Some(con) = &mut self.connection {
					if let Err(error) = con.disconnect(o) {
						error!(%error, "Failed to disconnect");
					}
				}
			}
			MessageF2P::SetClientVolume { client, volume } => {
				let client = UidBuf(client);

				if let Some(state) = self.get_book() {
					let mut created = false;
					for c in state.clients.values() {
						if c.uid.as_ref() == Some(&client) {
							let id = (self.id, c.id);
							self.send_to_ts2a(audio::ts_to_audio::SetVolumeMsg(id, volume));
							if !created {
								created = true;
								if let Err(error) = db::DbHandler::create_client(
									&self.state,
									self.connection.as_ref().unwrap(),
									state,
									c,
								) {
									error!(%error, "Failed to create client in database");
								}
							}
						}
					}
				} else {
					error!("Connection is not connected")
				}

				actix::spawn(self.state.database.send(SetClientVolumeMsg(client, volume)).map(
					move |r| match r {
						Ok(Ok(())) => {}
						Ok(Err(error)) => {
							error!(%error, "Failed to update volume in database");
						}
						Err(error) => {
							error!(%error, "Failed to send volume update to database");
						}
					},
				));
			}
			MessageF2P::SetWhispering(data) => {
				self.whisper_list = data;
			}
			MessageF2P::SendMessage { target, message, return_code } => {
				self.send_chat_message(target.into(), message, return_code.as_deref());
			}
			MessageF2P::SendCommand { command, return_code } => {
				self.send_command(command, return_code.as_deref())
			}
			MessageF2P::Change { mut change, return_code } => {
				if let Some(state) = self.get_book() {
					if let JsM2B::ConnectionClientUpdate(change) = &mut change {
						if let Some(client) = state.clients.get(&state.own_client) {
							let has_multiple_cons =
								self.state.connections.lock().unwrap().len() > 1;
							if let Some(c) = change.input_muted {
								if c {
									// Mute, change to disable if there is more than one
									// connection.
									if has_multiple_cons {
										change.input_muted = None;
										change.input_hardware_enabled = Some(false);
									}
								} else {
									// Unmute, also enable hardware if currently disabled
									if !client.input_hardware_enabled {
										change.input_hardware_enabled = Some(true);
									}
									if !client.input_muted {
										change.input_muted = None;
									}

									// Change all other muted servers to disabled
									self.for_other_connections(|con| {
										if let Some(client) = con.clients.get(&con.own_client) {
											if client.input_muted && client.input_hardware_enabled {
												return Some(
													con.client_update()
														.set_input_muted(false)
														.set_input_hardware_enabled(false),
												);
											}
										}
										None
									});
								}
							}

							if let Some(c) = change.output_muted {
								if c {
									// Mute, change to disable if there is more than one
									// connection.
									if has_multiple_cons {
										change.output_muted = None;
										change.output_hardware_enabled = Some(false);
									}
								} else {
									// Unmute, also enable hardware if currently disabled
									if !client.output_hardware_enabled {
										change.output_hardware_enabled = Some(true);
									}
									if !client.output_muted {
										change.output_muted = None;
									}

									// Change all other muted servers to disabled
									self.for_other_connections(|con| {
										if let Some(client) = con.clients.get(&con.own_client) {
											if client.output_muted && client.output_hardware_enabled
											{
												return Some(
													con.client_update()
														.set_output_muted(false)
														.set_output_hardware_enabled(false),
												);
											}
										}
										None
									});
								}
							}
						}
					} else if let JsM2B::ClientMove(move_change) = &mut change {
						let channel = move_change.channel;
						let password = move_change.password.clone();
						// Add a password if we have one saved
						if move_change.password.is_none() {
							let server = state.server.public_key.to_short();
							ctx.spawn(
								wrap_future(self.state.database.send(db::RunOnDbMsg(move |db| {
									use diesel::prelude::*;

									use db::schema::channels;

									channels::table
										.filter(
											channels::server
												.eq(&server)
												.and(channels::id.eq(channel.0 as i64)),
										)
										.select(channels::password)
										.first::<Option<String>>(&db.con)
								})))
								.map(move |r, actor: &mut Self, _| {
									let pw = match r {
										Ok(Ok(r)) => r,
										Ok(Err(error)) => {
											error!(%error, "Failed to query database for channel \
												password");
											None
										}
										Err(_) => {
											error!("Failed to query database for channel password");
											None
										}
									};
									if let JsM2B::ClientMove(change) = &mut change {
										change.password = pw;
									} else {
										error!("Unexpected message, should be a client move");
									}
									let _ = actor.send_change_with_result(
										change,
										return_code,
										Box::new(move |actor, res| {
											if res.id == TsError::Ok {
												actor.handle_channel_move_result(channel, password);
											}
										}),
									);
								}),
							);
							return;
						}

						let _ = self.send_change_with_result(
							change,
							return_code,
							Box::new(move |actor, res| {
								if res.id == TsError::Ok {
									actor.handle_channel_move_result(channel, password);
								}
							}),
						);
						return;
					}

					let _ = self.send_change(change, return_code);
				} else {
					self.send_error(return_code.as_deref(), format!("Failed to get state"));
				}
			}
		}
	}

	fn send_change(&mut self, change: JsM2B, return_code: Option<String>) -> Result<()> {
		let _span = self.span.clone().entered();
		if let Some(state) = self.get_book() {
			match change.to_packet(state) {
				Ok(msg) => self.send_ts_message(msg, return_code.as_deref()),
				Err(e) => {
					self.send_error(
						return_code.as_deref(),
						format!("Failed to create packet for change {:?}: {}", change, e),
					);
					bail!("Failed to create packet");
				}
			}
		} else {
			self.send_error(return_code.as_deref(), format!("Failed to get state"));
			bail!("Failed to get state");
		}
	}

	/// Send the message and execute the given function after the result is there.
	///
	/// If the `return_code` is `None`, a new one will be generated.
	///
	/// The `on_result` function will not be called if sending the message fails and this function
	/// returns an error.
	fn send_change_with_result(
		&mut self, change: JsM2B, return_code: Option<String>, on_result: ReturnCodeListener,
	) -> Result<()> {
		let return_code = return_code.unwrap_or_else(|| {
			let r = format!("{}{}", RETURN_CODE_PREFIX, self.cur_return_code);
			self.cur_return_code = self.cur_return_code.wrapping_add(1);
			r
		});

		let res = self.send_change(change, Some(return_code.clone()));
		if res.is_ok() {
			self.return_codes.insert(return_code, on_result);
		}

		res
	}

	fn send_message(&self, msg: &MessageP2F) { self.sender.send(msg) }

	fn send_error(&self, return_code: Option<&str>, error: String) {
		if let Some(code) = return_code {
			self.send_message(&MessageP2F::Result(ResultStruct {
				return_code: code.into(),
				details: ResultDetails {
					ts_result: None,
					missing_permission: None,
					description: Some(error),
				},
			}));
		} else {
			warn!(parent: &self.span, %error, "Proxy error");
		}
	}

	fn send_chat_message(
		&mut self, target: MessageTarget, message: String, return_code: Option<&str>,
	) {
		let _span = self.span.clone().entered();
		if let Some(state) = self.get_book() {
			let msg = state.send_message(target, &message);
			if self.send_ts_message(msg, return_code).is_err() {
				return;
			}
		} else {
			self.send_error(return_code.as_deref(), format!("Failed to get state"));
			return;
		}

		// Reborrow
		let con = self.connection.as_mut().unwrap();
		let server = match con.get_server_key() {
			Ok(key) => key,
			Err(e) => {
				self.send_error(return_code, format!("Failed to get server key: {}", e));
				return;
			}
		};

		// Reborrow
		if let Some(state) = self.get_book() {
			let own_channel;
			let invoker_uid = {
				if let Some(own_client) = state.clients.get(&state.own_client) {
					own_channel = own_client.channel.0;
					if let Some(uid) = own_client.uid.as_ref() {
						uid.clone()
					} else {
						self.send_error(return_code, "Failed to get own client uid".into());
						return;
					}
				} else {
					self.send_error(return_code, "Failed to get own client".into());
					return;
				}
			};

			let mut client_data = None;
			let chat_type = match target {
				MessageTarget::Server => ChatType::Server,
				MessageTarget::Channel => ChatType::Channel(own_channel),
				MessageTarget::Client(id) | MessageTarget::Poke(id) => {
					let client = state.clients.get(&id);
					let uid = client.and_then(|c| c.uid.as_ref());
					client_data = uid.map(|uid| {
						let c = client.unwrap();
						let icon = if c.icon.0 == 0 { None } else { Some(c.icon.0 as i32) };
						let avatar = if c.avatar_hash.is_empty() {
							None
						} else {
							Some(c.avatar_hash.clone())
						};
						db::ClientData {
							name: c.name.clone(),
							uid: uid.clone(),
							icon,
							avatar,
							phonetic_name: if c.phonetic_name != "" {
								Some(c.phonetic_name.clone())
							} else {
								None
							},
							description: if c.description != "" {
								Some(c.description.clone())
							} else {
								None
							},
						}
					});
					if let Some(uid) = uid {
						if let MessageTarget::Client(_) = target {
							ChatType::Client(uid.0.clone())
						} else {
							ChatType::Poke(uid.0.clone())
						}
					} else {
						self.send_error(return_code, "Failed to get uid of client".into());
						return;
					}
				}
			};

			let msg = db::WriteMessageMsg {
				message,
				invoker_uid,
				chat: ChatId { server, chat_type },
				client_data,
			};
			actix::spawn(self.state.database.send(msg).map(move |r| match r {
				Ok(Ok(())) => {}
				Ok(Err(error)) => {
					error!(%error, "Failed to handle event in database");
				}
				Err(_) => {
					error!("Failed to send event to database");
				}
			}));
		} else {
			self.send_error(return_code, "Failed to get connection state".into());
		}
	}

	fn send_ts_message(&mut self, mut msg: OutCommand, return_code: Option<&str>) -> Result<()> {
		let _span = self.span.clone().entered();
		if let Some(code) = &return_code {
			msg.write_arg("return_code", code);
		}
		if let Some(con) = &mut self.connection {
			let r = msg.send(con);
			if let Err(e) = &r {
				self.send_error(return_code, format!("Failed to send message: {}", e));
			}
			r.map_err(|e| e.into())
		} else {
			self.send_error(return_code, "Not connected".into());
			bail!("Not connected");
		}
	}

	fn send_command(&mut self, command: String, return_code: Option<&str>) {
		let cmd = tsproto_packets::packets::OutCommand::new(
			tsproto_packets::packets::Direction::C2S,
			tsproto_packets::packets::Flags::empty(),
			tsproto_packets::packets::PacketType::Command,
			&command,
		);
		let _ = self.send_ts_message(cmd, return_code);
	}

	fn for_other_connections<
		P: tsclientlib::OutCommandExt,
		F: FnOnce(&tsclientlib::data::Connection) -> Option<P> + Clone + Send + 'static,
	>(
		&self, f: F,
	) {
		let cons = self
			.state
			.connections
			.lock()
			.unwrap()
			.iter()
			.filter_map(|(id, addr)| if *id != self.id { Some(addr.clone()) } else { None })
			.collect::<Vec<_>>();
		let state = self.state.clone();
		actix::spawn(async move { state.send_each_con(cons.into_iter(), f).await });
	}
}

impl Handler<DownloadFile> for QintConnection {
	type Result = ActorResponse<Self, Result<DownloadFileContext, Error>>;
	fn handle(&mut self, msg: DownloadFile, _: &mut Self::Context) -> Self::Result {
		let con = match &mut self.connection {
			Some(con) => con,
			_ => {
				return ActorResponse::r#async(wrap_future(futures::future::err(
					Error::NoConnection,
				)));
			}
		};

		let handle = match con.download_file(msg.channel, &msg.path, None, None) {
			Ok(r) => r,
			Err(e) => {
				return ActorResponse::r#async(wrap_future(futures::future::err(e.into())));
			}
		};

		let (send, recv) = oneshot::channel();
		self.file_downloads.insert(handle, send);
		ActorResponse::r#async(wrap_future(recv).map(|r, this: &mut Self, _| {
			let result = match r {
				Ok(Ok(r)) => Ok(DownloadFileContext { size: r.size, stream: r.stream }),
				Ok(Err(e)) => Err(e.into()),
				Err(e) => Err(e.into()),
			};
			if let Some(return_code) = msg.return_code {
				this.send_message(&MessageP2F::Result(ResultStruct {
					return_code,
					details: (&result).try_into().unwrap_or_else(|e| {
						ResultDetails::from_desc(format!("Download failed, {}", e))
					}),
				}));
			}
			result
		}))
	}
}

impl Handler<UploadFile> for QintConnection {
	type Result = ActorResponse<Self, Result<UploadFileContext, Error>>;
	fn handle(&mut self, msg: UploadFile, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let handle = match con.upload_file(
				msg.channel,
				&msg.path,
				msg.channel_password.as_deref(),
				msg.size,
				msg.overwrite,
				msg.resume,
			) {
				Ok(r) => r,
				Err(e) => {
					return ActorResponse::r#async(wrap_future(futures::future::err(e.into())));
				}
			};
			let (send, recv) = oneshot::channel();
			self.file_uploads.insert(handle, send);
			ActorResponse::r#async(wrap_future(recv).map(|r, this: &mut Self, _| {
				let result = match r {
					Ok(Ok(r)) => Ok(UploadFileContext { stream: r.stream }),
					Ok(Err(e)) => Err(e.into()),
					Err(e) => Err(e.into()),
				};
				if let Some(return_code) = msg.return_code {
					this.send_message(&MessageP2F::Result(ResultStruct {
						return_code,
						details: (&result).try_into().unwrap_or_else(|e| {
							ResultDetails::from_desc(format!("Upload failed, {}", e))
						}),
					}));
				}
				result
			}))
		} else {
			ActorResponse::r#async(wrap_future(futures::future::err(Error::NoConnection)))
		}
	}
}

impl Handler<GetPublicKeyMsg> for QintConnection {
	type Result = Result<EccKeyPubP256>;
	fn handle(&mut self, _: GetPublicKeyMsg, _: &mut Self::Context) -> Self::Result {
		if let Some(con) = &self.connection {
			Ok(con.get_server_key()?)
		} else {
			Err(format_err!("Connection does not exist"))
		}
	}
}

impl Handler<GetClientVolumeMsg> for QintConnection {
	type Result = ActorResponse<Self, Result<f32>>;
	fn handle(
		&mut self, GetClientVolumeMsg(client): GetClientVolumeMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(state) = self.get_book() {
			let uid_fut: Box<dyn ActorFuture<Self, Output = Result<UidBuf>> + Unpin>;
			if let Some(client) = state.clients.get(&client) {
				uid_fut = Box::new(wrap_future(future::ready(
					client
						.uid
						.as_ref()
						.map(|u| u.clone())
						.ok_or_else(|| format_err!("Client has no uid")),
				)));
			} else {
				// TODO Get uid from server
				uid_fut = Box::new(wrap_future(future::err(format_err!("Not yet implemented"))));
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
		} else {
			ActorResponse::r#async(wrap_future(future::err(format_err!(
				"Connection does not exist"
			))))
		}
	}
}

impl Handler<SetSelfTalkingMsg> for QintConnection {
	type Result = ();
	fn handle(
		&mut self, SetSelfTalkingMsg(talking): SetSelfTalkingMsg, _: &mut Self::Context,
	) -> Self::Result {
		if self.self_talking != talking {
			self.self_talking = talking;
			self.update_talkers();
			if !talking {
				self.own_loudness.clear();
			}
		}
	}
}

impl Handler<TalkersChangedMsg> for QintConnection {
	type Result = ();
	fn handle(
		&mut self, TalkersChangedMsg(talkers): TalkersChangedMsg, _: &mut Self::Context,
	) -> Self::Result {
		if self.talkers != talkers {
			self.talkers = talkers;
			self.update_talkers();
		}
	}
}

impl Handler<LoudnessesMsg> for QintConnection {
	type Result = ();
	fn handle(
		&mut self, LoudnessesMsg(loudnesses): LoudnessesMsg, _: &mut Self::Context,
	) -> Self::Result {
		let mut ls;
		if self.talkers.is_empty() {
			ls = HashMap::new();
		} else {
			ls = loudnesses
				.into_iter()
				.map(|(id, l)| (id.to_string(), l as f32))
				.collect::<HashMap<_, _>>();
		}
		if let Some(own_loudness) = self.own_loudness.pop_front() {
			if let Some(state) = self.get_book() {
				ls.insert(state.own_client.to_string(), own_loudness as f32);
			}
		}
		self.send_message(&MessageP2F::Loudnesses(ls));
	}
}

impl Handler<CaptureLoudnessMsg> for QintConnection {
	type Result = ();
	fn handle(
		&mut self, CaptureLoudnessMsg(loudness, _vad): CaptureLoudnessMsg, _: &mut Self::Context,
	) -> Self::Result {
		// If nobody else is talking, sent it as a packet.
		if self.talkers.is_empty() {
			self.own_loudness.clear();
			if let Some(state) = self.get_book() {
				let mut loudnesses = HashMap::new();
				loudnesses.insert(state.own_client.to_string(), loudness as f32);
				self.send_message(&MessageP2F::Loudnesses(loudnesses));
			}
		} else {
			self.own_loudness.push_back(loudness);
		}
	}
}

impl Handler<SendPacketMsg> for QintConnection {
	type Result = Result<()>;
	fn handle(
		&mut self, SendPacketMsg(packet): SendPacketMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &mut self.connection {
			con.send_audio(packet)?;
			Ok(())
		} else {
			bail!("Connection does not exist")
		}
	}
}

impl Handler<SendAudioMsg> for QintConnection {
	type Result = Result<()>;
	fn handle(
		&mut self, SendAudioMsg(codec, data): SendAudioMsg, _: &mut Self::Context,
	) -> Self::Result {
		if let Some(con) = &mut self.connection {
			let packet = if let Some(whisper) = &self.whisper_list {
				let channels = whisper.channels.iter().map(|i| i.0).collect();
				let clients = whisper.clients.iter().map(|i| i.0).collect();
				OutAudio::new(&AudioData::C2SWhisper {
					id: 0,
					channels,
					clients,
					codec,
					data: &data,
				})
			} else {
				OutAudio::new(&AudioData::C2S { id: 0, codec, data: &data })
			};
			con.send_audio(packet)?;
			Ok(())
		} else {
			bail!("Connection does not exist")
		}
	}
}

impl Handler<DisconnectMsg> for QintConnection {
	type Result = ();
	fn handle(&mut self, _: DisconnectMsg, ctx: &mut Self::Context) -> Self::Result {
		self.disconnect(ctx);
	}
}

impl Handler<SetChannelListTaskMsg> for QintConnection {
	type Result = ();
	fn handle(&mut self, msg: SetChannelListTaskMsg, _: &mut Self::Context) -> Self::Result {
		self.channel_list_finished_task = Some(msg.0);
	}
}

impl<R: 'static, F: FnOnce(&mut QintConnection) -> R> Handler<RunOnConMsg<R, F>>
	for QintConnection
{
	type Result = MessageResult<RunOnConMsg<R, F>>;
	fn handle(&mut self, msg: RunOnConMsg<R, F>, _: &mut Self::Context) -> Self::Result {
		MessageResult(msg.0(self))
	}
}

impl Handler<MessageF2PWrapper> for QintConnection {
	type Result = ();
	fn handle(&mut self, msg: MessageF2PWrapper, ctx: &mut Self::Context) -> Self::Result {
		self.handle_ws_message(msg.0, ctx);
	}
}

impl ActorFuture<QintConnection> for ConnectionPoller {
	type Output = ();
	fn poll(
		self: Pin<&mut Self>, actor: &mut QintConnection,
		ctx: &mut <QintConnection as Actor>::Context, task: &mut task::Context,
	) -> Poll<Self::Output> {
		let _span = actor.span.clone().entered();
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
				Poll::Ready(Some(Err(error))) => {
					error!(%error, "Connection failed");
					actor.connection = None;

					// Send to frontend
					if let Some(return_code) =
						actor.connect_options.as_mut().and_then(|o| o.return_code.take())
					{
						let mut ts_result = None;
						let mut description = None;
						if let TsclError::ConnectTs(e) = &error {
							ts_result = Some(*e);
						} else {
							description = Some(error.to_string());
						}

						actor.send_message(&MessageP2F::Result(ResultStruct {
							return_code,
							details: ResultDetails {
								ts_result,
								missing_permission: None,
								description,
							},
						}));
					} else {
						actor.send_message(&MessageP2F::Error(error.to_string()));
					}
					break Poll::Ready(());
				}
				Poll::Ready(Some(Ok(item))) => {
					actor.handle_event(item, ctx);
				}
			}
		}
	}
}
