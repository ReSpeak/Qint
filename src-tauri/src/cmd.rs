use std::{fmt::Debug, thread, time::Duration};

use qint_proxy::{
	messages::{MessageF2P, MessageP2F},
	App, AppToFrontendBridge, ConnectionId,
};
use serde::{Deserialize, Serialize};
use tauri::{command, State, Window};
use tokio::runtime::Runtime;
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
pub async fn create_ws(app: State<'_, App>, window: Window, uuid: Uuid) -> Result<String, ()> {
	let id = ConnectionId(uuid);
	println!("CREATE WS: {:?}", uuid);

	let state = app.inner().0.clone();
	qint_proxy::State::create_ws(
		uuid,
		state,
		Box::new(WindowBridge { window, id }),
	).await;

	Ok("OK".into())
}

#[command]
pub async fn pass_ws_msg(
	app: State<'_, App>, connection: Uuid, msg: TauriMsg<MessageF2P>,
) -> Result<(), ()> {
	println!("WS: {:?} {:?}", connection, msg);
	match msg {
		TauriMsg::Close => {}
		TauriMsg::Msg(msg) => app.inner().0.send_ws(ConnectionId(connection), msg).await,
	}
	Ok(())
}
