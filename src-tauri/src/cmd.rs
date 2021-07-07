use std::{fmt::Debug, sync::Arc};

use actix::Addr;
use anyhow::bail;
use juniper::http::GraphQLRequest;
use qint_proxy::{
	messages::{MessageF2P, MessageP2F},
	AppToFrontendBridge, ConnectionId, QintState, Settings, SettingsUpdate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
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

type QState = Arc<QintState>;

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
pub async fn db(state: State<'_, QState>, request: GraphQLRequest) -> Result<String, ()> {
	let res = request.execute(&state.graphql_schema, &*state).await;
	if res.is_ok() {
		Ok(serde_json::to_string(&res).unwrap())
	} else {
		Err(())
	}
}

#[command]
pub fn get_settings(state: State<'_, QState>) -> Value {
	let values = state.settings.read().unwrap();
	serde_json::to_value(&*values).unwrap()
}

#[command]
pub fn set_settings(state: State<'_, QState>, diff: Value) -> Result<(), String> {
	let (r, res) = QintState::modify_settings(&state.inner(), |values| {
		let hotkeys_changed;
		if let Value::Object(o) = &diff {
			hotkeys_changed = o.contains_key(Settings::KEY_HOTKEYS);
			values.merge(&diff);
		} else {
			bail!("body must be an object");
		}
		Ok(SettingsUpdate { hotkeys_changed })
	});

	if let Err(e) = r {
		Err(format!("Malformed diff: {}", e))
	} else if let Err(e) = res {
		Err(format!("Internal error: {}", e))
	} else {
		Ok(())
	}
}
