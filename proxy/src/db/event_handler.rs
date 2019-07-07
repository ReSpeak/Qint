//! Handle incoming events and update the database accordingly.

use diesel::prelude::*;
use failure::Error;
use tsclientlib::events::{Event, PropertyId};

use super::{models, schema};

pub trait EventHandler {
	fn handle_events(&self, con: &tsclientlib::Connection, events: &[Event]) -> Result<(), Error>;
}

impl EventHandler for super::DbHandler {
	fn handle_events(&self, con: &tsclientlib::Connection, events: &[Event]) -> Result<(), Error> {
		for e in events {
			match e {
				Event::PropertyAdded { id, .. } => {
					match id {
						PropertyId::Client(id) => {
							let con = con.lock();
							let client = &con.server.clients[id];

							let uid = base64::decode(&client.uid.0).unwrap();
							let client = models::ClientInsert {
								uid: &uid,
								name: &client.name,
								public_key: None,
								custom_name: None,
							};
							diesel::insert_into(schema::clients::table)
								.values(&client)
								.execute(&self.con)?;
						}
						PropertyId::Channel(id) => {
							let server = con.get_server_key()?;
							let server = server.to_short();
							let con = con.lock();
							let channel = &con.server.channels[id];
							let parent = if channel.parent.0 == 0 {
								None
							} else {
								Some(channel.parent.0 as i64)
							};

							let channel = models::ChannelInsert {
								server: &server,
								id: id.0 as i64,
								parent,
								name: &channel.name,
								icon: None,
								deleted: false,
							};
							diesel::insert_into(schema::channels::table)
								.values(&channel)
								.execute(&self.con)?;
						}
						_ => {}
					}
				}
				Event::PropertyChanged { id, .. } => {}
				Event::PropertyRemoved { id, .. } => {}
				Event::Message { from, invoker, message } => {}
				Event::__NonExhaustive => unreachable!(),
			}
		}
		Ok(())
	}
}
