//! Matrix chat backend. All auth data stays here; the frontend only gets
//! [`MatrixStatus`] and messages.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Result, bail, format_err};
use base64::prelude::*;
use matrix_sdk::authentication::AuthSession;
use matrix_sdk::authentication::matrix::MatrixSession;
use matrix_sdk::authentication::oauth::registration::{
	ApplicationType, ClientMetadata, Localized, OAuthGrantType,
};
use matrix_sdk::authentication::oauth::{ClientId, OAuthSession, UserSession};
use matrix_sdk::config::SyncSettings;
use matrix_sdk::room::MessagesOptions;
use matrix_sdk::ruma::events::room::message::{
	MessageType, OriginalSyncRoomMessageEvent, Relation, RoomMessageEventContent,
};
use matrix_sdk::ruma::events::{AnySyncMessageLikeEvent, AnySyncTimelineEvent};
use matrix_sdk::ruma::serde::Raw;
use matrix_sdk::ruma::{OwnedRoomId, RoomId, uint};
use matrix_sdk::store::RoomLoadSettings;
use matrix_sdk::utils::UrlOrQuery;
use matrix_sdk::utils::local_server::LocalServerBuilder;
use matrix_sdk::{Client, Room, SessionChange};
use serde::{Deserialize, Serialize};
use tokio::runtime::Handle;
use tokio::sync::{Mutex, broadcast, oneshot};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::secret::Secret;

const SESSION_FILENAME: &str = "matrix_session.bin";
const STORE_DIRNAME: &str = "matrix";
const DEVICE_NAME: &str = "Qint";
/// Shown to the user by the OAuth server during login.
const CLIENT_URI: &str = "https://github.com/ReSpeak/Qint";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixStatus {
	pub logged_in: bool,
	pub user_id: Option<String>,
	pub homeserver: Option<String>,
	pub room_id: Option<String>,
	pub rooms: Vec<MatrixRoom>,
}

#[derive(Clone, Debug, Serialize)]
pub struct MatrixRoom {
	pub id: String,
	pub name: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMessage {
	pub id: String,
	pub room_id: String,
	pub sender: String,
	pub sender_name: String,
	pub body: String,
	/// Our own markdown rendering of `body`, never the remote `formatted_body`.
	pub rendered: String,
	/// Milliseconds since unix epoch
	pub time: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixMessages {
	pub messages: Vec<MatrixMessage>,
	pub end_token: Option<String>,
}

/// Pushed to every frontend.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum MatrixEvent {
	Status,
	Message(MatrixMessage),
}

/// Exactly one of `session` (legacy Matrix auth) and `oauth` is set.
#[derive(Serialize, Deserialize)]
struct SavedSession {
	homeserver: String,
	#[serde(default)]
	session: Option<MatrixSession>,
	#[serde(default)]
	oauth: Option<SavedOAuth>,
	room: Option<OwnedRoomId>,
}

#[derive(Serialize, Deserialize)]
struct SavedOAuth {
	client_id: String,
	user: UserSession,
}

struct LoggedIn {
	client: Client,
	homeserver: String,
	room: Option<OwnedRoomId>,
	/// Sync and session token watcher
	tasks: Vec<JoinHandle<()>>,
}

impl LoggedIn {
	fn stop(&self) {
		for t in &self.tasks {
			t.abort();
		}
	}
}

pub struct Matrix {
	config_path: PathBuf,
	secret: Secret,
	handle: Handle,
	inner: Mutex<Option<LoggedIn>>,
	/// A browser login waiting for its redirect
	pending_login: Mutex<Option<JoinHandle<()>>>,
	pub events: broadcast::Sender<MatrixEvent>,
}

impl Matrix {
	pub fn new(config_path: PathBuf, secret: Secret, handle: Handle) -> Arc<Self> {
		let (events, _) = broadcast::channel(64);
		Arc::new(Self {
			config_path,
			secret,
			handle,
			inner: Mutex::new(None),
			pending_login: Mutex::new(None),
			events,
		})
	}

	fn session_path(&self) -> PathBuf { self.config_path.join(SESSION_FILENAME) }
	fn store_path(&self) -> PathBuf { self.config_path.join(STORE_DIRNAME) }

