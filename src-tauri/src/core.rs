use std::sync::Arc;

use actix::prelude::*;
use futures::prelude::*;
use qint_proxy::connection::MessageF2PWrapper;
use qint_proxy::connection::QintConnection;
use qint_proxy::messages::MessageF2P;
use qint_proxy::with_log;
use qint_proxy::ConnectionId;
use qint_proxy::FrontBridge;
use qint_proxy::QintState;
use slog::warn;

#[derive(Clone)]
pub struct QintCore {
	pub state: Arc<QintState>,
}

impl Actor for QintCore {
	type Context = Context<Self>;
}

pub struct CreateWs {
	pub id: ConnectionId,
	pub sender: FrontBridge,
}

pub struct DispatchWsMsg {
	pub id: ConnectionId,
	pub msg: MessageF2P,
}

impl Message for CreateWs {
	type Result = ();
}
impl Message for DispatchWsMsg {
	type Result = ();
}
impl Handler<CreateWs> for QintCore {
	type Result = ();
	fn handle(&mut self, msg: CreateWs, _: &mut Self::Context) -> Self::Result {
		let CreateWs { id, sender } = msg;

		let mut cons = self.state.connections.lock().unwrap();
		// Check that the id does not exist
		if cons.contains_key(&id) || id.0.is_nil() {
			// TODO
			println!("uuid fuk up");
			return;
		}

		let ws =
			QintConnection::new(self.state.logger.clone(), self.state.clone(), id.clone(), sender);
		let addr = ws.start();
		cons.insert(id, addr);
	}
}
impl Handler<DispatchWsMsg> for QintCore {
	type Result = ();
	fn handle(&mut self, msg: DispatchWsMsg, _: &mut Self::Context) -> Self::Result {
		let DispatchWsMsg { id, msg } = msg;

		let con = {
			match self.state.connections.lock().unwrap().get(&id) {
				Some(con) => con.clone(),
				None => {
					println!("No con for msg found {:?}", id);
					warn!(self.state.logger, "No con for msg found"; "error" => ?id);
					return;
				}
			}
		};

		actix::spawn(with_log!(
			con.send(MessageF2PWrapper(msg)),
			self.state.logger.clone(),
			"Failed to forward Message to Proxy"
		));
	}
}
