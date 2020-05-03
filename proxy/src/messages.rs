use serde::{Deserialize, Serialize};
use tsclientlib::{ChannelId, ClientId, DisconnectOptions, MessageTarget, Version};

use crate::book_events::JsEvent;

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	Disconnect(DisconnectOptions),
	SendMessage { target: MessageTarget, message: String },
	SwitchChannel(ChannelId),
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
		/// The id of the server.
		server: String,
		/// The id of our own client.
		own_client: ClientId,
	},
	/// The list of currently talking clients and `true` if they are whispering.
	TalkersChanged(Vec<(ClientId, bool)>),
	/// The connection received events.
	Events(Vec<JsEvent>),
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectOptions {
	pub address: String,
	pub name: String,
	pub version: Version,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
}
