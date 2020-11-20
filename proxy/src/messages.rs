use serde::{Deserialize, Serialize};
use tsclientlib::{ClientId, DisconnectOptions, MessageTarget, Permission, TsError, Version};

use crate::book_events::{
	deserialize_id, deserialize_some_u64, serialize_id, serialize_some_u64, JsEvent, JsInMessage,
	JsM2B,
};

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(test, derive(Serialize))]
#[serde(deny_unknown_fields)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	Disconnect(DisconnectOptions),
	SendMessage {
		target: JsMessageTarget,
		message: String,
		#[serde(default, rename = "returnCode", skip_serializing_if = "Option::is_none")]
		return_code: Option<String>,
	},
	/// Send a TeamSpeak command, for debugging purposes.
	SendCommand {
		command: String,
		#[serde(default, rename = "returnCode", skip_serializing_if = "Option::is_none")]
		return_code: Option<String>,
	},
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
	Change {
		change: JsM2B,
		#[serde(default, rename = "returnCode", skip_serializing_if = "Option::is_none")]
		return_code: Option<String>,
	},
}

/// A message sent over a websocket connection from the proxy to the frontend.
#[derive(Debug, Serialize)]
#[cfg_attr(test, derive(Deserialize))]
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
		#[serde(rename = "ownClient")]
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
	Result {
		#[serde(rename = "returnCode")]
		return_code: String,
		#[serde(default, rename = "tsResult", skip_serializing_if = "Option::is_none")]
		ts_result: Option<TsError>,
		#[serde(default, rename = "missingPermission", skip_serializing_if = "Option::is_none")]
		missing_permission: Option<Permission>,
		/// Description for non-ts errors
		#[serde(default, skip_serializing_if = "Option::is_none")]
		description: Option<String>,
	},
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum JsMessageTarget {
	Server,
	Channel,
	Client(
		#[serde(default, deserialize_with = "deserialize_id", serialize_with = "serialize_id")]
		ClientId,
	),
	Poke(
		#[serde(default, deserialize_with = "deserialize_id", serialize_with = "serialize_id")]
		ClientId,
	),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ConnectOptions {
	/// Id of the bookmark
	#[serde(
		default,
		deserialize_with = "deserialize_some_u64",
		serialize_with = "serialize_some_u64",
		skip_serializing_if = "Option::is_none"
	)]
	pub bookmark: Option<u64>,
	pub address: String,
	pub name: String,
	pub channel: Option<String>,
	pub version: Version,
	pub input_muted: Option<bool>,
	pub output_muted: Option<bool>,
	pub away: Option<String>,
	/// Ignore if the identity of the server changed.
	pub ignore_identity_mismatch: bool,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
}

impl From<MessageTarget> for JsMessageTarget {
	fn from(target: MessageTarget) -> Self {
		match target {
			MessageTarget::Server => Self::Server,
			MessageTarget::Channel => Self::Channel,
			MessageTarget::Client(id) => Self::Client(id),
			MessageTarget::Poke(id) => Self::Poke(id),
		}
	}
}

impl Into<MessageTarget> for JsMessageTarget {
	fn into(self) -> MessageTarget {
		match self {
			Self::Server => MessageTarget::Server,
			Self::Channel => MessageTarget::Channel,
			Self::Client(id) => MessageTarget::Client(id),
			Self::Poke(id) => MessageTarget::Poke(id),
		}
	}
}
