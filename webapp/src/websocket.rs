use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use actix::prelude::*;
use actix_web_actors::ws;
use anyhow::{bail, format_err, Result};
use futures::FutureExt;
use juniper::http::GraphQLRequest;
use proxy_codegen::book_events::deserialize_u64;
use qint_proxy::audio::audio_to_ts::{AddLoudnessListenerMsg, LoudnessTrait};
use qint_proxy::connection::CaptureLoudnessMsg;
use qint_proxy::{
	connection::{DisconnectMsg, MessageF2PWrapper, QintConnection},
	db::models::UpdateIdentity,
	db::{
		DeleteIdentityMsg, FindIdentity, GenrateNewIdentityMsg, GetIdentitiesMsg, UpdateIdentityMsg,
	},
	hotkey::Action,
	identities::import_ts_identities_from_string,
	messages::{MessageF2P, MessageP2F},
	shared::UpdateIdentityOptions,
	AppToFrontendBridge, ConnectionId, QintState,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, error};

pub struct WsBridge {
	pub ws: Addr<Ws>,
	pub id: ConnectionId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct F2PMsg {
	cmd: String,
	return_code: String,
	args: serde_json::Value,
}

#[derive(Debug, Error)]
pub enum Error {
	#[error("Connection already in use")]
	ConnectionInUse,
	#[error("Connection does not exist")]
	NoConnection,
	#[error("Unknown command '{0}'")]
	UnknownCommand(String),
	#[error("Audio disabled")]
	NoAudio,
	#[error("Failed to setup audio listener")]
	LoudnessListenerFailed,
}

#[derive(Deserialize)]
pub struct StringId(#[serde(deserialize_with = "deserialize_u64")] pub u64);

pub struct SendToFrontendMsg(pub String);

pub struct Ws {
	state: Arc<QintState>,
	/// All connections managed by this websocket
	connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
	/// The loudness listener that is running
	loudness_listener: Arc<AtomicU32>,
}

impl Message for SendToFrontendMsg {
	type Result = ();
}

macro_rules! unwrap_send {
	($act:expr, $msg:expr) => {{
		match $act.send($msg).await {
			Ok(Ok(v)) => return Ok(serde_json::to_value(&v).unwrap()),
			Ok(Err(err)) => return Err(err.into()),
			Err(_) => {
				return Err(format_err!(concat!(
					"Mailbox error sending '",
					stringify!($msg),
					"' to ",
					stringify!($act),
				)));
			}
		}
	}};
}

impl AppToFrontendBridge for WsBridge {
	fn send(&self, msg: &MessageP2F) {
		#[derive(Serialize)]
		struct WsMsg<'a> {
			cmd: &'static str,
			con: ConnectionId,
			msg: &'a MessageP2F,
		}

		actix::spawn(with_log!(
			self.ws.send(SendToFrontendMsg(
				serde_json::to_string(&WsMsg { cmd: "ws", con: self.id.clone(), msg }).unwrap()
			)),
			"Failed to forward msg to frontend"
		));
	}

	fn close(&self) {
		#[derive(Serialize)]
		struct WsMsg {
			cmd: &'static str,
			con: ConnectionId,
		}

		actix::spawn(with_log!(
			self.ws.send(SendToFrontendMsg(
				serde_json::to_string(&WsMsg { cmd: "ws_close", con: self.id.clone() }).unwrap()
			)),
			"Failed to send close msg to websocket"
		));
	}
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
	fn stopped(&mut self, _: &mut Self::Context) { self.close(); }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(msg)) => {
				let msg: F2PMsg = match serde_json::from_str(&msg) {
					Ok(r) => r,
					Err(error) => {
						error!(%error, message = %msg, "json deserializing error");
						ctx.close(None);
						return;
					}
				};

				#[derive(Serialize)]
				#[serde(rename_all = "camelCase")]
				struct WsMsg<'a> {
					cmd: &'static str,
					return_code: &'a str,
					msg: serde_json::Value,
				}

				let return_code = msg.return_code;
				ctx.spawn(
					actix::fut::wrap_future::<_, Self>(Self::handle_msg(
						self.state.clone(),
						self.connections.clone(),
						self.loudness_listener.clone(),
						msg.cmd,
						msg.args,
						ctx.address(),
					))
					.map(move |res, _, ctx| {
						let resp = match res {
							Ok(r) => WsMsg { cmd: "resp", return_code: &return_code, msg: r },
							Err(e) => WsMsg {
								cmd: "resp_err",
								return_code: &return_code,
								msg: serde_json::Value::String(e.to_string()),
							},
						};
						ctx.text(serde_json::to_string(&resp).unwrap());
					}),
				);
			}
			Ok(ws::Message::Binary(_)) => {
				error!("binary protocol not supported");
			}
			Ok(ws::Message::Close(_)) => {
				self.close();
			}
			_ => {}
		}
	}
}

