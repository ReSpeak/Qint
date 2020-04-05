use std::convert::{TryFrom, TryInto};
use std::net::{IpAddr, SocketAddr};

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use tsclientlib::data::*;
use tsclientlib::data::Connection;
use tsclientlib::events::{Event, PropertyId, PropertyValueRef};
use tsclientlib::*;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum JsEvent {
	PropertyAdded {
		id: JsPropertyId,
		prop: JsProperty,
		invoker: Option<Invoker>,
	},
	PropertyChanged {
		id: JsPropertyId,
		prop: JsProperty,
		invoker: Option<Invoker>,
	},
	PropertyRemoved {
		id: JsPropertyId,
		invoker: Option<Invoker>,
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
	Client(ClientId),
	ClientServerGroup(ClientId, ServerGroupId),
	Server,
}

include!(concat!(env!("OUT_DIR"), "/book_events.rs"));
