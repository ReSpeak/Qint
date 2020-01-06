use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "db", derive(Queryable))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Bookmark {
	pub id: i64,
	pub name: Option<String>,
	pub username: String,
	pub address: String,
	pub bookmark: bool,
	pub last_used: Option<NaiveDateTime>,
	pub timezone: i32,
	pub channel_name: Option<String>,
	pub server_icon: Option<i32>,
}

#[cfg_attr(feature = "db", derive(Queryable))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Message {
	pub id: i64,
	pub invoker: Option<Vec<u8>>,
	pub invoker_name: Option<String>,
	pub content: String,
	pub time: NaiveDateTime,
	pub timezone: i32,

	pub client_name: Option<String>,
}

#[cfg_attr(feature = "db", derive(Queryable))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Chat {
	pub last_read: NaiveDateTime,
	pub timezone: i32,
}
