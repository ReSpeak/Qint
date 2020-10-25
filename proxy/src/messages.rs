use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tsclientlib::{ChannelId, DisconnectOptions, MessageTarget, Version};
use uuid::Uuid;

use crate::book_events::{JsEvent, JsInMessage, JsM2B};

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	Disconnect(DisconnectOptions),
	SendMessage {
		target: MessageTarget,
		message: String,
	},
	/// Send a TeamSpeak command, for debugging purposes.
	SendCommand(String),
	/// Set the loudness threshold for sending audio in LUFS.
	SetLoudnessThreshold(f64),
	/// Ask the proxy to send loudness data or not.
	SubscribeLoudness(bool),
	/// Change the volume of a client.
	SetClientVolume {
		/// Client uid
		client: Vec<u8>,
		volume: f32,
	},
	/// Change something in the book.
	Change(JsM2B),
}

/// A message sent over a websocket connection from the proxy to the frontend.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum MessageP2F {
	/// The connection failed. The websocket connection should be closed
	/// afterwards.
	Error(String),
	/// We are successfully connected to the server.
	Connected {
		/// The uid of the server.
		server: Vec<u8>,
		/// The id of our own client.
		own_client: String,
	},
	/// The connection to the server was lost. Trying to reconnect automatically.
	DisconnectedTemporarily(),
	/// The list of currently talking client ids and `true` if they are whispering.
	TalkersChanged(Vec<(String, bool)>),
	/// The connection received events.
	Events(Vec<JsEvent>),
	/// The connection received a message.
	Message(JsInMessage),
	Loudness(f64),
	FileList(Vec<JsChannelFile>)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectOptions {
	// TODO as camelCase
	/// Id of the bookmark
	pub bookmark: Option<i64>,
	pub address: String,
	pub name: String,
	pub channel: Option<String>,
	pub version: Version,
	/// Ignore if the identity of the server changed.
	pub ignore_identity_mismatch: bool,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct JsChannelFile {
	// TODO As string
	pub channel_id: ChannelId,
	pub path: String,
	pub name: String,
	pub size: u64,
	pub last_modified: OffsetDateTime,
	pub is_file: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum TauriWsF2P {
	Msg(MessageF2P),
	Close,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TauriWsEventF2P {
	pub connection: Uuid,
	pub msg: TauriWsF2P,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum TauriHttpRequest {
	DownloadFile {
		connection: Uuid,
		channel: u64,
		path: String,
	},
	DownloadCacheFile {
		/// Server public key
		server: Vec<u8>,
		channel: u64,
		path: String,
	},
	GetPlugin(String),
	GetTransientSetting(String),
	Graphql(juniper::http::GraphQLRequest),
	ListPlugins(),
	RunShortcut(crate::shortcut::Action),
	SetTransientSetting(String, serde_json::Value),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TauriHttpRequestWrapper {
	#[serde(flatten)]
	pub req: TauriHttpRequest,
	pub callback: String,
	pub error: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum TauriHttpResponse {
	Graphql(serde_json::Value),
	PluginList(Vec<String>),
	Plugin(String),
	TransientSetting(Option<serde_json::Value>),
	Void(),
}