impl Handler<SendToFrontendMsg> for Ws {
	type Result = ();
	fn handle(&mut self, msg: SendToFrontendMsg, ctx: &mut Self::Context) -> Self::Result {
		ctx.text(msg.0);
	}
}

impl Ws {
	pub fn new(state: Arc<QintState>) -> Self {
		Self { state, connections: Default::default(), loudness_listener: Default::default() }
	}

	fn close(&self) {
		debug!("Websocket closed");

		// Close all connections for this websocket
		let mut own_cons = self.connections.lock().unwrap();
		for con in own_cons.drain() {
			actix::spawn(with_log!(
				con.1.send(DisconnectMsg),
				"Failed to send disconnect msg to QintConnection"
			));
		}
	}

	async fn enable_loudness_callback(
		state: Arc<QintState>, loudness_listener: Arc<AtomicU32>, addr: Addr<Self>,
	) -> Option<Result<serde_json::Value>> {
		if let Some(ad) = &state.audio_data {
			#[derive(Serialize)]
			struct WsMsg {
				cmd: &'static str,
				msg: LoudnessFrame,
			}
			#[derive(Serialize, Copy, Clone)]
			struct LoudnessFrame(f64, f32);

			pub struct LoudnessCallback {
				addr: Addr<Ws>,
				own_id: u32,
				listening: Arc<AtomicU32>,
			}

			impl LoudnessTrait for LoudnessCallback {
				fn send(&self, msg: CaptureLoudnessMsg) {
					let listening = self.listening.clone();
					let own_id = self.own_id;
					actix::spawn(
						self.addr
							.send(SendToFrontendMsg(
								serde_json::to_string(&WsMsg {
									cmd: "loudness",
									msg: LoudnessFrame(msg.0, msg.1),
								})
								.unwrap(),
							))
							.map(move |r| {
								if r.is_err() {
									// Connection is probably gone, disable
									let _ = listening.compare_exchange(
										own_id,
										own_id.wrapping_add(1),
										Ordering::Relaxed,
										Ordering::Relaxed,
									);
								}
							}),
					);
				}

				fn connected(&self) -> bool {
					self.listening.load(Ordering::Relaxed) == self.own_id
				}
			}

			let own_id = loudness_listener.fetch_add(1, Ordering::Relaxed) + 1;
			let callback = LoudnessCallback { addr, listening: loudness_listener.clone(), own_id };

			if ad.a2ts.send(AddLoudnessListenerMsg(Box::new(callback))).await.is_err() {
				Some(Err(Error::LoudnessListenerFailed.into()))
			} else {
				None
			}
		} else {
			Some(Err(Error::NoAudio.into()))
		}
	}

