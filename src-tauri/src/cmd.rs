use std::convert::TryInto;
use std::fmt::Write;
use std::path::PathBuf;
use std::{fmt::Debug, sync::Arc};

use actix::Addr;
use actix::MailboxError;
use bytes::{Bytes, BytesMut};
use futures::{Stream, StreamExt};
use juniper::http::GraphQLRequest;
use proxy_codegen::book_events::deserialize_id;
use proxy_codegen::book_events::deserialize_u64;
use qint_proxy::connection::{DownloadFileContext, UploadFile};
use qint_proxy::messages::ResultDetails;
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
use sha2::{Digest, Sha256};
use tauri::{command, Emitter, Manager, State, Window};
use tauri_plugin_dialog::DialogExt;
use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio_util::codec::{BytesCodec, FramedRead};
use tracing::{debug, error, info, warn};
use tsclientlib::Error as TsError;
use tsproto_types::{crypto::EccKeyPubP256, ChannelId};

use crate::audio::LoudnessShare;
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
	con: ConnectionId,
	msg: T,
}

struct WindowBridge {
	window: Window,
	id: ConnectionId,
}

type QState = Arc<QintState>;
type QCore = Arc<QintCore>;
type QCoreAddr = Addr<QintCore>;

impl AppToFrontendBridge for WindowBridge {
	fn send(&self, msg: &MessageP2F) {
		debug!(?msg, "Sending to frontend");
		let res = self.window.emit("ws", &TauriWs { con: self.id, msg });
		if let Err(error) = res {
			warn!(%error, "Failed sending to frontend");
		}
	}

