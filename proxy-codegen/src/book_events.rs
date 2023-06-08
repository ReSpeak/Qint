use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};
use std::hash::Hash;
use std::net::IpAddr;

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

#[derive(Clone, Debug, Deserialize, Serialize)]
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
	Message {
		target: MessageTarget,
		invoker: Invoker,
		message: String,
	},
}

pub trait Id {
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
pub fn deserialize_some<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	Deserialize::deserialize(deserializer).map(Some)
}

// Serialize OffsetDateTime as unix timestamp with timezone as i32 in seconds
pub fn deserialize_date_time<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<OffsetDateTime, D::Error> {
	let (ts, offset) = Deserialize::deserialize(deserializer)?;
	Ok(OffsetDateTime::from_unix_timestamp(ts).map_err(serde::de::Error::custom)?.to_offset(offset))
}

pub fn serialize_date_time<S: Serializer>(
	datetime: &OffsetDateTime, serializer: S,
) -> Result<S::Ok, S::Error> {
	(datetime.unix_timestamp(), datetime.offset()).serialize(serializer)
}

pub fn deserialize_some_date_time<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<OffsetDateTime>, D::Error> {
	let (ts, offset) = Deserialize::deserialize(deserializer)?;
	Ok(Some(
		OffsetDateTime::from_unix_timestamp(ts)
			.map_err(serde::de::Error::custom)?
			.to_offset(offset),
	))
}

pub fn serialize_some_date_time<S: Serializer>(
	datetime: &Option<OffsetDateTime>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.map(|d| (d.unix_timestamp(), d.offset())).serialize(serializer)
}

// Serialize Duration as seconds + nanoseconds (default serializer)
pub fn deserialize_duration<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Duration, D::Error> {
	Ok(Deserialize::deserialize(deserializer)?)
}

pub fn serialize_duration<S: Serializer>(
	datetime: &Duration, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

pub fn deserialize_some_duration<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Duration>, D::Error> {
	Ok(Some(Deserialize::deserialize(deserializer)?))
}

pub fn serialize_some_duration<S: Serializer>(
	datetime: &Option<Duration>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

pub fn deserialize_some_some_duration<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Option<Duration>>, D::Error> {
	Ok(Some(Deserialize::deserialize(deserializer)?))
}

pub fn serialize_some_some_duration<S: Serializer>(
	datetime: &Option<Option<Duration>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	datetime.serialize(serializer)
}

pub fn deserialize_i64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(s.parse().map_err(SerdeError::custom)?)
}

pub fn serialize_i64<S: Serializer>(i: &i64, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string().serialize(serializer)
}

pub fn deserialize_u64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(s.parse().map_err(SerdeError::custom)?)
}

pub fn serialize_u64<S: Serializer>(i: &u64, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string().serialize(serializer)
}

pub fn deserialize_some_u64<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<u64>, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.parse().map_err(SerdeError::custom)?))
}

pub fn serialize_some_u64<S: Serializer>(
	i: &Option<u64>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.map(|i| i.to_string()).serialize(serializer)
}

pub fn deserialize_some_some_u64<'de, D: Deserializer<'de>>(
	deserializer: D,
) -> Result<Option<Option<u64>>, D::Error> {
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.map(|s| s.parse()).transpose().map_err(SerdeError::custom)?))
}

pub fn serialize_some_some_u64<S: Serializer>(
	i: &Option<Option<u64>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.map(|i| i.map(|i| i.to_string())).serialize(serializer)
}

pub fn deserialize_id<'de, D: Deserializer<'de>, T: Id>(deserializer: D) -> Result<T, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(T::parse_id(&s).map_err(SerdeError::custom)?)
}

pub fn serialize_id<S: Serializer, T: Id>(i: &T, serializer: S) -> Result<S::Ok, S::Error> {
	i.to_string_id().serialize(serializer)
}

pub fn deserialize_some_id<'de, D: Deserializer<'de>, T: Id>(
	deserializer: D,
) -> Result<Option<T>, D::Error> {
	let s: String = Deserialize::deserialize(deserializer)?;
	Ok(Some(T::parse_id(&s).map_err(SerdeError::custom)?))
}

pub fn serialize_some_id<S: Serializer, T: Id>(
	i: &Option<T>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref().map(|i| i.to_string_id()).serialize(serializer)
}

pub fn deserialize_some_some_id<'de, D: Deserializer<'de>, T: Id>(
	deserializer: D,
) -> Result<Option<Option<T>>, D::Error> {
	let s: Option<String> = Deserialize::deserialize(deserializer)?;
	Ok(Some(s.map(|s| T::parse_id(&s)).transpose().map_err(SerdeError::custom)?))
}

pub fn serialize_some_some_id<S: Serializer, T: Id>(
	i: &Option<Option<T>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref().map(|i| i.as_ref().map(|i| i.to_string_id())).serialize(serializer)
}

pub fn deserialize_some_set_id<'de, D: Deserializer<'de>, T: Eq + Hash + Id>(
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

pub fn serialize_some_set_id<S: Serializer, T: Eq + Hash + Id>(
	i: &Option<HashSet<T>>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.as_ref()
		.map(|i| i.iter().map(|i| i.to_string_id()).collect::<HashSet<String>>())
		.serialize(serializer)
}

pub fn deserialize_set_id<'de, D: Deserializer<'de>, T: Eq + Hash + Id>(
	deserializer: D,
) -> Result<HashSet<T>, D::Error> {
	let s: HashSet<String> = Deserialize::deserialize(deserializer)?;
	Ok(s.into_iter()
		.map(|s| T::parse_id(&s))
		.collect::<Result<HashSet<T>>>()
		.map_err(SerdeError::custom)?)
}

pub fn serialize_set_id<S: Serializer, T: Eq + Hash + Id>(
	i: &HashSet<T>, serializer: S,
) -> Result<S::Ok, S::Error> {
	i.iter().map(|i| i.to_string_id()).collect::<HashSet<String>>().serialize(serializer)
}

include!(concat!(env!("OUT_DIR"), "/book_events.rs"));
