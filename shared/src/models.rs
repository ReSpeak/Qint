use std::cmp::{Ordering, Ord};

use chrono::{DateTime, FixedOffset, NaiveDateTime, TimeZone};
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
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
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

impl Message {
	pub fn get_date_time(&self) -> DateTime<FixedOffset> {
		FixedOffset::east(self.timezone).from_utc_datetime(&self.time)
	}
}

impl Ord for Message {
	fn cmp(&self, other: &Self) -> Ordering {
		self.time.cmp(&other.time).then_with(|| self.id.cmp(&other.id))
	}
}

impl PartialOrd for Message {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}
