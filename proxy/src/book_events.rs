use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};

use anyhow::{format_err, Result};
use serde::de::Error as SerdeError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{Duration, OffsetDateTime};
use tsclientlib::data::Connection;
use tsclientlib::events::{Event, ExtraInfo, PropertyId, PropertyValueRef};
use tsclientlib::prelude::*;
use tsclientlib::TsError as Error;
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
	Channel(
		#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")] ChannelId,
	),
	ChannelGroup(
		#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")]
		ChannelGroupId,
	),
	Client(#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")] ClientId),
	ClientServerGroup(
		#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")] ClientId,
		#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")]
		ServerGroupId,
	),
	Server,
	ServerIp(IpAddr),
	ServerGroup(
		#[serde(deserialize_with = "deserialize_id", serialize_with = "serialize_id")]
		ServerGroupId,
	),
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

trait Id {
	fn to_string_id(&self) -> String;
	fn parse_id(s: &str) -> Result<Self>
	where Self: Sized;
}

impl Id for ChannelGroupId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

impl Id for ChannelId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

impl Id for ClientDbId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

impl Id for ClientId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

impl Id for IconId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

impl Id for ServerGroupId {
	fn to_string_id(&self) -> String { self.0.to_string() }
	fn parse_id(s: &str) -> Result<Self> { Ok(Self(s.parse()?)) }
}

// Any value that is present is considered Some value, including null.
fn deserialize_some<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	Deserialize::deserialize(deserializer).map(Some)
}

// Serialize OffsetDateTime as unix timestamp with timezone as i32 in seconds
fn deserialize_date_time<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<OffsetDateTime, D::Error> {
	let (ts, offset) = Deserialize::deserialize(deserializer)?;
	Ok(OffsetDateTime::from_unix_timestamp(ts).to_offset(offset))
}

fn serialize_date_time<S: Serializer>(
	datetime: &OffsetDateTime, serializer: S,
) -> Result<S::Ok, S::Error> {
	(datetime.timestamp(), datetime.offset()).serialize(serializer)
}

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

// Serialize Duration as seconds + nanoseconds (default serializer)
fn deserialize_duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Duration, D::Error> {
	Ok(Deserialize::deserialize(deserializer)?)
}

fn serialize_duration<S: Serializer>(
	datetime: &Duration, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

fn deserialize_some_duration<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Duration>, D::Error> {
	Ok(Some(Deserialize::deserialize(deserializer)?))
}

fn serialize_some_duration<S: Serializer>(
	datetime: &Option<Duration>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

fn deserialize_some_some_duration<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Option<Duration>>, D::Error> {
	Ok(Some(Deserialize::deserialize(deserializer)?))
}

fn serialize_some_some_duration<S: Serializer>(
	datetime: &Option<Option<Duration>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

fn deserialize_i64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(s.parse().map_err(SerdeError::custom)?)
}

fn serialize_i64<S: Serializer>(i: &i64, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string().serialize(serializer)
}

fn deserialize_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(s.parse().map_err(SerdeError::custom)?)
}

fn serialize_u64<S: Serializer>(i: &u64, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string().serialize(serializer)
}

fn deserialize_some_u64<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<u64>, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.parse().map_err(SerdeError::custom)?))
}

fn serialize_some_u64<S: Serializer>(i: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error> {
	i.map(|i| i.to_string()).serialize(serializer)
}

fn deserialize_some_some_u64<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Option<u64>>, D::Error> {
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.map(|s| s.parse()).transpose().map_err(SerdeError::custom)?))
}

fn serialize_some_some_u64<S: Serializer>(
	i: &Option<Option<u64>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.map(|i| i.map(|i| i.to_string())).serialize(serializer)
}

fn deserialize_id<'de, D: Deserializer<'de>, T: Id>(deserializer: D) -> Result<T, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(T::parse_id(&s).map_err(SerdeError::custom)?)
}

fn serialize_id<S: Serializer, T: Id>(i: &T, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string_id().serialize(serializer)
}

fn deserialize_some_id<'de, D: Deserializer<'de>, T: Id>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(Some(T::parse_id(&s).map_err(SerdeError::custom)?))
}

fn serialize_some_id<S: Serializer, T: Id>(
	i: &Option<T>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref().map(|i| i.to_string_id()).serialize(serializer)
}

fn deserialize_some_some_id<'de, D: Deserializer<'de>, T: Id>(
	deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.map(|s| T::parse_id(&s)).transpose().map_err(SerdeError::custom)?))
}

fn serialize_some_some_id<S: Serializer, T: Id>(
	i: &Option<Option<T>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref().map(|i| i.as_ref().map(|i| i.to_string_id())).serialize(serializer)
}

fn deserialize_some_set_id<'de, D: Deserializer<'de>, T: Eq + Hash + Id>(
	deserializer: D,
) -> Result<Option<HashSet<T>>, D::Error> {
	let s: HashSet<String> = Deserialize::deserialize(deserializer)?;
	Ok(Some(
		s.into_iter()
			.map(|s| T::parse_id(&s))
			.collect::<Result<HashSet<T>>>()
			.map_err(SerdeError::custom)?,
	))
}

fn serialize_some_set_id<S: Serializer, T: Eq + Hash + Id>(
	i: &Option<HashSet<T>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref()
		.map(|i| i.iter().map(|i| i.to_string_id()).collect::<HashSet<String>>())
		.serialize(serializer)
}

include!(concat!(env!("OUT_DIR"), "/book_events.rs"));
