use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, SocketAddr};

use anyhow::{format_err, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{Duration, OffsetDateTime};
use tsclientlib::data::Connection;
use tsclientlib::events::{Event, ExtraInfo, PropertyId, PropertyValueRef};
use tsclientlib::prelude::*;
use tsclientlib::*;
use tsproto_packets::packets::OutCommand;
use tsproto_types::crypto::EccKeyPubP256;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum JsEvent {
	PropertyAdded {
		id: JsPropertyId,
		prop: JsProperty,
		invoker: Option<Invoker>,
		extra: ExtraInfo,
	},
	PropertyChanged {
		id: JsPropertyId,
		prop: JsProperty,
		invoker: Option<Invoker>,
		extra: ExtraInfo,
	},
	PropertyRemoved {
		id: JsPropertyId,
		invoker: Option<Invoker>,
		extra: ExtraInfo,
	},
	ChannelListFinished,
	Message {
		target: MessageTarget,
		invoker: Invoker,
		message: String,
	},
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum JsPropertyId {
	Channel(ChannelId),
	ChannelGroup(ChannelGroupId),
	Client(ClientId),
	ClientServerGroup(ClientId, ServerGroupId),
	Server,
	ServerIp(IpAddr),
	ServerGroup(ServerGroupId),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum JsProperty {
	Channel(JsChannel),
	ChannelGroup(JsChannelGroup),
	Client(JsClient),
	Server(JsServer),
	IpAddr(IpAddr),
	ServerGroup(JsServerGroup),
	ServerGroupId(ServerGroupId),
}

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	Deserialize::deserialize(deserializer).map(Some)
}

// Serialize OffsetDateTime as unix timestamp with timezone as i32 in seconds
fn deserialize_some_date_time<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<OffsetDateTime>, D::Error> {
	let (ts, offset) = Deserialize::deserialize(deserializer)?;
	Ok(Some(OffsetDateTime::from_unix_timestamp(ts).to_offset(offset)))
}

fn serialize_some_date_time<S: Serializer>(
	datetime: &Option<OffsetDateTime>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.map(|d| (d.timestamp(), d.offset())).serialize(serializer)
}

include!(concat!(env!("OUT_DIR"), "/book_events.rs"));
