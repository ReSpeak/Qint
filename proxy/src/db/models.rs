use std::cmp::{Ord, Ordering};

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel_derive_enum::DbEnum;
use juniper::GraphQLEnum;
use serde::{Deserialize, Serialize};
use tsproto_types::crypto::EccKeyPrivP256;

use super::{FindIdentity, schema::*};
use crate::secret::Secret;

#[derive(Queryable)]
pub struct Client {
	pub uid: Vec<u8>,
	pub name: String,
	pub public_key: Option<Vec<u8>>,
	pub custom_name: Option<String>,
	pub volume: f32,
	pub custom_phonetic_name: Option<String>,
}

#[derive(Insertable)]
#[table_name = "clients"]
pub struct ClientInsert<'a> {
	pub uid: &'a [u8],
	pub name: &'a str,
	pub public_key: Option<&'a [u8]>,
	pub custom_name: Option<&'a str>,
	pub custom_phonetic_name: Option<&'a str>,
}

#[derive(Clone, Identifiable, Queryable)]
#[table_name = "identities"]
pub struct Identity {
	pub id: i64,
	pub private_key: Vec<u8>,
	pub name: String,
	/// The offset that reaches the highest identity level
	pub counter: i64,
	/// The maximum offset that we computed so far (can reach a lower level than
	/// `counter`).
	pub max_counter: i64,
	/// Client uid
	pub client: Vec<u8>,
}

#[derive(Queryable)]
pub struct Server {
	pub public_key: Vec<u8>,
	pub name: String,
	/// Last used address
	pub address: String,
	pub icon: Option<i32>,
}

#[derive(Insertable)]
#[table_name = "servers"]
pub struct ServerInsert<'a> {
	pub public_key: &'a [u8],
	pub name: &'a str,
	pub address: &'a str,
	pub icon: Option<i32>,
}

#[derive(Queryable)]
pub struct Channel {
	pub server: Vec<u8>,
	pub id: i64,
	pub parent: Option<i64>,
	pub order_id: Option<i64>,
	pub name: String,
	pub icon: Option<i32>,
	pub deleted: bool,
}

#[derive(Insertable)]
#[table_name = "channels"]
pub struct ChannelInsert<'a> {
	pub server: &'a [u8],
	pub id: i64,
	pub parent: Option<i64>,
	pub order_id: Option<i64>,
	pub name: &'a str,
	pub icon: Option<i32>,
	pub deleted: bool,
}

#[derive(Debug, Insertable)]
#[table_name = "bookmarks"]
pub struct BookmarkInsert<'a> {
	pub name: Option<&'a str>,
	pub username: &'a str,
	pub address: &'a str,
	pub channel: Option<i64>,
	pub identity: i64,
	pub bookmark: bool,
	/// Time of last successful connection
	pub last_used: Option<NaiveDateTime>,
	pub timezone: i32,
	pub password: Option<&'a str>,
	pub channel_password: Option<&'a str>,
	/// Reference to the server if we already connected once
	pub server: Option<&'a [u8]>,
}

#[derive(Queryable)]
pub struct Chat {
	pub id: i64,
	pub last_read: NaiveDateTime,
	pub timezone: i32,
}

#[derive(Queryable)]
pub struct Message {
	pub id: i64,
	pub chat: i64,
	/// Client uid of sender, `None` if we got the message from the server.
	pub invoker: Option<Vec<u8>>,
	pub invoker_name: Option<String>,
	pub content: String,
	pub status: MessageStatus,
	pub time: NaiveDateTime,
	pub timezone: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "messages"]
pub struct MessageInsert<'a> {
	pub chat: i64,
	pub invoker: Option<&'a [u8]>,
	pub invoker_name: Option<&'a str>,
	pub content: &'a str,
	pub status: MessageStatus,
	pub time: &'a NaiveDateTime,
	pub timezone: i32,
}

#[derive(Queryable)]
pub struct Event {
	pub id: i64,
	pub server: Option<i64>,
	pub invoker: Option<Vec<u8>>,
	pub channel1: Option<i64>,
	pub channel2: Option<i64>,
	pub client: Option<Vec<u8>>,
	pub typ: EventType,
	pub content: Option<Vec<u8>>,
	pub time: DateTime<Utc>,
}

