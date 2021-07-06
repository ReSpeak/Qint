use std::{fmt::Debug};

use actix::Addr;
use qint_proxy::{
	messages::{MessageF2P, MessageP2F},
	AppToFrontendBridge, ConnectionId, CreateWs, DispatchWsMsg, QintCore,
};
use serde::{Deserialize, Serialize};
use tauri::{command, State, Window};
use uuid::Uuid;

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
	window: Window,
	id: ConnectionId,
}

impl AppToFrontendBridge for WindowBridge {
	fn send(&self, msg: &MessageP2F) {
		println!("SEND: {:?}", msg);
		let res =
			self.window.emit("ws", TauriWs { connection: self.id.0, msg: TauriMsg::Msg(msg) });
		if res.is_err() {
			println!("Hey can you put this into the logger? {:?}", res);
		}
	}
}

#[command]
pub async fn create_ws(
	qc_act: State<'_, Addr<QintCore>>, window: Window, uuid: Uuid,
) -> Result<String, ()> {
	let id = ConnectionId(uuid);
	let state = qc_act.inner();
	println!("CREATE WS: {:?}", id);

	let sender = Box::new(WindowBridge { window, id: id.clone() });
	state.send(CreateWs { id, sender }).await.unwrap();

	Ok("OK".into())
}

#[command]
pub async fn pass_ws_msg(
	qc_act: State<'_, Addr<QintCore>>, connection: Uuid, msg: TauriMsg<MessageF2P>,
) -> Result<(), ()> {
	let id = ConnectionId(connection);
	let state = qc_act.inner();
	println!("WS: {:?} {:?}", id, msg);

	match msg {
		TauriMsg::Close => {}
		TauriMsg::Msg(msg) => state.send(DispatchWsMsg { id, msg }).await.unwrap(),
	}
	Ok(())
}

#[command]
pub async fn pass_ws_msg2(
	qc_act: State<'_, QintCore>, connection: Uuid, msg: TauriMsg<MessageF2P>,
) -> Result<(), ()> {
	qc_act.state.connections.lock();
	Ok(())
}
