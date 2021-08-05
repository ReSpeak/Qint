use std::{fmt::Debug, sync::Arc};

use actix::Addr;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use juniper::http::GraphQLRequest;
use proxy_codegen::book_events::deserialize_id;
use proxy_codegen::book_events::deserialize_u64;
use qint_proxy::MuteStates;
use qint_proxy::SettingsUpdateError;
use qint_proxy::{
	db::{
		models::UpdateIdentity, DeleteIdentityMsg, FindIdentity, GenrateNewIdentityMsg,
		GetIdentitiesMsg, UpdateIdentityMsg,
	},
	filecache::guess_content_type,
	identities::{import_ts_identities_from_string, ApiIdentity},
	link_previewer::AnalyzeResult,
	messages::{MessageF2P, MessageP2F},
	shared::{AudioDeviceList, UpdateIdentityOptions},
	AppToFrontendBridge, ConnectionId, QintState,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, info, warn, Logger};
use tauri::{command, State, Window};
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::Error as TsError;
use tsproto_types::{crypto::EccKeyPubP256, ChannelId};
use uuid::Uuid;

use crate::core::{CloseWs, CreateWs, DispatchWsMsg, Error, QintCore};

macro_rules! unwrap_send {
	($act:expr, $msg:expr) => {{
		match $act.send($msg).await {
			Ok(Ok(v)) => Ok(v),
			Ok(Err(err)) => Err(err.to_string()),
			Err(_) => Err(concat!(
				"Mailbox error sending '",
				stringify!($msg),
				"' to ",
				stringify!($act),
			)
			.into()),
		}
	}};
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TauriWs<T>
where
	T: Debug,
{
	con: Uuid,
	msg: T,
}

struct WindowBridge {
	logger: Logger,
	window: Window,
	id: ConnectionId,
}

type QState = Arc<QintState>;
type QCore = Addr<QintCore>;

impl AppToFrontendBridge for WindowBridge {
	fn send(&self, msg: &MessageP2F) {
		debug!(self.logger, "Sending to frontend"; "msg" => ?msg);
		let res = self.window.emit("ws", TauriWs { con: self.id.0, msg });
		if let Err(e) = res {
			warn!(self.logger, "Failed sending to frontend"; "error" => %e);
		}
	}

	fn close(&self) {
		let res = self.window.emit("ws_close", self.id.0);
		if let Err(e) = res {
			warn!(self.logger, "Failed sending to frontend"; "error" => %e);
		}
	}
}

#[command]
pub async fn create_ws(
	logger: State<'_, Logger>, core: State<'_, QCore>, window: Window, con: Uuid,
) -> Result<(), Error> {
	let id = ConnectionId(con);
	info!(logger.inner().clone(), "Creating tauri connection"; "id" => ?id);

	let sender = Box::new(WindowBridge { logger: logger.inner().clone(), window, id: id.clone() });
	core.send(CreateWs { id, sender }).await.unwrap()
}

#[command]
pub async fn close_ws(
	logger: State<'_, Logger>, core: State<'_, QCore>, con: Uuid,
) -> Result<(), Error> {
	let id = ConnectionId(con);
	info!(logger.inner().clone(), "Closing tauri connection"; "id" => ?id);
	core.send(CloseWs { id }).await.unwrap()
}

#[command]
pub async fn pass_ws_msg(core: State<'_, QCore>, con: Uuid, msg: MessageF2P) -> Result<(), Error> {
	let id = ConnectionId(con);
	core.send(DispatchWsMsg { id, msg }).await.unwrap()
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
	match QintState::set_settings_diff(&state, &diff) {
		Err(SettingsUpdateError::ModifyFailed(e)) => Err(e.to_string()),
		Err(SettingsUpdateError::InternalError(e)) => Err(format!("Internal error: {}", e)),
		Ok(_) => Ok(()),
	}
}

#[derive(Debug, Deserialize)]
pub struct FileRequest {
	con: Uuid,
	#[serde(deserialize_with = "deserialize_id")]
	channel: ChannelId,
	path: String,
	hash: Option<String>,
	cache: Option<bool>,
	return_code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FileCacheRequest {
	con: String,
	#[serde(deserialize_with = "deserialize_id")]
	channel: ChannelId,
	path: String,
	hash: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
	pub data: Vec<u8>,
	pub mime: Option<String>,
}

async fn collect_to_bytes<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin + 'static>(
	mut stream: S,
) -> Vec<u8> {
	let mut vec = Vec::new();
	while let Some(Ok(b)) = stream.next().await {
		vec.extend_from_slice(&b);
	}
	vec
}

#[command]
pub async fn get_file(state: State<'_, QState>, req: FileRequest) -> Result<FileResponse, String> {
	let state = state.inner();
	let FileRequest { con, channel, path, return_code, cache, .. } = req;
	let cache = cache.unwrap_or(false);

	let conn;
	{
		let cons = state.connections.lock().unwrap();
		conn = cons.get(&ConnectionId(con)).cloned();
	}

	if let Some(con) = conn {
		// Lookup in cache
		let server = match con.send(qint_proxy::connection::GetPublicKeyMsg).await {
			Ok(Ok(r)) => r,
			Ok(Err(e)) => {
				error!(state.logger, "Failed to get server public key"; "error" => %e);
				return Err("Failed to get server public key".into());
			}
			Err(_) => {
				return Err("Mailbox error: GetPublicKeyMsg".into());
			}
		};

		if let Some((_, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await {
			let (stream, mime) = guess_content_type(stream).await;
			let data = collect_to_bytes(stream).await;
			return Ok(FileResponse { data, mime: mime.map(|s| s.to_string()) });
		}

		debug!(state.logger, "Downloading file"; "channel" => channel.0, "path" => &path);
		let (len, file_stream, server) = match con
			.send(qint_proxy::connection::DownloadFile { channel, path: path.clone(), return_code })
			.await
		{
			Err(_) => {
				return Err("Mailbox error: DownloadFile".into());
			}
			Ok(Err(qint_proxy::connection::Error::TsError(TsError::CommandError(err)))) => {
				debug!(state.logger, "File download error"; "error" => %err, "path" => &path);
				return match err.error {
					tsclientlib::TsError::FileInvalidPath => Err("Invalid file".into()),
					tsclientlib::TsError::PermissionsClientInsufficient => {
						Err("Missing perm".into())
					}
					err => Err(format!("other err: {0}", err)),
				};
			}
			Ok(Err(e)) => {
				error!(state.logger, "File download failed"; "error" => %e, "path" => &path);
				return Err(format!("download err: {0}", e));
			}
			Ok(Ok(r)) => r,
		};

		let stream =
			FramedRead::new(file_stream, BytesCodec::new()).map(|r| r.map(BytesMut::freeze));
		let (stream, mime) = guess_content_type(stream).await;

		// Cache for offline usage if smaller than 5 MiB
		let data = if cache && len < 5 * 1024 * 1024 {
			let stream = state.file_cache.cache_file(&server, channel, &path, stream).await;
			collect_to_bytes(stream).await
		} else {
			collect_to_bytes(stream).await
		};
		return Ok(FileResponse { data, mime: mime.map(|s| s.to_string()) });
	} else {
		return Err("Connection not found".into());
	}
}

#[command]
pub async fn get_cache_file(
	state: State<'_, QState>, req: FileCacheRequest,
) -> Result<FileResponse, String> {
	let state = state.inner();
	let FileCacheRequest { con, channel, path, .. } = req;

	let server = match base64::decode_config(&con, base64::URL_SAFE_NO_PAD)
		.map_err(|e| e.into())
		.and_then(|id| EccKeyPubP256::from_short(&id))
	{
		Err(e) => {
			return Err(format!("Not a valid server id: {}", e));
		}
		Ok(id) => id,
	};

	if let Some((_, stream)) = state.file_cache.get_cached_file(&server, channel, &path).await {
		let (stream, mime) = guess_content_type(stream).await;
		let data = collect_to_bytes(stream).await;
		Ok(FileResponse { data, mime: mime.map(|s| s.to_string()) })
	} else {
		Err("File not found".into())
	}
}

#[command]
pub async fn download_file(
	state: State<'_, QState>, server_file: FileRequest, local_path: String,
) -> Result<(), ()> {
	Ok(()) // TODO
}

#[command]
pub async fn upload_file(
	state: State<'_, QState>, server_file: FileRequest, local_path: String,
) -> Result<(), ()> {
	Ok(()) // TODO
}

#[command]
pub async fn peek_link(state: State<'_, QState>, link: String) -> Result<AnalyzeResult, ()> {
	Ok(state.link_previewer.analyze_link(&link).await)
}

#[command]
pub async fn get_audio_device_list(state: State<'_, QState>) -> Result<AudioDeviceList, ()> {
	Ok(state.audio_device_list().await)
}

#[derive(Deserialize)]
pub struct StringId(#[serde(deserialize_with = "deserialize_u64")] pub u64);

#[command]
pub async fn identity_create(state: State<'_, QState>) -> Result<ApiIdentity, String> {
	unwrap_send!(state.database, GenrateNewIdentityMsg())
}

#[command]
pub async fn identity_import(state: State<'_, QState>, data: String) -> Result<(), String> {
	match import_ts_identities_from_string(&state, &data).await {
		Ok(_) => Ok(()),
		Err(e) => Err(e.to_string()),
	}
}

#[command]
pub async fn identity_list(
	state: State<'_, QState>, find: FindIdentity,
) -> Result<Vec<ApiIdentity>, String> {
	unwrap_send!(state.database, GetIdentitiesMsg(find))
}

#[command]
pub async fn identity_update(
	state: State<'_, QState>, id: StringId, update: UpdateIdentityOptions,
) -> Result<(), String> {
	unwrap_send!(
		state.database,
		UpdateIdentityMsg(
			FindIdentity::ById(id.0),
			UpdateIdentity { name: update.name, ..Default::default() },
		)
	)
}

#[command]
pub async fn identity_delete(state: State<'_, QState>, id: StringId) -> Result<(), String> {
	unwrap_send!(state.database, DeleteIdentityMsg(FindIdentity::ById(id.0)))
}

#[command]
pub async fn get_mutestate(state: State<'_, QState>) -> Result<MuteStates, ()> {
	Ok(state.get_mute_state().await)
}

#[command]
pub async fn run_hotkey(
	state: State<'_, QState>, action: qint_proxy::hotkey::Action,
) -> Result<(), ()> {
	action.run(&state).await;
	Ok(())
}

#[command]
pub fn plugin_list(state: State<'_, QState>) -> Vec<String> {
	state.plugin_list()
}

#[command]
pub fn plugin_get(state: State<'_, QState>, name: String) -> Result<String, String> {
	state.plugin_get(&name).map_err(|err| err.to_string())
}

#[command]
pub fn plugin_save(state: State<'_, QState>, name: String, content: String) -> Result<(), String> {
	state.plugin_save(&name, &content).map_err(|err| err.to_string())
}

#[command]
pub fn plugin_delete(state: State<'_, QState>, name: String) -> Result<(), String> {
	state.plugin_delete(&name).map_err(|err| err.to_string())
}