	fn notify_status(&self) { let _ = self.events.send(MatrixEvent::Status); }

	async fn build_client(&self, homeserver: &str) -> Result<Client> {
		// The store holds the e2ee keys, encrypt it with our local secret.
		let passphrase = BASE64_STANDARD.encode(self.secret.0.as_slice());
		Ok(Client::builder()
			.server_name_or_homeserver_url(homeserver)
			.sqlite_store(self.store_path(), Some(&passphrase))
			// OAuth servers hand out short-lived access tokens
			.handle_refresh_tokens()
			.build()
			.await?)
	}

	fn save(&self, l: &LoggedIn) -> Result<()> {
		let mut saved = SavedSession {
			homeserver: l.homeserver.clone(),
			session: None,
			oauth: None,
			room: l.room.clone(),
		};
		match l.client.session() {
			Some(AuthSession::Matrix(s)) => saved.session = Some(s),
			Some(AuthSession::OAuth(s)) => {
				saved.oauth = Some(SavedOAuth { client_id: s.client_id.as_str().into(), user: s.user })
			}
			_ => bail!("No matrix session"),
		}
		let data = self.secret.seal(serde_json::to_vec(&saved)?)?;
		std::fs::write(self.session_path(), data)?;
		Ok(())
	}

	/// Restore a saved session on startup.
	pub async fn restore(self: Arc<Self>) {
		let data = match std::fs::read(self.session_path()) {
			Ok(d) => d,
			Err(_) => return,
		};
		let res: Result<()> = async {
			let saved: SavedSession = serde_json::from_slice(&self.secret.open(data)?)?;
			let client = self.build_client(&saved.homeserver).await?;
			if let Some(o) = saved.oauth {
				let session = OAuthSession { client_id: ClientId::new(o.client_id), user: o.user };
				client.oauth().restore_session(session, RoomLoadSettings::default()).await?;
			} else if let Some(s) = saved.session {
				client.matrix_auth().restore_session(s, RoomLoadSettings::default()).await?;
			} else {
				bail!("Saved matrix session is empty");
			}
			self.set_logged_in(client, saved.homeserver, saved.room).await;
			Ok(())
		}
		.await;
		if let Err(error) = res {
			warn!(%error, "Failed to restore matrix session");
		}
	}

	async fn set_logged_in(
		self: &Arc<Self>, client: Client, homeserver: String, room: Option<OwnedRoomId>,
	) {
		let sync = self.handle.spawn(run_sync(client.clone(), self.events.clone()));
		let watch = self.handle.spawn(self.clone().watch_session(client.clone()));
		let l = LoggedIn { client, homeserver, room, tasks: vec![sync, watch] };
		if let Err(error) = self.save(&l) {
			warn!(%error, "Failed to save matrix session");
		}
		if let Some(old) = self.inner.lock().await.replace(l) {
			old.stop();
		}
		self.notify_status();
	}

	/// Persist refreshed tokens, otherwise the next start has a dead session.
	async fn watch_session(self: Arc<Self>, client: Client) {
		use tokio::sync::broadcast::error::RecvError;
		let mut changes = client.subscribe_to_session_changes();
		loop {
			match changes.recv().await {
				Ok(SessionChange::TokensRefreshed) => {
					if let Some(l) = &*self.inner.lock().await {
						if let Err(error) = self.save(l) {
							warn!(%error, "Failed to save refreshed matrix session");
						}
					}
				}
				// shortcut: the user has to log out and in again manually, add a
				// re-login prompt if this happens often.
				Ok(SessionChange::UnknownToken(_)) => warn!("Matrix session got invalidated"),
				Err(RecvError::Lagged(_)) => {}
				Err(RecvError::Closed) => break,
			}
		}
	}

	/// Start with a clean store, an old one may belong to another account.
	async fn fresh_client(&self, homeserver: &str) -> Result<Client> {
		if let Some(pending) = self.pending_login.lock().await.take() {
			pending.abort();
		}
		if let Some(old) = self.inner.lock().await.take() {
			old.stop();
		}
		let _ = std::fs::remove_dir_all(self.store_path());
		self.build_client(homeserver).await
	}

