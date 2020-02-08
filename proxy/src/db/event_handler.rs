//! Handle incoming events and update the database accordingly.

use chrono::offset::{FixedOffset, TimeZone};
use chrono::{Duration, Local, Utc};
use diesel::prelude::*;
use failure::{bail, Error};
use qint_shared::{ChatId, ChatType};
use qint_shared::models::MessageStatus;
use tsclientlib::events::{Event, PropertyId, PropertyValue};
use tsclientlib::MessageTarget;

use super::{models, schema};

pub trait EventHandler {
	fn handle_events(
		&mut self,
		con: &tsclientlib::Connection,
		events: &[Event],
	) -> Result<(), Error>;
}

impl EventHandler for super::DbHandler {
	fn handle_events(
		&mut self,
		con: &tsclientlib::Connection,
		events: &[Event],
	) -> Result<(), Error>
	{
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

							let client_uid =
								base64::decode(&client.uid.0).unwrap();

							// Check if we already know this client
							if diesel::select(diesel::dsl::exists(
								clients.filter(uid.eq(&client_uid)),
							))
							.get_result(&self.con)?
							{
								// Update
								diesel::update(
									clients.filter(uid.eq(&client_uid)),
								)
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
							let utc_to_local_offset = local_zone
								.offset_from_utc_datetime(&utc_time)
								.local_minus_utc();

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
							let icon_id = channel.icon_id.and_then(|i| {
								if i.0 == 0 { None } else { Some(i.0 as i32) }
							});

							// Check if we already know this channel
							if diesel::select(diesel::dsl::exists(
								channels
									.filter(server.eq(&ch_server))
									.filter(id.eq(ch_id.0 as i64)),
							))
							.get_result(&self.con)?
							{
								// Update
								diesel::update(
									channels
										.filter(server.eq(&ch_server))
										.filter(id.eq(ch_id.0 as i64)),
								)
								.set((
									parent.eq(ch_parent),
									name.eq(&channel.name),
									deleted.eq(false),
								))
								.execute(&self.con)?;
							} else {
								let channel = models::ChannelInsert {
									server: &ch_server,
									id: ch_id.0 as i64,
									parent: ch_parent,
									name: &channel.name,
									icon: icon_id,
									deleted: false,
								};
								diesel::replace_into(schema::channels::table)
									.values(&channel)
									.execute(&self.con)?;
							}
						}
						_ => {}
					}
				}
				Event::PropertyChanged { .. } => {}
				Event::PropertyRemoved { id, old, .. } => {
					match id {
						PropertyId::Client(_) => {
							use schema::servers_clients;

							let server = con.get_server_key()?;
							let server = server.to_short();
							let client = match old {
								PropertyValue::Client(client) => client,
								_ => panic!(
									"Property value should be a client but \
									 wasn't"
								),
							};

							let client_uid =
								base64::decode(&client.uid.0).unwrap();

							// Update last seen
							let utc_time = Utc::now().naive_utc();
							let dummy_offset = FixedOffset::east(0);
							let local_zone = Local::from_offset(&dummy_offset);
							let utc_to_local_offset = local_zone
								.offset_from_utc_datetime(&utc_time)
								.local_minus_utc();

							diesel::update(servers_clients::table.filter(
								servers_clients::server.eq(server).and(
									servers_clients::client.eq(&client_uid),
								),
							))
							.set((
								servers_clients::last_seen.eq(utc_time),
								servers_clients::timezone
									.eq(utc_to_local_offset),
							))
							.execute(&self.con)?;
						}
						PropertyId::Channel(ch_id) => {
							use schema::channels::dsl::*;

							let ch_server = con.get_server_key()?;
							let ch_server = ch_server.to_short();

							// Mark channel as deleted
							diesel::update(
								channels.filter(
									server
										.eq(&ch_server)
										.and(id.eq(ch_id.0 as i64)),
								),
							)
							.set(deleted.eq(true))
							.execute(&self.con)?;
						}
						_ => {}
					}
				}
				Event::Message { from, invoker, message } => {
					use schema::messages;

					let server = con.get_server_key()?;
					let server = server.to_short();

					let utc_time = Utc::now().naive_utc();
					let dummy_offset = FixedOffset::east(0);
					let local_zone = Local::from_offset(&dummy_offset);
					let utc_to_local_offset = local_zone
						.offset_from_utc_datetime(&utc_time)
						.local_minus_utc();

					let invoker_uid = if let Some(uid) = &invoker.uid {
						Some(base64::decode(&uid.0).unwrap())
					} else {
						None
					};
					let invoker_name = if invoker_uid.is_none() {
						Some(invoker.name.as_str())
					} else {
						None
					};

					let own_message = invoker.id == con.lock().own_client;

					// Make sure the chat exists
					let chat;
					// If it is possible that the message is inserted by
					// multiple clients which are connected to the same server.
					// We have to make sure that only one instance of a message
					// is inserted into the database.
					let can_be_duplicate;
					match from {
						MessageTarget::Server => {
							can_be_duplicate = true;
							chat = ChatType::Server;
						}
						MessageTarget::Channel => {
							can_be_duplicate = true;
							let con = con.lock();
							let client = con.clients.get(&con.own_client);
							let own_client = if let Some(client) = client {
								client
							} else {
								bail!("Failed to find own client");
							};
							chat = ChatType::Channel(own_client.channel.0);
						}
						MessageTarget::Client(id) => {
							can_be_duplicate = false;
							let con = con.lock();
							let client = con.clients.get(id);
							let client_uid = if let Some(client) = client {
								Some(&client.uid.0)
							} else if !own_message {
								invoker.uid.as_ref().map(|uid| &uid.0)
							} else {
								None
							};

							let client_uid = if let Some(uid) = client_uid {
								uid
							} else {
								bail!("Failed to find client");
							};
							chat = ChatType::Client(base64::decode(client_uid).unwrap());
						}
						MessageTarget::Poke(id) => {
							can_be_duplicate = false;
							let con = con.lock();
							let client = &con.clients[id];
							chat = ChatType::Client(base64::decode(&client.uid.0).unwrap());
						}
					}
					let chat = self.get_or_create_chat(&ChatId {
						server: server.to_vec(),
						chat_type: chat,
					})?;

					self.last_message_id = self
						.con
						.transaction::<_, diesel::result::Error, _>(|| {
							if can_be_duplicate {
								// Check if the message is already in the database
								let start_check_time =
									utc_time - Duration::seconds(1);
								let cmp = messages::chat
									.eq(chat)
									.and(messages::invoker.eq(&invoker_uid))
									.and(
										messages::invoker_name.eq(invoker_name),
									)
									.and(messages::content.eq(message))
									.and(messages::time.gt(&start_check_time))
									.and(messages::id.gt(self.last_message_id));
								let id = messages::table
									.filter(cmp)
									.select(messages::id)
									.first::<i64>(&self.con)
									.optional()?;

								if let Some(id) = id {
									return Ok(id);
								}
							}

							if own_message {
								// Check if the message is already in the database
								let cmp = messages::chat
									.eq(chat)
									.and(messages::invoker.eq(&invoker_uid))
									.and(messages::invoker_name.eq(invoker_name))
									.and(messages::content.eq(message))
									.and(messages::status.eq(MessageStatus::Sending));
								// Update status
								let res = diesel::update(messages::table.filter(cmp))
									.set(messages::status.eq(MessageStatus::Success))
									.execute(&self.con)?;

								if res != 0 {
									// Successfully updated
									return messages::table
										.order(messages::id.desc())
										.select(messages::id)
										.first::<i64>(&self.con);
								}
							}

							// Insert message
							let message = models::MessageInsert {
								chat,
								invoker: invoker_uid
									.as_ref()
									.map(|v| v.as_slice()),
								invoker_name,
								content: message,
								status: MessageStatus::Success,
								time: &utc_time,
								timezone: utc_to_local_offset,
							};
							diesel::insert_into(messages::table)
								.values(&message)
								.execute(&self.con)?;
							messages::table
								.order(messages::id.desc())
								.select(messages::id)
								.first::<i64>(&self.con)
						})?;
				}
				Event::__NonExhaustive => unreachable!(),
			}
		}
		Ok(())
	}
}
