//! Handle incoming events and update the database accordingly.

use chrono::{Local, Utc};
use chrono::offset::{FixedOffset, TimeZone};
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
							use schema::servers_clients;

							let server = con.get_server_key()?;
							let server = server.to_short();
							let con = con.lock();
							let client = &con.clients[id];

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

							// Update last seen
							let icon = if client.icon_id.0 == 0 {
								None
							} else {
								Some(client.icon_id.0 as i32)
							};
							let avatar = if client.avatar_hash.is_empty() {
								None
							} else {
								Some(client.avatar_hash.as_str())
							};

							let utc_time = Utc::now().naive_utc();
							let dummy_offset = FixedOffset::east(0);
							let local_zone = Local::from_offset(&dummy_offset);
							let utc_to_local_offset = local_zone.offset_from_utc_datetime(&utc_time).local_minus_utc();

							let server_client = models::ServersClientsInsert {
								server: &server,
								client: &client_uid,
								icon,
								avatar,
								last_seen: utc_time,
								timezone: utc_to_local_offset,
							};
							diesel::replace_into(servers_clients::table)
								.values(&server_client)
								.execute(&self.con)?;
						}
						PropertyId::Channel(ch_id) => {
							use schema::channels::dsl::*;

							let ch_server = con.get_server_key()?;
							let ch_server = ch_server.to_short();
							let con = con.lock();
							let channel = &con.channels[ch_id];
							let ch_parent = if channel.parent.0 == 0 {
								None
							} else {
								Some(channel.parent.0 as i64)
							};
							let icon_id = channel.icon_id.and_then(|i| if i.0 == 0 {
								None
							} else {
								Some(i.0 as i32)
							});

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
									icon: icon_id,
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
				Event::PropertyChanged { id: _, .. } => {}
				Event::PropertyRemoved { id: _, .. } => {}
				Event::Message { from: _, invoker: _, message: _ } => {}
				Event::__NonExhaustive => unreachable!(),
			}
		}
		Ok(())
	}
}
