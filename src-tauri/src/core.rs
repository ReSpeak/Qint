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
use slog::error;
use thiserror::Error;

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

pub struct CloseWs {
	pub id: ConnectionId,
}

pub struct DispatchWsMsg {
	pub id: ConnectionId,
	pub msg: MessageF2P,
}

#[derive(Debug, Error, Serialize)]
pub enum Error {
	#[error("Connection already in use")]
	ConnectionInUse,
	#[error("Connection does not exist")]
	NoConnection,
}

impl Message for CreateWs {
	type Result = Result<(), Error>;
}
impl Message for CloseWs {
	type Result = Result<(), Error>;
}
impl Message for DispatchWsMsg {
	type Result = Result<(), Error>;
}

impl Handler<CreateWs> for QintCore {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: CreateWs, _: &mut Self::Context) -> Self::Result {
		let CreateWs { id, sender } = msg;

		let mut cons = self.state.connections.lock().unwrap();
		if cons.contains_key(&id) || id.0.is_nil() {
			error!(self.state.logger, "Connection already in use. Duplicate create call?"; "error" => ?id);
			return Err(Error::ConnectionInUse);
		}

		let ws =
			QintConnection::new(self.state.logger.clone(), self.state.clone(), id.clone(), sender);
		let addr = ws.start();
		cons.insert(id, addr);
		Ok(())
	}
}

impl Handler<CloseWs> for QintCore {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: CloseWs, _: &mut Self::Context) -> Self::Result {
		let CloseWs { id } = msg;

		let con = {
			match self.state.connections.lock().unwrap().get(&id) {
				Some(con) => con.clone(),
				None => {
					error!(self.state.logger, "No con for msg found"; "error" => ?id);
					return Err(Error::NoConnection);
				}
			}
		};

		actix::spawn(with_log!(
			con.send(qint_proxy::connection::DisconnectMsg),
			self.state.logger.clone(),
			"Failed to send disconnect to connection"
		));
		Ok(())
	}
}

impl Handler<DispatchWsMsg> for QintCore {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: DispatchWsMsg, _: &mut Self::Context) -> Self::Result {
		let DispatchWsMsg { id, msg } = msg;

		let con = {
			match self.state.connections.lock().unwrap().get(&id) {
				Some(con) => con.clone(),
				None => {
					error!(self.state.logger, "No con for msg found"; "error" => ?id);
					return Err(Error::NoConnection);
				}
			}
		};

		actix::spawn(with_log!(
			con.send(MessageF2PWrapper(msg)),
			self.state.logger.clone(),
			"Failed to forward Message to Proxy"
		));
		Ok(())
	}
}