#[derive(Insertable)]
#[table_name = "identities"]
pub struct NewIdentity<'a> {
	pub private_key: Vec<u8>,
	pub name: &'a str,
	/// The offset that reaches the highest identity level
	pub counter: i64,
	/// The maximum offset that we computed so far (can reach a lower level than
	/// `counter`).
	pub max_counter: i64,
	/// Client uid
	pub client: Vec<u8>,
}

#[derive(AsChangeset, Default)]
#[table_name="identities"]
pub struct UpdateIdentity {
    pub name: Option<String>,
    pub counter: Option<i64>,
	pub max_counter: Option<i64>,
}

#[derive(Debug, Queryable)]
pub struct ServersClients {
	pub server: Vec<u8>,
	pub client: Vec<u8>,
	pub icon: Option<i32>,
	pub avatar: Option<String>,
	pub last_seen: NaiveDateTime,
	pub timezone: i32,
}

#[derive(Debug, Insertable)]
#[table_name = "servers_clients"]
pub struct ServersClientsInsert<'a> {
	pub server: &'a [u8],
	pub client: &'a [u8],
	pub icon: Option<i32>,
	pub avatar: Option<&'a str>,
	pub last_seen: NaiveDateTime,
	pub timezone: i32,
}

#[derive(Clone, Copy, DbEnum, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum EventType {
	ChannelSwitched,
	NameChanged,
}

#[derive(Clone, Copy, DbEnum, Debug, Deserialize, Eq, GraphQLEnum, Hash, PartialEq, Serialize)]
pub enum MessageStatus {
	Sending,
	Success,
	Error,
}

#[derive(Clone, Debug, Deserialize, Queryable, Serialize)]
pub struct Bookmark {
	pub id: i64,
	pub name: Option<String>,
	pub username: String,
	pub address: String,
	pub channel: Option<i64>,
	pub identity: i64,
	pub bookmark: bool,
	pub last_used: Option<NaiveDateTime>,
	pub timezone: i32,
	pub server: Option<Vec<u8>>,
	pub password: Option<String>,
	pub channel_password: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Queryable, Serialize)]
pub struct OldMessage {
	pub id: i64,
	pub invoker: Option<Vec<u8>>,
	pub invoker_name: Option<String>,
	pub content: String,
	pub status: MessageStatus,
	pub time: NaiveDateTime,
	pub timezone: i32,

	pub client_name: Option<String>,
	pub client_icon: Option<i32>,
	pub client_avatar: Option<String>,
}

impl Ord for OldMessage {
	fn cmp(&self, other: &Self) -> Ordering {
		self.time.cmp(&other.time).then_with(|| self.id.cmp(&other.id))
	}
}

impl PartialOrd for OldMessage {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Identity {
	pub fn into_identity(self, secret: &Secret) -> Result<tsclientlib::Identity> {
		let key = secret.open(self.private_key)?;
		Ok(tsclientlib::Identity::new_with_max_counter(
			EccKeyPrivP256::import(&key)?,
			self.counter as u64,
			self.max_counter as u64,
		))
	}
}

impl<'a> NewIdentity<'a> {
	pub fn new_default(identity: &tsclientlib::Identity, secret: &Secret) -> Result<Self> {
		Self::new_with_name(identity, "Default", secret)
	}

	pub fn new_with_name(identity: &tsclientlib::Identity, name: &'a str, secret: &Secret) -> Result<Self> {
		let private_key = secret.seal(identity.key().to_short().to_vec())?;
		let client = identity.key().to_pub().get_uid_no_base64()?;
		Ok(Self {
			private_key,
			name,
			counter: identity.counter() as i64,
			max_counter: identity.max_counter() as i64,
			client,
		})
	}
}

impl UpdateIdentity {
	pub fn from_identity(&mut self, identity: &tsclientlib::Identity) {
		self.counter = Some(identity.counter() as i64);
		self.max_counter = Some(identity.max_counter() as i64);
	}

	pub fn from_identity_with_find(&mut self, identity: &tsclientlib::Identity) -> Result<FindIdentity> {
		self.from_identity(&identity);
		Ok(FindIdentity::ByUid(identity.key().to_pub().get_uid_no_base64()?))
	}
}
