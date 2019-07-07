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
							use schema::clients::dsl::*;

							let con = con.lock();
							let client = &con.server.clients[id];

							let client_uid = base64::decode(&client.uid.0).unwrap();

							// Check if we already know this client
							if diesel::select(diesel::dsl::exists(clients.filter(
								uid.eq(&client_uid)))).get_result(&self.con)? {
								// Update
								diesel::update(clients.filter(uid.eq(&client_uid)))
									.set(name.eq(&client.name))
									.execute(&self.con)?;
							} else {
								let client = models::ClientInsert {
									uid: &client_uid,
									name: &client.name,
									public_key: None,
									custom_name: None,
								};
								diesel::insert_into(schema::clients::table)
									.values(&client)
									.execute(&self.con)?;
							}
						}
						PropertyId::Channel(ch_id) => {
							use schema::channels::dsl::*;

							let ch_server = con.get_server_key()?;
							let ch_server = ch_server.to_short();
							let con = con.lock();
							let channel = &con.server.channels[ch_id];
							let ch_parent = if channel.parent.0 == 0 {
								None
							} else {
								Some(channel.parent.0 as i64)
							};

							// Check if we already know this channel
							if diesel::select(diesel::dsl::exists(channels
									.filter(server.eq(&ch_server))
									.filter(id.eq(ch_id.0 as i64))
								)).get_result(&self.con)? {
								// Update
								diesel::update(channels
									.filter(server.eq(&ch_server))
									.filter(id.eq(ch_id.0 as i64))
								).set((
									parent.eq(ch_parent),
									name.eq(&channel.name),
									deleted.eq(false),
								)).execute(&self.con)?;
							} else {
								let channel = models::ChannelInsert {
									server: &ch_server,
									id: ch_id.0 as i64,
									parent: ch_parent,
									name: &channel.name,
									icon: None,
									deleted: false,
								};
								diesel::insert_into(schema::channels::table)
									.values(&channel)
									.execute(&self.con)?;
							}
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