	pub async fn login_password(self: &Arc<Self>, homeserver: &str, user: &str, password: &str) -> Result<()> {
		let client = self.fresh_client(homeserver).await?;
		client
			.matrix_auth()
			.login_username(user, password)
			.initial_device_display_name(DEVICE_NAME)
			.send()
			.await?;
		self.set_logged_in(client, homeserver.into(), None).await;
		Ok(())
	}

	/// Returns the url the user has to open in a browser. The login finishes in
	/// the background via a localhost redirect and emits a status event.
	///
	/// Uses OAuth 2.0 if the server supports it, the legacy SSO redirect otherwise.
	pub async fn login_sso(self: Arc<Self>, homeserver: &str) -> Result<String> {
		let client = self.fresh_client(homeserver).await?;
		let homeserver = homeserver.to_string();
		if client.oauth().server_metadata().await.is_ok() {
			return self.login_oauth(client, homeserver).await;
		}

		let (tx, rx) = oneshot::channel();
		let this = self.clone();
		let task = self.handle.spawn(async move {
			let res = client
				.matrix_auth()
				.login_sso(|url| async move {
					let _ = tx.send(url);
					Ok(())
				})
				.initial_device_display_name(DEVICE_NAME)
				.send()
				.await;
			match res {
				Ok(_) => this.set_logged_in(client, homeserver, None).await,
				Err(error) => warn!(%error, "Matrix SSO login failed"),
			}
		});
		*self.pending_login.lock().await = Some(task);
		rx.await.map_err(|_| format_err!("SSO login failed to start"))
	}

	async fn login_oauth(self: Arc<Self>, client: Client, homeserver: String) -> Result<String> {
		let (redirect_uri, redirect) = LocalServerBuilder::new().spawn().await?;
		// Registered anew on every login, the server allows any loopback port
		// for native clients.
		let metadata = ClientMetadata {
			client_name: Some(Localized::new(DEVICE_NAME.to_string(), [])),
			..ClientMetadata::new(
				ApplicationType::Native,
				vec![OAuthGrantType::AuthorizationCode { redirect_uris: vec![redirect_uri.clone()] }],
				Localized::new(url::Url::parse(CLIENT_URI)?, []),
			)
		};
		let auth = client
			.oauth()
			.login(redirect_uri, None, Some(Raw::new(&metadata)?.into()), None)
			.build()
			.await?;

		let this = self.clone();
		let task = self.handle.spawn(async move {
			let res: Result<()> = async {
				let query = redirect.await.ok_or_else(|| format_err!("No OAuth redirect received"))?;
				client.oauth().finish_login(UrlOrQuery::Query(query.0)).await?;
				Ok(())
			}
			.await;
			match res {
				Ok(()) => this.set_logged_in(client, homeserver, None).await,
				Err(error) => warn!(%error, "Matrix OAuth login failed"),
			}
		});
		*self.pending_login.lock().await = Some(task);
		Ok(auth.url.to_string())
	}

	pub async fn logout(&self) -> Result<()> {
		if let Some(pending) = self.pending_login.lock().await.take() {
			pending.abort();
		}
		if let Some(l) = self.inner.lock().await.take() {
			l.stop();
			if let Err(error) = l.client.logout().await {
				warn!(%error, "Failed to log out on matrix server");
			}
		}
		let _ = std::fs::remove_file(self.session_path());
		let _ = std::fs::remove_dir_all(self.store_path());
		self.notify_status();
		Ok(())
	}

	pub async fn status(&self) -> MatrixStatus {
		let inner = self.inner.lock().await;
		let Some(l) = &*inner else {
			return MatrixStatus {
				logged_in: false,
				user_id: None,
				homeserver: None,
				room_id: None,
				rooms: Vec::new(),
			};
		};
		let mut rooms = Vec::new();
		for room in l.client.joined_rooms() {
			let name = match room.display_name().await {
				Ok(n) => n.to_string(),
				Err(_) => room.room_id().to_string(),
			};
			rooms.push(MatrixRoom { id: room.room_id().to_string(), name });
		}
		rooms.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
		MatrixStatus {
			logged_in: true,
			user_id: l.client.user_id().map(|u| u.to_string()),
			homeserver: Some(l.homeserver.clone()),
			room_id: l.room.as_ref().map(|r| r.to_string()),
			rooms,
		}
	}