	async fn handle_msg(
		state: Arc<QintState>,
		connections: Arc<Mutex<HashMap<ConnectionId, Addr<QintConnection>>>>,
		loudness_listener: Arc<AtomicU32>, cmd: String, args: serde_json::Value, addr: Addr<Self>,
	) -> Result<serde_json::Value> {
		#[derive(Deserialize)]
		struct ConArgs {
			con: ConnectionId,
		}

		match cmd.as_str() {
			"create_ws" => {
				let args: ConArgs = serde_json::from_value(args)?;
				let id = args.con;

				let mut cons = state.connections.lock().unwrap();
				if cons.contains_key(&id) || !id.is_valid() {
					error!(error = ?id, "Connection already in use. Duplicate create call?");
					return Err(Error::ConnectionInUse.into());
				}

				let sender = Box::new(WsBridge { ws: addr, id: id.clone() });
				let ws = QintConnection::new(state.clone(), id.clone(), sender);
				let addr = ws.start();
				connections.lock().unwrap().insert(id.clone(), addr.clone());
				cons.insert(id, addr);
			}
			"close_ws" => {
				let args: ConArgs = serde_json::from_value(args)?;
				let id = args.con;

				let con = {
					match state.connections.lock().unwrap().get(&id) {
						Some(con) => con.clone(),
						None => {
							error!(error = ?id, "No con for msg found");
							return Err(Error::NoConnection.into());
						}
					}
				};

				actix::spawn(with_log!(
					con.send(qint_proxy::connection::DisconnectMsg),
					"Failed to send disconnect to connection"
				));
				connections.lock().unwrap().remove(&id);
			}
			"pass_ws_msg" => {
				#[derive(Deserialize)]
				struct Args {
					con: ConnectionId,
					msg: MessageF2P,
				}
				let args: Args = serde_json::from_value(args)?;

				let con = state.get_connection(&args.con).ok_or_else(|| {
					error!(error = ?args.con, "No con for msg found");
					Error::NoConnection
				})?;

				actix::spawn(with_log!(
					con.send(MessageF2PWrapper(args.msg)),
					"Failed to forward Message to Proxy"
				));
			}
			"db" => {
				#[derive(Deserialize)]
				struct Args {
					request: GraphQLRequest,
				}
				let args: Args = serde_json::from_value(args)?;

				let res = args.request.execute(&state.graphql_schema, &*state).await;
				if res.is_ok() {
					return Ok(serde_json::to_value(&res).unwrap());
				}
				bail!("Failed to handle graphql request ({:?})", res);
			}
			"get_settings" => {
				let values = state.settings.read().unwrap();
				return Ok(serde_json::to_value(&*values).unwrap());
			}
			"set_settings" => {
				#[derive(Deserialize)]
				struct Args {
					diff: serde_json::Value,
				}
				let args: Args = serde_json::from_value(args)?;

				QintState::set_settings_diff(&state, &args.diff)?;
			}
			"peek_link" => {
				#[derive(Deserialize)]
				struct Args {
					link: String,
				}
				let args: Args = serde_json::from_value(args)?;

				return Ok(serde_json::to_value(
					&state.link_previewer.analyze_link(&args.link).await,
				)
				.unwrap());
			}
			"get_audio_device_list" => {
				return Ok(serde_json::to_value(&state.audio_device_list().await).unwrap());
			}
			"identity_create" => {
				// TODO -> Gen*e*rate
				unwrap_send!(state.database, GenrateNewIdentityMsg())
			}
			"identity_import" => {
				#[derive(Deserialize)]
				struct Args {
					data: String,
				}
				let args: Args = serde_json::from_value(args)?;

				import_ts_identities_from_string(&state, &args.data).await?;
			}
			"identity_list" => {
				#[derive(Deserialize)]
				struct Args {
					find: FindIdentity,
				}
				let args: Args = serde_json::from_value(args)?;

				unwrap_send!(state.database, GetIdentitiesMsg(args.find))
			}
			"identity_update" => {
				#[derive(Deserialize)]
				struct Args {
					id: StringId,
					update: UpdateIdentityOptions,
				}
				let args: Args = serde_json::from_value(args)?;

				unwrap_send!(
					state.database,
					UpdateIdentityMsg(FindIdentity::ById(args.id.0), UpdateIdentity {
						name: args.update.name,
						..Default::default()
					},)
				)
			}
			"identity_delete" => {
				#[derive(Deserialize)]
				struct Args {
					id: StringId,
				}
				let args: Args = serde_json::from_value(args)?;

				unwrap_send!(state.database, DeleteIdentityMsg(FindIdentity::ById(args.id.0)))
			}
			"get_mutestate" => {
				return Ok(serde_json::to_value(&state.get_mute_state().await).unwrap());
			}
			"run_hotkey" => {
				#[derive(Deserialize)]
				struct Args {
					action: Action,
				}
				let args: Args = serde_json::from_value(args)?;

				args.action.run(&state).await;
			}
			"plugin_list" => {
				return Ok(serde_json::to_value(&state.plugin_list()).unwrap());
			}
			"plugin_get" => {
				#[derive(Deserialize)]
				struct Args {
					name: String,
				}
				let args: Args = serde_json::from_value(args)?;

				return Ok(serde_json::to_value(&state.plugin_get(&args.name)?).unwrap());
			}
			"plugin_save" => {
				#[derive(Deserialize)]
				struct Args {
					name: String,
					content: String,
				}
				let args: Args = serde_json::from_value(args)?;

				return Ok(
					serde_json::to_value(&state.plugin_save(&args.name, &args.content)?).unwrap()
				);
			}
			"plugin_delete" => {
				#[derive(Deserialize)]
				struct Args {
					name: String,
				}
				let args: Args = serde_json::from_value(args)?;

				return Ok(serde_json::to_value(&state.plugin_delete(&args.name)?).unwrap());
			}
			"markdown" => {
				#[derive(Deserialize)]
				struct Args {
					md: String,
				}
				let args: Args = serde_json::from_value(args)?;

				return Ok(serde_json::Value::String(proxy_codegen::markdown::markdown(&args.md)));
			}
			"set_loudness_callback" => {
				#[derive(Deserialize)]
				struct Args {
					enabled: bool,
				}
				let args: Args = serde_json::from_value(args)?;

				if args.enabled {
					if let Some(res) =
						Self::enable_loudness_callback(state, loudness_listener, addr).await
					{
						return res;
					}
				} else {
					// Increase id, so the listener discards itself
					loudness_listener.fetch_add(1, Ordering::Relaxed);
				}
			}
			_ => {
				return Err(Error::UnknownCommand(cmd.to_string()).into());
			}
		}

		Ok(serde_json::Value::Null)
	}
}
