use std::{fmt::Debug, sync::Arc};

use actix::Addr;
use anyhow::bail;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use juniper::http::GraphQLRequest;
use proxy_codegen::book_events::deserialize_id;
use qint_proxy::{
	filecache::guess_content_type,
	link_previewer::AnalyzeResult,
	messages::{MessageF2P, MessageP2F},
	shared::AudioDeviceList,
	AppToFrontendBridge, ConnectionId, QintState, Settings, SettingsUpdate,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use slog::{debug, info, warn, Logger};
use tauri::{command, State, Window};
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::Error as TsError;
use tsproto_types::{crypto::EccKeyPubP256, ChannelId};
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
	Ok(qint_proxy::shared::audio_device_list(&**state).await)
}
