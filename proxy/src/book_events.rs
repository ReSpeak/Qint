use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Deserializer, Serialize};
use time::{Duration, OffsetDateTime};
use tsclientlib::data::Connection;
use tsclientlib::events::{Event, ExtraInfo, PropertyId, PropertyValueRef};
use tsclientlib::*;
use tsproto_types::crypto::EccKeyPubP256;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum JsEvent {
	PropertyAdded { id: JsPropertyId, prop: JsProperty, invoker: Option<Invoker>, extra: ExtraInfo },
	PropertyChanged { id: JsPropertyId, prop: JsProperty, invoker: Option<Invoker>, extra: ExtraInfo },
	PropertyRemoved { id: JsPropertyId, invoker: Option<Invoker>, extra: ExtraInfo },
	ChannelListFinished,
	Message { target: MessageTarget, invoker: Invoker, message: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JsPropertyId {
	Channel(ChannelId),
	ChannelGroup(ChannelGroupId),
	Client(ClientId),
	ClientServerGroup(ClientId, ServerGroupId),
	Server,
	ServerGroup(ServerGroupId),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum JsProperty {
	Channel(JsChannel),
	ChannelGroup(JsChannelGroup),
	Client(JsClient),
	Server(JsServer),
	ServerGroup(JsServerGroup),
}

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T: Deserialize<'de>, D: Deserializer<'de>>(deserializer: D) -> Result<Option<T>, D::Error> {
	Deserialize::deserialize(deserializer).map(Some)
}

include!(concat!(env!("OUT_DIR"), "/book_events.rs"));
