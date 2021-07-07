use actix::prelude::*;
use actix_web_actors::ws;
use futures::FutureExt;
use qint_proxy::{
	connection::{DisconnectMsg, MessageF2PWrapper, QintConnection},
	messages::{MessageF2P, MessageP2F},
	AppToFrontendBridge,
};
use slog::{debug, error, Logger};

pub struct WsBridge(pub Addr<Ws>);
impl AppToFrontendBridge for WsBridge {
	fn send(&self, msg: &MessageP2F) {
		actix::spawn(self.0.send(SendToFrontendMsg(serde_json::to_string(msg).unwrap())).map(|_| ()));
	}

	fn close(&self) {
		actix::spawn(self.0.send(CloseMsg()).map(|_| ()));
	}
}

pub struct SendToFrontendMsg(pub String);
pub struct SetConnectionMsg(pub Addr<QintConnection>);
pub struct CloseMsg();

impl Message for SendToFrontendMsg {
	type Result = ();
}
impl Message for SetConnectionMsg {
	type Result = ();
}
impl Message for CloseMsg {
	type Result = ();
}

pub struct Ws {
	logger: Logger,
	qint_con: Option<Addr<QintConnection>>,
}

impl Ws {
	pub fn new(logger: Logger, qint_con: Option<Addr<QintConnection>>) -> Self {
		Self { logger, qint_con }
	}

	fn send_message(&mut self, msg: &MessageP2F, ctx: &mut <Self as Actor>::Context) {
		ctx.text(serde_json::to_string(msg).unwrap());
	}
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Ws {
	fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(msg)) => {
				let msg: MessageF2P = match serde_json::from_str(&msg) {
					Ok(r) => r,
					Err(e) => {
						let msg_str: &str = msg.as_ref();
						error!(self.logger, "json deserializing error"; "error" => %e,
							"message" => msg_str);
						self.send_message(
							&MessageP2F::Error(format!("json deserializing error: {}", e)),
							ctx,
						);
						return;
					}
				};
				if let Some(qint_con) = &self.qint_con {
					actix::spawn(qint_con.send(MessageF2PWrapper(msg)).map(|_| ()));
				} else {
					panic!("Connection was not yet set. How should we handle this?");
				}
			}
			Ok(ws::Message::Binary(_)) => {
				error!(self.logger, "binary protocol not supported");
			}
			Ok(ws::Message::Close(_)) => {
				debug!(self.logger, "Websocket closed");
				if let Some(qint_con) = &self.qint_con {
					actix::spawn(qint_con.send(DisconnectMsg).map(|_| ()));
				}
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

impl Handler<SetConnectionMsg> for Ws {
	type Result = ();
	fn handle(&mut self, msg: SetConnectionMsg, _: &mut Self::Context) -> Self::Result {
		if self.qint_con.is_some() {
			panic!("Connection was set twice. This should not happen");
		}

		self.qint_con = Some(msg.0);
	}
}

impl Handler<CloseMsg> for Ws {
	type Result = ();
	fn handle(&mut self, _: CloseMsg, ctx: &mut Self::Context) -> Self::Result {
		ctx.close(None);
	}
}
