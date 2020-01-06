use chrono::{DateTime, NaiveDateTime, Utc};
use diesel_derive_enum::DbEnum;
use failure::Error;

use super::schema::*;
use crate::secret::Secret;

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
	pub custom_name: Option<String>,
}

#[derive(Insertable)]
#[table_name = "clients"]
pub struct ClientInsert<'a> {
	pub uid: &'a [u8],
	pub name: &'a str,
	pub public_key: Option<&'a [u8]>,
	pub custom_name: Option<&'a str>,
}

#[derive(Identifiable, Queryable)]
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
	pub name: String,
	pub icon: Option<u32>,
	pub deleted: bool,
}

#[derive(Insertable)]
#[table_name = "channels"]
pub struct ChannelInsert<'a> {
	pub server: &'a [u8],
	pub id: i64,
	pub parent: Option<i64>,
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
	/// Client uid of sender, `None` if we got the message from the server.
	pub invoker: Option<Vec<u8>>,
	pub content: String,
	pub time: DateTime<Utc>,
}

#[derive(Debug, Insertable)]
#[table_name = "messages"]
pub struct MessageInsert<'a> {
	pub chat: i64,
	pub invoker: Option<&'a [u8]>,
	pub invoker_name: Option<&'a str>,
	pub content: &'a str,
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
	pub client: &'a [u8],
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

impl Identity {
	pub fn into_identity(
		self,
		secret: &Secret,
	) -> Result<tsclientlib::Identity, Error>
	{
		let key = secret.open(self.private_key)?;
		Ok(tsclientlib::Identity::new_with_max_counter(
			tsproto::crypto::EccKeyPrivP256::import(&key)?,
			self.counter as u64,
			self.max_counter as u64,
		))
	}
}

impl<'a> NewIdentity<'a> {
	pub fn new(
		id: &tsclientlib::Identity,
		client_uid: &'a [u8],
		secret: &Secret,
	) -> Result<Self, Error>
	{
		let private_key = secret.seal(id.key().to_short().to_vec())?;
		Ok(Self {
			private_key,
			name: "Default",
			counter: id.counter() as i64,
			max_counter: id.max_counter() as i64,
			client: client_uid,
		})
	}
}
