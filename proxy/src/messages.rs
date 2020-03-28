use serde::{Deserialize, Serialize};
use tsclientlib::{ClientId, MessageTarget, Version};

/// A message sent over a websocket connection from the frontend to the proxy.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MessageF2P {
	Connect(ConnectOptions),
	SendMessage {
		target: MessageTarget,
		message: String,
	}
}

/// A message sent over a websocket connection from the proxy to the frontend.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MessageP2F {
	/// The connection failed. The websocket connection should be closed
	/// afterwards.
	Error(String),
	/// The list of currently talking clients.
	TalkersChanged(Vec<ClientId>),
	/// The connection received events.
	Events(), // TODO
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConnectOptions {
	pub address: String,
	pub name: String,
	pub version: Version,
	pub log_commands: bool,
	pub log_packets: bool,
	pub log_udp_packets: bool,
}
