use chrono::{DateTime, Utc};
use diesel_derive_enum::DbEnum;

use super::schema::*;

#[derive(Clone, Copy, DbEnum, Debug, Eq, Hash, PartialEq)]
pub enum EventType {
	ChannelSwitched,
	NameChanged,
}

#[derive(Queryable)]
pub struct Client {
	pub uid: Vec<u8>,
	pub name: String,
	pub public_key: Option<Vec<u8>>,
	pub icon: Option<u32>,
	pub custom_name: Option<String>,
}

#[derive(Identifiable, Queryable)]
#[table_name="identities"]
pub struct Identity {
	pub id: u64,
	pub private_key: Vec<u8>,
	pub name: String,
	/// The offset that reaches the highest identity level
	pub offset: u64,
	/// The maximum offset that we computed so far (can reach a lower level than
	/// `offset`).
	pub max_counter: u64,
	/// Client uid
	pub client: Vec<u8>,
}

#[derive(Queryable)]
pub struct Server {
	pub id: u64,
	pub name: String,
	/// Last used address
	pub address: String,
}

#[derive(Queryable)]
pub struct Channel {
	pub server: u64,
	pub id: u64,
	pub parent: Option<u64>,
	pub name: String,
	pub icon: Option<u32>,
	pub deleted: bool,
}

#[derive(Queryable)]
pub struct Bookmark {
	pub id: u64,
	pub name: Option<String>,
	pub address: String,
	pub channel: Option<u64>,
	pub identity: u64,
	pub bookmark: bool,
	/// Time of last successful connection
	pub last_used: Option<DateTime<Utc>>,
	/// Reference to the server if we already connected once
	pub server: Option<u64>,
}

#[derive(Queryable)]
pub struct Message {
	pub id: u64,
	/// Client uid of sender, `None` if we got the message from the server.
	pub invoker: Option<Vec<u8>>,
	pub content: String,
	pub time: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct Event {
	pub id: u64,
	pub server: Option<u64>,
	pub invoker: Option<Vec<u8>>,
	pub channel1: Option<u64>,
	pub channel2: Option<u64>,
	pub client: Option<Vec<u8>>,
	pub typ: EventType,
	pub content: Option<Vec<u8>>,
	pub time: DateTime<Utc>,
}
