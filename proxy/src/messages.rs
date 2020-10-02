use serde::{Deserialize, Serialize};
use tsclientlib::{ClientId, DisconnectOptions, MessageTarget, Version};

use crate::book_events::{JsEvent, JsM2B};

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	Disconnect(DisconnectOptions),
	SendMessage { target: MessageTarget, message: String },
	/// Set the loudness threshold for sending audio in LUFS
	SetLoudnessThreshold(f64),
	/// Ask the proxy to send loudness data or not.
	SubscribeLoudness(bool),
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
		own_client: ClientId,
	},
	/// The connection to the server was lost. Trying to reconnect automatically.
	DisconnectedTemporarily(),
	/// The list of currently talking clients and `true` if they are whispering.
	TalkersChanged(Vec<(ClientId, bool)>),
	/// The connection received events.
	Events(Vec<JsEvent>),
	Loudness(f64),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectOptions {
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
