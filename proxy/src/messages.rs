use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;

use proxy_codegen::book_events::{
	deserialize_id, deserialize_set_id, deserialize_some_u64, serialize_id, serialize_set_id,
	serialize_some_u64, JsEvent, JsInMessage, JsM2B,
};
use serde::{Deserialize, Serialize};
use tsclientlib::{
	ChannelId, ClientId, CommandError, DisconnectOptions, Error as TsclError, MessageTarget,
	Permission, TsError, UidBuf, Version,
};

use super::connection::Error as WsError;

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
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
	/// Change the volume of a client.
	SetClientVolume {
		/// Client uid
		client: Vec<u8>,
		volume: f32,
	},
	/// Start or stop whispering to the specified targets.
	SetWhispering(Option<WhisperData>),
	/// Change something in the book.
	Change {
		change: JsM2B,
		#[serde(default, rename = "returnCode", skip_serializing_if = "Option::is_none")]
		return_code: Option<String>,
	},
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
	/// Loudness of clients that are currently talking.
	Loudnesses(HashMap<String, f32>),
	Result(ResultStruct),
}

// Has to be an extra struct for flatten to work
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ResultStruct {
	pub return_code: String,
	#[serde(flatten)]
	pub details: ResultDetails,
}

/// Usually, either `ts_result` (and optionally `missing_permission`) or `description` is set.
/// If all are `None`, the result is success.
/// `lib` provides more information if
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResultDetails {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub ts_result: Option<TsError>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub missing_permission: Option<Permission>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub description: Option<String>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub lib: Option<LibError>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LibError {
	ServerUidMismatch { actual: UidBuf },
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
#[serde(deny_unknown_fields)]
pub struct WhisperData {
	#[serde(default, deserialize_with = "deserialize_set_id", serialize_with = "serialize_set_id")]
	pub clients: HashSet<ClientId>,
	#[serde(default, deserialize_with = "deserialize_set_id", serialize_with = "serialize_set_id")]
	pub channels: HashSet<ChannelId>,
}

#[derive(Clone, Debug, Deserialize, Default, Serialize)]
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
	/// Id of the identity
	#[serde(
		default,
		deserialize_with = "deserialize_some_u64",
		serialize_with = "serialize_some_u64",
		skip_serializing_if = "Option::is_none"
	)]
	pub identity_id: Option<u64>,
	pub address: String,
	pub name: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub channel: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub version: Option<Version>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub input_muted: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub input_hardware_enabled: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub output_muted: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub output_hardware_enabled: Option<bool>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub away: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub password: Option<String>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub channel_password: Option<String>,
	/// Ignore if the identity of the server changed.
	pub ignore_identity_mismatch: bool,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
	pub return_code: Option<String>,
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

impl ResultDetails {
	pub fn ok() -> Self { Self { ts_result: Some(TsError::Ok), ..Default::default() } }
	pub fn from_desc(error: String) -> Self {
		Self { description: Some(error), ..Default::default() }
	}
	pub fn from_str(error: &str) -> Self { Self::from_desc(error.to_string()) }
}

impl<'a, T> TryFrom<&'a Result<T, WsError>> for ResultDetails {
	type Error = &'a WsError;
	fn try_from(err: &'a Result<T, WsError>) -> Result<Self, &'a WsError> {
		if let Err(wserr) = err {
			if let WsError::TsError(TsclError::CommandError(ceerr)) = wserr {
				Ok(ceerr.into())
			} else {
				Err(wserr)
			}
		} else {
			Ok(Self::ok())
		}
	}
}

impl From<CommandError> for ResultDetails {
	fn from(err: CommandError) -> Self { (&err).into() }
}

impl From<&CommandError> for ResultDetails {
	fn from(err: &CommandError) -> Self {
		Self {
			ts_result: Some(err.error),
			missing_permission: err.missing_permission,
			description: None,
			lib: None,
		}
	}
}

impl<T> From<Result<T, CommandError>> for ResultDetails {
	fn from(err: Result<T, CommandError>) -> Self {
		if let Err(err) = err { err.into() } else { Self::ok() }
	}
}

impl From<String> for ResultDetails {
	fn from(err: String) -> Self { Self::from_desc(err) }
}

impl From<&str> for ResultDetails {
	fn from(err: &str) -> Self { Self::from_str(err) }
}