	fn close(&self) {
		let res = self.window.emit("ws_close", self.id);
		if let Err(error) = res {
			warn!(%error, "Failed sending to frontend");
		}
	}
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileRequest {
	con: ConnectionId,
	#[serde(deserialize_with = "deserialize_id")]
	channel: ChannelId,
	path: String,
	#[serde(default)]
	channel_password: Option<String>,
	existing: FileExistsAction,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCacheRequest {
	con: String,
	#[serde(deserialize_with = "deserialize_id")]
	channel: ChannelId,
	path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaRequest {
	con: ConnectionId,
	#[serde(default)]
	channel_password: Option<String>,
	existing: FileExistsAction,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
enum FileExistsAction {
	Error,
	Overwrite,
	Resume,
}
impl FileExistsAction {
	fn overwrite(&self) -> bool {
		*self == FileExistsAction::Overwrite
	}
	fn resume(&self) -> bool {
		*self == FileExistsAction::Resume
	}
}

#[derive(Debug, Serialize)]
pub struct FileTransferStatus {
	con: ConnectionId,
	transfer_handle: u16,
	file_size: usize,
	/// Wording is 'progress' and not 'transferred' since files can be resumed.
	/// This is so that this value is transparent whether a file is transferred
	/// from start or resumed.
	progress_size: usize,
	file_name: String,
	/// in Bytes/s
	transfer_speed: usize,
}

#[derive(Debug, Serialize)]
pub struct FileResponse {
	pub data: Vec<u8>,
	pub mime: Option<String>,
}

#[derive(Deserialize)]
pub struct StringId(#[serde(deserialize_with = "deserialize_u64")] pub u64);

// === CMDS ===

#[command]
pub async fn create_ws(
	core: State<'_, QCoreAddr>, window: Window, con: ConnectionId,
) -> Result<(), Error> {
	info!(id = ?con, "Creating tauri connection");

	let sender = Box::new(WindowBridge { window, id: con.clone() });
	core.send(CreateWs { id: con, sender }).await.unwrap()
}

#[command]
pub fn close_ws(core: State<'_, QCore>, con: ConnectionId) -> Result<(), Error> {
	info!(id = ?con, "Closing tauri connection");
	core.close_ws(CloseWs { id: con })
}

#[command]
pub fn pass_ws_msg(
	core: State<'_, QCore>, con: ConnectionId, msg: MessageF2P,
) -> Result<(), Error> {
	core.ws_msg(DispatchWsMsg { id: con, msg })
}

#[command]
pub async fn db(
	state: State<'_, QState>, request: GraphQLRequest,
) -> Result<serde_json::Value, ()> {
	let res = request.execute(&state.graphql_schema, &*state).await;
	if res.is_ok() {
		Ok(serde_json::to_value(&res).unwrap())
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

async fn collect_to_bytes<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin + 'static>(
	mut stream: S,
) -> Vec<u8> {
	let mut vec = Vec::new();
	while let Some(Ok(b)) = stream.next().await {
		vec.extend_from_slice(&b);
	}
	vec
}

fn format_tx_err<T>(
	res: Result<Result<T, qint_proxy::connection::Error>, MailboxError>,
) -> Result<T, ResultDetails> {
	match res {
		Err(_) => Err("Mailbox error".into()),
		Ok(Err(qint_proxy::connection::Error::TsError(TsError::CommandError(error)))) => {
			debug!(%error, "Common Teamspeak error");
			Err(error.into())
		}
		Ok(Err(error)) => {
			error!(%error, "Unknown Teamspeak error");
			Err(format!("Unknown transfer err: {}", error).into())
		}
		Ok(Ok(r)) => Ok(r),
	}
}

#[command]
pub async fn download_bytes(
	state: State<'_, QState>, req: FileRequest, cache: bool,
) -> Result<FileResponse, ResultDetails> {
	let state = state.inner();
	// move 'cache' into parameter from FileReq
	let FileRequest { con, channel, path, existing, channel_password } = req;

	let con_addr = state.get_connection(&con).ok_or("Connection not found".to_string())?;

	// Lookup in cache
	let server = match con_addr.send(qint_proxy::connection::GetPublicKeyMsg).await {
		Ok(Ok(r)) => r,
		Ok(Err(error)) => {
			error!(%error, "Failed to get server public key");
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

	let DownloadFileContext { size, stream } = format_tx_err(
		con_addr
			.send(qint_proxy::connection::DownloadFile {
				channel,
				path: path.clone(),
				channel_password,
				return_code: None,
				resume: existing.resume(),
			})
			.await,
	)?;

	let stream = FramedRead::new(stream, BytesCodec::new()).map(|r| r.map(BytesMut::freeze));
	let (stream, mime) = guess_content_type(stream).await;

	// Cache for offline usage if smaller than 5 MiB
	let data = if cache && size < 5 * 1024 * 1024 {
		let stream = state.file_cache.cache_file(&server, channel, &path, stream).await;
		collect_to_bytes(stream).await
	} else {
		collect_to_bytes(stream).await
	};
	return Ok(FileResponse { data, mime: mime.map(|s| s.to_string()) });
}

#[command]
pub async fn download_bytes_from_cache(
	state: State<'_, QState>, req: FileCacheRequest,
) -> Result<FileResponse, String> {
	let state = state.inner();
	let FileCacheRequest { con, channel, path } = req;

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
pub async fn upload_bytes(
	core: State<'_, QCore>, state: State<'_, QState>, req: FileRequest, data: Vec<u8>,
) -> Result<(), ResultDetails> {
	let FileRequest { con, channel, path, channel_password, existing } = req;
	// TODO try fetch pw from database

	let prepare =
		core.filetransfer.prepare_upload_from_bytes(data).map_err(|err| err.to_string())?;
	let size = prepare.get_size();

	let con_addr = state.get_connection(&con).ok_or("Connection not found".to_string())?;

	debug!(channel = channel.0, %path, "Uploading file");
	let ctx = format_tx_err(
		con_addr
			.send(qint_proxy::connection::UploadFile {
				channel,
				path,
				channel_password,
				return_code: None,
				overwrite: existing.overwrite(),
				resume: false,
				size,
			})
			.await,
	)?;

	core.filetransfer.add_upload(ctx, prepare);
	Ok(())
}

#[command]
pub async fn read_file(window: Window) -> Result<(String, String), String> {
	let path_buf = tauri::async_runtime::spawn(async move {
		let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
		window.app_handle().dialog().file().add_filter("JavaScript File", &["js"]).pick_file(
			move |p| {
				let _ = tx.send(p.map(|p| p.path));
			},
		);
		rx.recv().unwrap_or(None)
	})
	.await
	.unwrap();
	if let Some(path_buf) = path_buf {
		let content = std::fs::read_to_string(&path_buf).map_err(|err| err.to_string())?;
		let file = if let Some(file) = path_buf.file_name() {
			file.to_string_lossy()
		} else {
			path_buf.to_string_lossy()
		};
		return Ok((file.to_string(), content));
	} else {
		Err("No file selected".to_string())
	}
	//std::fs::read_to_string(filename).map_err(|err| err.to_string())
}

#[command]
pub async fn filetransfer_list() -> Result<Vec<FileTransferStatus>, ()> {
	// state
	// 	.aggregate(|con, _| FileTransferStatus {
	// 		con: con.id,
	// 		file_name: "file.txt".into(),
	// 		file_size: 1024,
	// 		progress_size: 420,
	// 		transfer_handle: 13,
	// 		transfer_speed: 2,
	// 	})
	// 	.collect()
	// 	.await
	Ok(Vec::new())
}

#[command]
pub async fn download_file(
	core: State<'_, QCore>, state: State<'_, QState>, window: Window, req: FileRequest,
) -> Result<(), ResultDetails> {
	let FileRequest { con, channel, existing, path, channel_password } = req;
	// TODO try fetch pw from database

	let con_addr = state.get_connection(&con).ok_or("Connection not found".to_string())?;

	let suggest_file =
		if let Some(i) = path.rfind('/') { &path[(i + 1)..] } else { &path }.to_string();

	let path_buf = tauri::async_runtime::spawn(async move {
		let (tx, rx) = std::sync::mpsc::channel::<Option<PathBuf>>();
		window.app_handle().dialog().file().set_file_name(&suggest_file).save_file(move |p| {
			let _ = tx.send(p);
		});
		rx.recv().unwrap_or(None)
	})
	.await
	.unwrap();

	let local_file = path_buf.ok_or("No file selected".to_string())?;

	let prepare =
		core.filetransfer.prepare_download(&local_file).await.map_err(|err| err.to_string())?;

	let ctx = format_tx_err(
		con_addr
			.send(qint_proxy::connection::DownloadFile {
				channel,
				path: path.clone(),
				channel_password,
				return_code: None,
				resume: existing.resume(),
			})
			.await,
	)?;

	core.filetransfer.add_download(ctx, prepare);
	Ok(()) // TODO
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum UploadFeature {
	Files(#[serde(deserialize_with = "deserialize_id")] ChannelId, String),
	Avatar,
	Icon,
}

async fn ask_for_files(multiple: bool, window: Window) -> Result<Vec<PathBuf>, String> {
	let picked = tauri::async_runtime::spawn(async move {
		let (tx, rx) = std::sync::mpsc::channel::<Vec<PathBuf>>();
		let builder = window.app_handle().dialog().file();
		if multiple {
			builder.pick_files(move |p| {
				let picked =
					p.map(|p| p.into_iter().map(|r| r.path).collect::<Vec<_>>()).unwrap_or_default();
				let _ = tx.send(picked);
			});
		} else {
			builder.pick_file(move |p| {
				let picked = p.map(|p| vec![p.path]).unwrap_or_default();
				let _ = tx.send(picked);
			});
		}
		rx.recv().unwrap_or(Vec::new())
	})
	.await
	.unwrap();
	if picked.len() == 0 {
		return Err("No file selected".to_string());
	} else {
		return Ok(picked);
	}
}

#[command]
pub async fn upload_file(
	core: State<'_, QCore>, state: State<'_, QState>, window: Window, req: FileMetaRequest,
	feature: UploadFeature,
) -> Result<Option<String>, ResultDetails> {
	let FileMetaRequest { con, existing, channel_password, .. } = req;
	// TODO try fetch pw from database

	let con_addr = state.get_connection(&con).ok_or("Connection not found".to_string())?;

	match feature {
		UploadFeature::Files(channel, mut path) => {
			let local_file = ask_for_files(false, window).await?.into_iter().next().unwrap();
			let filename = local_file
				.file_name()
				.map(|p| p.to_string_lossy().to_string())
				.ok_or("No file name ??!?".to_string())?;

			let prepare = core
				.filetransfer
				.prepare_upload(&local_file)
				.await
				.map_err(|err| err.to_string())?;
			let size = prepare.get_size();

			if !path.ends_with('/') {
				path.push('/');
			}
			path.push_str(&filename);

			debug!(?local_file, %path, "Uploading");
			let ctx = format_tx_err(
				con_addr
					.send(UploadFile {
						channel,
						path,
						channel_password,
						return_code: None,
						overwrite: existing.overwrite(),
						resume: existing.resume(),
						size,
					})
					.await,
			)?;

			core.filetransfer.add_upload(ctx, prepare);
			Ok(None)
		}
		UploadFeature::Avatar | UploadFeature::Icon => {
			let local_file = ask_for_files(false, window).await?.into_iter().next().unwrap();
			let mut file = File::open(local_file).await.map_err(|err| err.to_string())?;
			let meta = file.metadata().await.map_err(|err| err.to_string())?;
			let size = meta.len();
			let mut buf = Vec::with_capacity(size as usize);
			file.read_to_end(&mut buf).await.map_err(|err| err.to_string())?;

			let hash_bytes = Sha256::digest(&buf);
			let hash_bytes = hash_bytes.as_slice();

			let path = if feature == UploadFeature::Avatar {
				"/avatar".to_string()
			} else {
				format!("/icon_{}", u32::from_le_bytes(hash_bytes[0..4].try_into().unwrap()))
			};

			debug!(?feature, %path, "Uploading");
			let ctx = format_tx_err(
				con_addr
					.send(UploadFile {
						channel: ChannelId(0),
						path,
						channel_password,
						return_code: None,
						overwrite: true,
						resume: false,
						size,
					})
					.await,
			)?;

			let prepare =
				core.filetransfer.prepare_upload_from_bytes(buf).map_err(|err| err.to_string())?;
			core.filetransfer.add_upload(ctx, prepare);

			if feature == UploadFeature::Avatar {
				let mut hash = String::with_capacity(64);
				for byte in hash_bytes {
					let _ = write!(hash, "{:02X}", byte);
				}
				Ok(Some(hash))
			} else {
				Ok(None)
			}
		}
	}
}

#[command]
pub async fn peek_link(state: State<'_, QState>, link: String) -> Result<AnalyzeResult, ()> {
	Ok(state.link_previewer.analyze_link(&link).await)
}

#[command]
pub async fn get_audio_device_list(state: State<'_, QState>) -> Result<AudioDeviceList, ()> {
	Ok(state.audio_device_list().await)
}

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
pub fn plugin_delete(state: State<QState>, name: String) -> Result<(), String> {
	state.plugin_delete(&name).map_err(|err| err.to_string())
}

#[command]
pub fn markdown(md: String) -> String {
	proxy_codegen::markdown::markdown(&md)
}

#[command]
pub async fn set_loudness_callback(
	state: State<'_, QState>, listener: State<'_, LoudnessShare>, window: Window, enabled: bool,
) -> Result<(), ()> {
	if enabled {
		listener.enable(&state, window).await;
	} else {
		listener.disable()
	}
	Ok(())
}
