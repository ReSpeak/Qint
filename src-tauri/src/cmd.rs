use std::fmt::Debug;

use actix::Addr;
use juniper::http::GraphQLRequest;
use qint_proxy::{
	messages::{MessageF2P, MessageP2F},
	AppToFrontendBridge, ConnectionId,
};
use serde::{Deserialize, Serialize};
use slog::{debug, info, warn, Logger};
use tauri::{command, State, Window};
use uuid::Uuid;

use crate::core::{CreateWs, DispatchWsMsg, QintCore};

#[derive(Debug, Serialize, Deserialize)]
pub enum TauriMsg<T>
where
	T: Debug,
{
	Close,
	Msg(T),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TauriWs<T>
where
	T: Debug,
{
	connection: Uuid,
	msg: TauriMsg<T>,
}

struct WindowBridge {
	logger: Logger,
	window: Window,
	id: ConnectionId,
}

impl AppToFrontendBridge for WindowBridge {
	fn send(&self, msg: &MessageP2F) {
		debug!(self.logger, "Sending to frontend"; "msg" => ?msg);
		let res =
			self.window.emit("ws", TauriWs { connection: self.id.0, msg: TauriMsg::Msg(msg) });
		if let Err(e) = res {
			warn!(self.logger, "Failed sending to frontend"; "error" => %e);
		}
	}

	fn close(&self) {
		// (TODO: Check that there is) nothing to do here for now ?
	}
}

#[command]
pub async fn create_ws(
	logger: State<'_, Logger>, qc_act: State<'_, Addr<QintCore>>, window: Window, uuid: Uuid,
) -> Result<String, ()> {
	let id = ConnectionId(uuid);
	let state = qc_act.inner();
	info!(logger.inner().clone(), "Creating tauri connection"; "id" => ?id);

	let sender = Box::new(WindowBridge { logger: logger.inner().clone(), window, id: id.clone() });
	state.send(CreateWs { id, sender }).await.unwrap();

	Ok("OK".into())
}

#[command]
pub async fn pass_ws_msg(
	qc_act: State<'_, Addr<QintCore>>, connection: Uuid, msg: TauriMsg<MessageF2P>,
) -> Result<(), ()> {
	let id = ConnectionId(connection);
	let state = qc_act.inner();
	match msg {
		TauriMsg::Close => {}
		TauriMsg::Msg(msg) => state.send(DispatchWsMsg { id, msg }).await.unwrap(),
	}
	Ok(())
}

#[command]
pub async fn db(core: State<'_, QintCore>, request: GraphQLRequest) -> Result<String, ()> {
	let res = request.execute(&core.state.graphql_schema, &*core.state).await;
	if res.is_ok() {
		Ok(serde_json::to_string(&res).unwrap())
	} else {
		Err(())
	}
}
