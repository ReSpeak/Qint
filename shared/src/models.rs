use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[cfg_attr(feature="db", derive(Queryable))]
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
