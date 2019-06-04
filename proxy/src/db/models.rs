use chrono::{DateTime, Utc};
use diesel_derive_enum::DbEnum;
use failure::Error;

use crate::secret::Secret;
use super::schema::*;

#[derive(Clone, Copy, DbEnum, Debug, Eq, Hash, PartialEq)]
pub enum EventType {
	ChannelSwitched,
	NameChanged,
}

#[derive(Insertable, Queryable)]
pub struct Client {
	pub uid: Vec<u8>,
	pub name: String,
	pub public_key: Option<Vec<u8>>,
	pub custom_name: Option<String>,
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
	pub id: i64,
	pub name: String,
	/// Last used address
	pub address: String,
}

#[derive(Queryable)]
pub struct Channel {
	pub server: i64,
	pub id: i64,
	pub parent: Option<i64>,
	pub name: String,
	pub icon: Option<u32>,
	pub deleted: bool,
}

#[derive(Queryable)]
pub struct Bookmark {
	pub id: i64,
	pub name: Option<String>,
	pub address: String,
	pub channel: Option<i64>,
	pub identity: i64,
	pub bookmark: bool,
	/// Time of last successful connection
	pub last_used: Option<DateTime<Utc>>,
	/// Reference to the server if we already connected once
	pub server: Option<i64>,
}

#[derive(Queryable)]
pub struct Message {
	pub id: i64,
	/// Client uid of sender, `None` if we got the message from the server.
	pub invoker: Option<Vec<u8>>,
	pub content: String,
	pub time: DateTime<Utc>,
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
pub struct NewIdentity {
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


impl Identity {
	pub fn into_identity(self, secret: &Secret) -> Result<tsclientlib::Identity, Error> {
		let key = secret.open(self.private_key)?;
		Ok(tsclientlib::Identity::new_with_max_counter(
			tsproto::crypto::EccKeyPrivP256::import(&key)?,
			self.counter as u64,
			self.max_counter as u64,
		))
	}
}

impl NewIdentity {
	pub fn new(id: tsclientlib::Identity, client_uid: Vec<u8>, secret: &Secret) -> Result<Self, Error> {
		let private_key = secret.seal(id.key().to_short().to_vec())?;
		Ok(Self {
			private_key,
			name: "Default".into(),
			counter: id.counter() as i64,
			max_counter: id.max_counter() as i64,
			client: client_uid,
		})
	}
}