	pub async fn set_room(&self, room_id: &str) -> Result<()> {
		let room = RoomId::parse(room_id)?;
		{
			let mut inner = self.inner.lock().await;
			let l = inner.as_mut().ok_or_else(|| format_err!("Not logged in to matrix"))?;
			l.room = Some(room);
			self.save(l)?;
		}
		self.notify_status();
		Ok(())
	}

	async fn room(&self) -> Result<Room> {
		let inner = self.inner.lock().await;
		let l = inner.as_ref().ok_or_else(|| format_err!("Not logged in to matrix"))?;
		let Some(id) = &l.room else { bail!("No matrix room selected") };
		l.client.get_room(id).ok_or_else(|| format_err!("Matrix room not found"))
	}

	/// Older messages, newest first. Pass the previous `end_token` to continue.
	pub async fn messages(&self, from: Option<String>) -> Result<MatrixMessages> {
		let room = self.room().await?;
		let mut opts = MessagesOptions::backward().from(from.as_deref());
		opts.limit = uint!(50);
		let resp = room.messages(opts).await?;
		let mut messages = Vec::new();
		for ev in resp.chunk {
			let Some(id) = ev.event_id() else { continue };
			let msg = match ev.raw().deserialize() {
				Ok(AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomMessage(m))) => {
					match m.as_original() {
						Some(m) => convert(&room, m).await,
						None => None,
					}
				}
				Ok(AnySyncTimelineEvent::MessageLike(AnySyncMessageLikeEvent::RoomEncrypted(m))) => {
					let body = "🔒 Unable to decrypt message".to_string();
					Some(MatrixMessage {
						id: id.to_string(),
						room_id: room.room_id().to_string(),
						sender_name: sender_name(&room, m.sender()).await,
						sender: m.sender().to_string(),
						rendered: proxy_codegen::markdown::markdown(&body),
						body,
						time: m.origin_server_ts().get().into(),
					})
				}
				_ => None,
			};
			messages.extend(msg);
		}
		Ok(MatrixMessages { messages, end_token: resp.end })
	}

	pub async fn send(&self, text: &str) -> Result<()> {
		let room = self.room().await?;
		room.send(RoomMessageEventContent::text_markdown(text)).await?;
		Ok(())
	}
}

async fn sender_name(room: &Room, user: &matrix_sdk::ruma::UserId) -> String {
	match room.get_member_no_sync(user).await {
		Ok(Some(m)) => m.name().to_string(),
		_ => user.to_string(),
	}
}

/// Only text messages for now, edits are skipped.
async fn convert(room: &Room, ev: &OriginalSyncRoomMessageEvent) -> Option<MatrixMessage> {
	if matches!(ev.content.relates_to, Some(Relation::Replacement(_))) {
		return None;
	}
	let body = match &ev.content.msgtype {
		MessageType::Text(t) => t.body.clone(),
		MessageType::Notice(t) => t.body.clone(),
		MessageType::Emote(t) => format!("*{}*", t.body),
		_ => return None,
	};
	Some(MatrixMessage {
		id: ev.event_id.to_string(),
		room_id: room.room_id().to_string(),
		sender: ev.sender.to_string(),
		sender_name: sender_name(room, &ev.sender).await,
		rendered: proxy_codegen::markdown::markdown(&body),
		body,
		time: ev.origin_server_ts.get().into(),
	})
}

async fn run_sync(client: Client, events: broadcast::Sender<MatrixEvent>) {
	// First sync without handler, so old timeline events are not pushed as new.
	let settings = match client.sync_once(SyncSettings::default()).await {
		Ok(r) => SyncSettings::default().token(r.next_batch),
		Err(error) => {
			warn!(%error, "Initial matrix sync failed");
			SyncSettings::default()
		}
	};
	let _ = events.send(MatrixEvent::Status);

	let ev_events = events.clone();
	client.add_event_handler(move |ev: OriginalSyncRoomMessageEvent, room: Room| {
		let events = ev_events.clone();
		async move {
			if let Some(msg) = convert(&room, &ev).await {
				let _ = events.send(MatrixEvent::Message(msg));
			}
		}
	});
	info!("Matrix sync started");
	if let Err(error) = client.sync(settings).await {
		warn!(%error, "Matrix sync stopped");
	}
}
