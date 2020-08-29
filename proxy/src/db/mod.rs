use std::fs;
use std::result;

use actix::*;
use anyhow::{bail, Result};
use chrono::offset::{FixedOffset, TimeZone};
use chrono::{Duration, Local, NaiveDateTime, Utc};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::prelude::*;
use slog::{error, info, warn, Logger};
use tsclientlib::data::Client;
use tsclientlib::data::Connection as TsData;
use tsclientlib::events::{Event, PropertyId, PropertyValue};
use tsclientlib::Connection as TsConnection;
use tsclientlib::{ChannelId, ClientId, Identity, Invoker, MessageTarget};
use tsproto_types::crypto::EccKeyPubP256;

use crate::secret::Secret;
use crate::Settings;
use models::MessageStatus;

pub(crate) mod graphql;
mod models;
mod schema;

type DieselResult<T> = std::result::Result<T, diesel::result::Error>;

diesel_migrations::embed_migrations!();

pub struct DbHandler {
	secret: Secret,
	con: SqliteConnection,
	last_message_id: i64,
}

struct EventHandler<'a> {
	db: &'a Addr<DbHandler>,
	logger: &'a Logger,
	con: &'a TsConnection,
	data: &'a TsData,
}

/// Identity id, `true` will create a new identity if this id does not exist.
#[derive(Clone, Debug)]
pub struct GetIdentityMsg(pub u64, pub bool);
#[derive(Clone, Debug)]
pub struct GetClientVolumeMsg(pub Vec<u8>);
pub struct UpdateIdentityMsg(pub Identity);
struct RunOnDbMsg<I: 'static, E: 'static, F: FnOnce(&mut DbHandler) -> result::Result<I, E>>(F);

pub struct ConnectedMsg {
	pub bookmark: Option<i64>,
	pub username: String,
	pub address: String,
	pub channel: Option<i64>,
	pub identity: i64,
	pub server_key: EccKeyPubP256,
}

pub struct WriteMessageMsg {
	pub message: String,
	pub invoker_uid: Vec<u8>,
	pub chat: ChatId,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ChatId {
	/// The public key of the server.
	pub server: Vec<u8>,
	pub chat_type: ChatType,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ChatType {
	Server,
	Channel(u64),
	/// A chat with a client, identified by the client uid.
	Client(Vec<u8>),
	/// Pokes with a client, identified by the client uid.
	Poke(Vec<u8>),
}

type RunFn =
	Box<dyn FnOnce(&mut DbHandler, &mut <DbHandler as Actor>::Context) -> Result<()> + Send>;

pub struct RunMsg(RunFn);

impl Actor for DbHandler {
	type Context = Context<Self>;
}

impl Message for GetIdentityMsg {
	type Result = Result<Identity>;
}
impl Message for GetClientVolumeMsg {
	type Result = Result<Option<f32>>;
}
impl<I: 'static, E: 'static, F: FnOnce(&mut DbHandler) -> result::Result<I, E>> Message
	for RunOnDbMsg<I, E, F>
{
	type Result = result::Result<I, E>;
}
impl Message for UpdateIdentityMsg {
	type Result = Result<()>;
}
impl Message for WriteMessageMsg {
	type Result = Result<()>;
}
impl Message for ConnectedMsg {
	type Result = Result<()>;
}
impl Message for RunMsg {
	type Result = Result<()>;
}

impl DbHandler {
	pub(crate) fn new(logger: Logger, settings: &Settings, secret: Secret) -> Result<Self> {
		let database_url = settings.config_path.join("storage.sqlite");
		let con = SqliteConnection::establish(database_url.to_str().unwrap())?;

		// The database can be opened successfully, create backup
		fs::copy(database_url, settings.config_path.join("storage.sqlite.bak"))?;

		// Enable foreign keys
		con.batch_execute("PRAGMA foreign_keys = ON")?;

		// Run migrations
		let mut s = Vec::new();
		embedded_migrations::run_with_output(&con, &mut s)?;
		let s = std::str::from_utf8(&s)?;
		if !s.is_empty() {
			info!(logger, "Run database migrations"; "output" => s);
		}

		Ok(Self { secret, con, last_message_id: 0 })
	}

	/// Create a new chat entry in the database and returns the id.
	/// This has to be executed inside a transaction.
	fn create_chat(&self) -> DieselResult<i64> {
		use schema::chats;

		// Make sure it does not count as read
		let utc_time = Utc::now().naive_utc() - Duration::days(1);
		let dummy_offset = FixedOffset::east(0);
		let local_zone = Local::from_offset(&dummy_offset);
		let utc_to_local_offset = local_zone.offset_from_utc_datetime(&utc_time).local_minus_utc();

		diesel::insert_into(chats::table)
			.values(&(chats::last_read.eq(&utc_time), chats::timezone.eq(utc_to_local_offset)))
			.execute(&self.con)?;
		chats::table.order(chats::id.desc()).select(chats::id).first::<i64>(&self.con)
	}

	fn get_or_create_chat(&self, id: &ChatId) -> DieselResult<i64> {
		use schema::{channel_chats, client_chats, client_pokes, server_chats};

		match &id.chat_type {
			ChatType::Server => {
				self.con.transaction::<_, diesel::result::Error, _>(|| {
					if let Some(chat) = server_chats::table
						.find(&id.server)
						.select(server_chats::chat)
						.first::<i64>(&self.con)
						.optional()?
					{
						Ok(chat)
					} else {
						// Create new chat
						let chat = self.create_chat()?;

						diesel::insert_into(server_chats::table)
							.values(&(
								server_chats::server.eq(&id.server),
								server_chats::chat.eq(chat),
							))
							.execute(&self.con)?;
						Ok(chat)
					}
				})
			}
			ChatType::Channel(channel) => {
				self.con.transaction::<_, diesel::result::Error, _>(|| {
					if let Some(chat) = channel_chats::table
						.find((&id.server, *channel as i64))
						.select(channel_chats::chat)
						.first::<i64>(&self.con)
						.optional()?
					{
						Ok(chat)
					} else {
						// Create new chat
						let chat = self.create_chat()?;

						diesel::insert_into(channel_chats::table)
							.values(&(
								channel_chats::server.eq(&id.server),
								channel_chats::channel.eq(*channel as i64),
								channel_chats::chat.eq(chat),
							))
							.execute(&self.con)?;
						Ok(chat)
					}
				})
			}
			ChatType::Client(client) => {
				self.con.transaction::<_, diesel::result::Error, _>(|| {
					if let Some(chat) = client_chats::table
						.find((&id.server, &client))
						.select(client_chats::chat)
						.first::<i64>(&self.con)
						.optional()?
					{
						Ok(chat)
					} else {
						// Create new chat
						let chat = self.create_chat()?;

						diesel::insert_into(client_chats::table)
							.values(&(
								client_chats::server.eq(&id.server),
								client_chats::client.eq(&client),
								client_chats::chat.eq(chat),
							))
							.execute(&self.con)?;
						Ok(chat)
					}
				})
			}
			ChatType::Poke(client) => {
				self.con.transaction::<_, diesel::result::Error, _>(|| {
					if let Some(chat) = client_pokes::table
						.find((&id.server, &client))
						.select(client_pokes::chat)
						.first::<i64>(&self.con)
						.optional()?
					{
						Ok(chat)
					} else {
						// Create new chat
						let chat = self.create_chat()?;

						diesel::insert_into(client_pokes::table)
							.values(&(
								client_pokes::server.eq(&id.server),
								client_pokes::client.eq(&client),
								client_pokes::chat.eq(chat),
							))
							.execute(&self.con)?;
						Ok(chat)
					}
				})
			}
		}
	}
}

impl Handler<GetIdentityMsg> for DbHandler {
	type Result = Result<Identity>;
	fn handle(&mut self, msg: GetIdentityMsg, _: &mut Self::Context) -> Self::Result {
		use schema::identities::dsl::*;

		match identities.find(msg.0 as i64).first::<models::Identity>(&self.con) {
			Ok(r) => r.into_identity(&self.secret),
			Err(_) => {
				// Pick an existing identity if one exists
				if let Ok(r) = identities.order(id).first::<models::Identity>(&self.con) {
					return r.into_identity(&self.secret);
				}

				// Create new identity
				let identity = tsclientlib::Identity::create()?;
				let pub_key = identity.key().to_pub();
				let uid = pub_key.get_uid_no_base64()?;

				let cli = models::ClientInsert {
					uid: &uid,
					name: "TeamSpeakUser",
					public_key: Some(pub_key.to_short()),
					custom_name: None,
				};
				diesel::insert_into(schema::clients::table).values(&cli).execute(&self.con)?;

				let new_identity = models::NewIdentity::new(&identity, &uid, &self.secret)?;
				diesel::insert_into(identities).values(&new_identity).execute(&self.con)?;

				Ok(identity)
			}
		}
	}
}

impl Handler<GetClientVolumeMsg> for DbHandler {
	type Result = Result<Option<f32>>;
	fn handle(&mut self, msg: GetClientVolumeMsg, _: &mut Self::Context) -> Self::Result {
		use schema::clients::dsl::*;
		Ok(clients.find(&msg.0).select(volume).first::<f32>(&self.con).optional()?)
	}
}

impl<I: 'static, E: 'static, F: FnOnce(&mut DbHandler) -> result::Result<I, E>>
	Handler<RunOnDbMsg<I, E, F>> for DbHandler
{
	type Result = result::Result<I, E>;
	fn handle(&mut self, msg: RunOnDbMsg<I, E, F>, _: &mut Self::Context) -> Self::Result {
		msg.0(self)
	}
}

impl Handler<UpdateIdentityMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(
		&mut self, UpdateIdentityMsg(identity): UpdateIdentityMsg, _: &mut Self::Context,
	) -> Self::Result {
		use schema::identities::dsl::*;

		let pub_key = identity.key().to_pub();
		let uid = pub_key.get_uid_no_base64()?;
		diesel::update(identities.filter(client.eq(uid)))
			.set((
				counter.eq(identity.counter() as i64),
				max_counter.eq(identity.max_counter() as i64),
			))
			.execute(&self.con)?;
		Ok(())
	}
}

impl Handler<WriteMessageMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(&mut self, message: WriteMessageMsg, _: &mut Self::Context) -> Self::Result {
		use schema::messages;

		let (utc_time, utc_to_local_offset) = EventHandler::get_now();

		let chat = self.get_or_create_chat(&message.chat)?;
		self.last_message_id = self.con.transaction::<_, diesel::result::Error, _>(|| {
			// Insert message
			let message = models::MessageInsert {
				chat,
				invoker: Some(&message.invoker_uid),
				invoker_name: None,
				content: &message.message,
				status: MessageStatus::Sending,
				time: &utc_time,
				timezone: utc_to_local_offset,
			};
			diesel::insert_into(messages::table).values(&message).execute(&self.con)?;
			messages::table.order(messages::id.desc()).select(messages::id).first::<i64>(&self.con)
		})?;

		Ok(())
	}
}

impl Handler<ConnectedMsg> for DbHandler {
	type Result = Result<()>;
	/// Has to be called after the server was added in handle_connected.
	fn handle(&mut self, msg: ConnectedMsg, _: &mut Self::Context) -> Self::Result {
		use diesel::dsl::not;
		use schema::{bookmarks, identities};
		let server = msg.server_key.to_short();

		// Find identity
		let identity = match identities::table
			.find(msg.identity as i64)
			.select(identities::id)
			.first::<i64>(&self.con)
		{
			Ok(r) => r,
			Err(_) => {
				// Pick an existing identity
				identities::table
					.order(identities::id)
					.select(identities::id)
					.first::<i64>(&self.con)?
			}
		};

		// Compare channel: bookmarks::channel == msg.channel
		// But with null == null
		//
		// https://stackoverflow.com/questions/10416789/how-to-rewrite-is-distinct-from-and-is-not-distinct-from
		// a IS NOT DISTINCT FROM b can be rewritten as:
		// (NOT (a <> b OR a IS NULL OR b IS NULL) OR (a IS NULL AND b IS NULL))
		let cmp = not(bookmarks::channel
			.ne(msg.channel)
			.or(bookmarks::channel.is_null())
			.or(msg.channel.is_none()))
		.or(bookmarks::channel.is_null().and(msg.channel.is_none()));

		// Check if we already know that address
		let id = msg
			.bookmark
			.map(Ok)
			.or_else(|| {
				bookmarks::table
					.filter(
						cmp.and(bookmarks::address.eq(&msg.address))
							.and(bookmarks::username.eq(&msg.username))
							.and(bookmarks::identity.eq(identity)),
					)
					.select(bookmarks::id)
					.first::<i64>(&self.con)
					.optional()
					.transpose()
			})
			.transpose()?;

		let (utc_time, utc_to_local_offset) = EventHandler::get_now();
		if let Some(id) = id {
			// Update
			diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
				.set((
					bookmarks::username.eq(&msg.username),
					bookmarks::server.eq(&server),
					bookmarks::last_used.eq(Some(utc_time)),
					bookmarks::timezone.eq(utc_to_local_offset),
				))
				.execute(&self.con)?;
		} else {
			let bookmark = models::BookmarkInsert {
				name: None,
				username: &msg.username,
				address: &msg.address,
				channel: msg.channel,
				identity,
				bookmark: false,
				last_used: Some(utc_time),
				timezone: utc_to_local_offset,
				server: Some(&server),
			};
			diesel::insert_into(bookmarks::table).values(&bookmark).execute(&self.con)?;
		}
		Ok(())
	}
}

impl Handler<RunMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(&mut self, RunMsg(f): RunMsg, ctx: &mut Self::Context) -> Self::Result {
		f(self, ctx)
	}
}

impl DbHandler {
	pub fn handle_events(
		db: &Addr<Self>, logger: &Logger, con: &TsConnection, data: &TsData, events: &[Event],
		connected_msg: Option<ConnectedMsg>,
	) -> Result<()>
	{
		let handler = EventHandler::new(db, logger, con, data);

		for e in events {
			let r = match e {
				Event::PropertyAdded { id, .. } => {
					match id {
						PropertyId::Server => handler.handle_connected(),
						PropertyId::Client(id) => {
							if let Some(client) = data.clients.get(id) {
								Self::register_client(logger, &handler, e, data, Some(client));
								// Update servers_clients if we know this client
								handler.handle_add_client(client, false)
							} else {
								Ok(())
							}
						}
						PropertyId::Channel(id) => handler.handle_add_channel(*id),
						_ => Ok(()),
					}
				}
				Event::PropertyChanged { id, .. } => {
					match id {
						PropertyId::ServerName => handler.handle_server_name(),
						PropertyId::ServerIconId => handler.handle_server_icon(),

						PropertyId::ClientAvatarHash(id) => {
							Self::register_client(logger, &handler, e, data, data.clients.get(id));
							handler.handle_client_avatar(*id)
						}
						PropertyId::ClientIconId(id) => {
							Self::register_client(logger, &handler, e, data, data.clients.get(id));
							handler.handle_client_icon(*id)
						}
						PropertyId::ClientName(id) => handler.handle_client_name(*id),
						// TODO register_client for other changes
						PropertyId::ClientChannel(id) => {
							Self::register_client(logger, &handler, e, data, data.clients.get(id));
							Ok(())
						}

						PropertyId::ChannelParent(id) => handler.handle_channel_parent(*id),
						PropertyId::ChannelOrder(id) => handler.handle_channel_order(*id),
						PropertyId::ChannelName(id) => handler.handle_channel_name(*id),
						PropertyId::ChannelIconId(id) => handler.handle_channel_icon(*id),
						_ => Ok(()),
					}
				}
				Event::PropertyRemoved { id, old, .. } => match id {
					PropertyId::Client(_) => {
						let client = match old {
							PropertyValue::Client(client) => client,
							_ => panic!("Property value should be a client but wasn't"),
						};
						Self::register_client(logger, &handler, e, data, Some(client));
						handler.handle_remove_client(client)
					}
					PropertyId::Channel(id) => handler.handle_remove_channel(*id),
					_ => Ok(()),
				},
				Event::Message { target, invoker, message } => {
					if let MessageTarget::Client(id) = target {
						if id != &data.own_client {
							Self::register_client(logger, &handler, e, data, data.clients.get(id));
						} else {
							Self::register_client(
								logger,
								&handler,
								e,
								data,
								data.clients.get(&data.own_client),
							);
						}
					} else {
						Self::register_client(
							logger,
							&handler,
							e,
							data,
							data.clients.get(&data.own_client),
						);
					}
					handler.handle_message(*target, invoker, message)
				}
				Event::ChannelListFinished => handler.handle_channellistfinished(),
			};

			if let Err(e) = r {
				error!(logger, "Failed to handle event for database"; "error" => %e);
			}
		}

		if let Some(msg) = connected_msg {
			let logger = logger.clone();
			actix::spawn(db.send(msg).map(move |r| match r {
				Err(e) => warn!(logger, "Failed to save connection in database"; "error" => %e),
				Ok(Err(e)) => warn!(logger, "Failed to save connection in database"; "error" => %e),
				_ => {}
			}));
		}

		Ok(())
	}

	pub fn create_client(
		db: &Addr<Self>, logger: &Logger, con: &TsConnection, data: &TsData, client: &Client,
	) -> Result<()> {
		let handler = EventHandler::new(db, logger, con, data);
		handler.handle_add_client(client, true)
	}

	/// Only add clients to database when we interact with them:
	/// We receive or write a message to them, we are modified with them
	/// as invoker or they are modified with us as invoker.
	fn register_client(
		logger: &Logger, handler: &EventHandler, e: &Event, data: &TsData, client: Option<&Client>,
	) {
		if let Some(c) = client {
			if let Some(i) = e.get_invoker() {
				if c.id != i.id {
					let r = if c.id == data.own_client {
						handler.handle_add_invoker(i)
					} else if i.id == data.own_client {
						handler.handle_add_client(c, true)
					} else {
						Ok(())
					};

					if let Err(e) = r {
						error!(logger, "Failed to handle event for database"; "error" => %e);
					}
				}
			}
		}
	}
}

impl<'a> EventHandler<'a> {
	fn new(
		db: &'a Addr<DbHandler>, logger: &'a Logger, con: &'a TsConnection, data: &'a TsData,
	) -> Self {
		Self { db, logger, con, data }
	}

	fn run<
		F: FnOnce(&mut DbHandler, &mut <DbHandler as Actor>::Context) -> Result<()> + Send + 'static,
	>(
		&self, f: F,
	) {
		let logger = self.logger.clone();
		actix::spawn(self.db.send(RunMsg(Box::new(f))).map(move |r| match r {
			Err(e) => {
				error!(logger, "Failed to send to database"; "error" => %e);
			}
			Ok(Err(e)) => {
				error!(logger, "Failed to write to database"; "error" => %e);
			}
			_ => {}
		}))
	}

	fn get_server_key(&self) -> Result<Vec<u8>> {
		let key = self.con.get_server_key()?;
		Ok(key.to_short().to_vec())
	}

	/// Returns the current time in utc and the offset.
	fn get_now() -> (NaiveDateTime, i32) {
		let utc_time = Utc::now().naive_utc();
		let dummy_offset = FixedOffset::east(0);
		let local_zone = Local::from_offset(&dummy_offset);
		let utc_to_local_offset = local_zone.offset_from_utc_datetime(&utc_time).local_minus_utc();
		(utc_time, utc_to_local_offset)
	}

	fn handle_connected(&self) -> Result<()> {
		let key = self.get_server_key()?;
		let icon_id = if self.data.server.icon_id.0 != 0 {
			Some(self.data.server.icon_id.0 as i32)
		} else {
			None
		};
		let server_name = self.data.server.name.clone();
		let addr = self.con.get_options().get_address().to_string();

		self.run(move |db, _| {
			use schema::servers::dsl::*;

			// Check if we already know that server
			if diesel::select(diesel::dsl::exists(servers.filter(public_key.eq(&key))))
				.get_result(&db.con)?
			{
				// Update
				diesel::update(servers.filter(public_key.eq(&key)))
					.set((name.eq(&server_name), address.eq(&addr), icon.eq(&icon_id)))
					.execute(&db.con)?;
			} else {
				let server = models::ServerInsert {
					public_key: &key,
					name: &server_name,
					address: &addr,
					icon: icon_id,
				};
				diesel::insert_into(schema::servers::table).values(&server).execute(&db.con)?;
			}
			Ok(())
		});
		Ok(())
	}

	fn handle_server_name(&self) -> Result<()> {
		let key = self.get_server_key()?;
		let server_name = self.data.server.name.clone();
		self.run(move |db, _| {
			use schema::servers::dsl::*;
			diesel::update(servers.filter(public_key.eq(&key)))
				.set(name.eq(&server_name))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_server_icon(&self) -> Result<()> {
		let key = self.get_server_key()?;
		let icon_id = if self.data.server.icon_id.0 != 0 {
			Some(self.data.server.icon_id.0 as i32)
		} else {
			None
		};
		self.run(move |db, _| {
			use schema::servers::dsl::*;
			diesel::update(servers.filter(public_key.eq(&key)))
				.set(icon.eq(&icon_id))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_add_invoker(&self, invoker: &Invoker) -> Result<()> {
		match self.data.clients.get(&invoker.id) {
			Some(client) => self.handle_add_client(client, true),
			None => {
				if let Some(uid) = &invoker.uid {
					self.handle_add_client_internal(
						true,
						invoker.name.clone(),
						uid.0.clone(),
						None,
						None,
					)
				} else {
					Ok(())
				}
			}
		}
	}

	pub fn handle_add_client(&self, client: &Client, create: bool) -> Result<()> {
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		let icon = if client.icon_id.0 == 0 { None } else { Some(client.icon_id.0 as i32) };
		let avatar =
			if client.avatar_hash.is_empty() { None } else { Some(client.avatar_hash.clone()) };
		let client_name = client.name.clone();
		self.handle_add_client_internal(create, client_name, client_uid, icon, avatar)
	}

	fn handle_add_client_internal(
		&self, create: bool, client_name: String, client_uid: Vec<u8>, icon: Option<i32>,
		avatar: Option<String>,
	) -> Result<()>
	{
		let server = self.get_server_key()?;
		self.run(move |db, _| {
			use schema::clients::dsl::*;
			use schema::servers_clients;

			// Check if we already know this client
			if diesel::select(diesel::dsl::exists(clients.filter(uid.eq(&client_uid))))
				.get_result(&db.con)?
			{
				// Update
				diesel::update(clients.filter(uid.eq(&client_uid)))
					.set(name.eq(&client_name))
					.execute(&db.con)?;
			} else {
				if !create {
					return Ok(());
				}

				let client = models::ClientInsert {
					uid: &client_uid,
					name: &client_name,
					public_key: None,
					custom_name: None,
				};
				diesel::insert_into(schema::clients::table).values(&client).execute(&db.con)?;
			}

			let (utc_time, utc_to_local_offset) = Self::get_now();
			let server_client = models::ServersClientsInsert {
				server: &server,
				client: &client_uid,
				icon,
				avatar: avatar.as_deref(),
				last_seen: utc_time,
				timezone: utc_to_local_offset,
			};
			diesel::replace_into(servers_clients::table).values(&server_client).execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_remove_client(&self, client: &Client) -> Result<()> {
		// If own client removed, handle for all other clients
		if client.id == self.data.own_client {
			self.handle_disconnect()?;
		}

		let server = self.get_server_key()?;
		let client_uid = match &client.uid {
			Some(uid) => uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		self.run(move |db, _| {
			use schema::servers_clients;

			// Update last seen
			let (utc_time, utc_to_local_offset) = Self::get_now();

			diesel::update(servers_clients::table.filter(
				servers_clients::server.eq(server).and(servers_clients::client.eq(&client_uid)),
			))
			.set((
				servers_clients::last_seen.eq(utc_time),
				servers_clients::timezone.eq(utc_to_local_offset),
			))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	/// Update last seen for all clients.
	fn handle_disconnect(&self) -> Result<()> {
		let server = self.get_server_key()?;
		let uids = self
			.data
			.clients
			.values()
			.filter_map(|c| c.uid.as_ref().map(|u| u.0.clone()))
			.collect::<Vec<_>>();

		self.run(move |db, _| {
			use schema::servers_clients;

			// Update last seen
			let (utc_time, utc_to_local_offset) = Self::get_now();

			diesel::update(servers_clients::table.filter(
				servers_clients::server.eq(server).and(servers_clients::client.eq_any(&uids)),
			))
			.set((
				servers_clients::last_seen.eq(utc_time),
				servers_clients::timezone.eq(utc_to_local_offset),
			))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_client_avatar(&self, id: ClientId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let client = if let Some(r) = self.data.clients.get(&id) {
			r
		} else {
			bail!("Client not found");
		};
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		let client_avatar =
			if client.avatar_hash.is_empty() { None } else { Some(client.avatar_hash.clone()) };
		self.run(move |db, _| {
			use schema::servers_clients::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				servers_clients.filter(server.eq(&server_id).and(client.eq(&client_uid))),
			)
			.set(avatar.eq(&client_avatar))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_client_icon(&self, id: ClientId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let client = if let Some(r) = self.data.clients.get(&id) {
			r
		} else {
			bail!("Client not found");
		};
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		let client_icon = if client.icon_id.0 == 0 { None } else { Some(client.icon_id.0 as i32) };
		self.run(move |db, _| {
			use schema::servers_clients::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				servers_clients.filter(server.eq(&server_id).and(client.eq(&client_uid))),
			)
			.set(icon.eq(&client_icon))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_client_name(&self, id: ClientId) -> Result<()> {
		let client = if let Some(r) = self.data.clients.get(&id) {
			r
		} else {
			bail!("Client not found");
		};
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		let client_name = client.name.clone();
		self.run(move |db, _| {
			use schema::clients::dsl::*;
			// Update, ignored if not exists
			diesel::update(clients.filter(uid.eq(&client_uid)))
				.set(name.eq(&client_name))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_add_channel(&self, ch_id: ChannelId) -> Result<()> {
		let ch_server = self.get_server_key()?;
		let channel = match self.data.channels.get(&ch_id) {
			Some(c) => c,
			None => bail!("Failed to find channel"),
		};
		let ch_parent = if channel.parent.0 == 0 { None } else { Some(channel.parent.0 as i64) };
		let icon_id = channel.icon_id.and_then(|i| if i.0 == 0 { None } else { Some(i.0 as i32) });
		let ch_name = channel.name.clone();
		let ch_order = if channel.order.0 == 0 { None } else { Some(channel.order.0 as i64) };
		let ch_icon = channel.icon_id.map(|i| i.0 as i32);

		self.run(move |db, _| {
			use schema::channels::dsl::*;

			// Check if we already know this channel
			if diesel::select(diesel::dsl::exists(
				channels.filter(server.eq(&ch_server)).filter(id.eq(ch_id.0 as i64)),
			))
			.get_result(&db.con)?
			{
				// Update
				diesel::update(
					channels.filter(server.eq(&ch_server)).filter(id.eq(ch_id.0 as i64)),
				)
				.set((
					parent.eq(ch_parent),
					name.eq(&ch_name),
					order_id.eq(&ch_order),
					icon.eq(&ch_icon),
					deleted.eq(false),
				))
				.execute(&db.con)?;
			} else {
				let channel = models::ChannelInsert {
					server: &ch_server,
					id: ch_id.0 as i64,
					parent: ch_parent,
					order_id: ch_order,
					name: &ch_name,
					icon: icon_id,
					deleted: false,
				};
				diesel::replace_into(schema::channels::table).values(&channel).execute(&db.con)?;
			}
			Ok(())
		});
		Ok(())
	}

	fn handle_remove_channel(&self, ch_id: ChannelId) -> Result<()> {
		let ch_server = self.get_server_key()?;

		self.run(move |db, _| {
			use schema::channels::dsl::*;

			// Mark channel as deleted
			diesel::update(channels.filter(server.eq(&ch_server).and(id.eq(ch_id.0 as i64))))
				.set(deleted.eq(true))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_parent(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_parent = channel.parent;
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(channels.filter(server.eq(&server_id).and(id.eq(ch_id.0 as i64))))
				.set(parent.eq(ch_parent.0 as i64))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_order(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_order = channel.order;
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(channels.filter(server.eq(&server_id).and(id.eq(ch_id.0 as i64))))
				.set(order_id.eq(ch_order.0 as i64))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_name(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_name = channel.name.clone();
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(channels.filter(server.eq(&server_id).and(id.eq(ch_id.0 as i64))))
				.set(name.eq(&ch_name))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_icon(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_icon = channel.icon_id.and_then(|i| if i.0 == 0 { None } else { Some(i.0 as i32) });
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(channels.filter(server.eq(&server_id).and(id.eq(ch_id.0 as i64))))
				.set(icon.eq(&ch_icon))
				.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_message(
		&self, target: MessageTarget, invoker: &Invoker, message: &str,
	) -> Result<()> {
		let server = self.get_server_key()?;

		let invoker_uid = if let Some(uid) = &invoker.uid { Some(uid.0.clone()) } else { None };
		let invoker_name = if invoker_uid.is_none() { Some(invoker.name.clone()) } else { None };

		let own_message = invoker.id == self.data.own_client;

		// Make sure the chat exists
		let chat;
		// If it is possible that the message is inserted by
		// multiple clients which are connected to the same server.
		// We have to make sure that only one instance of a message
		// is inserted into the database.
		let can_be_duplicate;
		match target {
			MessageTarget::Server => {
				can_be_duplicate = true;
				chat = ChatType::Server;
			}
			MessageTarget::Channel => {
				can_be_duplicate = true;
				let client = self.data.clients.get(&self.data.own_client);
				let own_client = if let Some(client) = client {
					client
				} else {
					bail!("Failed to find own client");
				};
				chat = ChatType::Channel(own_client.channel.0);
			}
			MessageTarget::Client(id) => {
				can_be_duplicate = false;
				let client = self.data.clients.get(&id);
				let client_uid = if own_message {
					client.and_then(|c| c.uid.as_ref()).map(|u| &u.0)
				} else {
					invoker.uid.as_ref().map(|uid| &uid.0)
				};

				let client_uid = if let Some(uid) = client_uid {
					uid
				} else {
					bail!("Failed to find client");
				};
				chat = ChatType::Client(client_uid.clone());
			}
			MessageTarget::Poke(id) => {
				can_be_duplicate = false;
				let client = self.data.clients.get(&id);
				let uid = client.and_then(|c| c.uid.as_ref());
				if let Some(uid) = uid {
					chat = ChatType::Poke(uid.0.clone());
				} else {
					bail!("Client has no uid");
				}
			}
		}

		let message = message.to_string();
		self.run(move |db, _| {
			use schema::messages;

			let chat = db.get_or_create_chat(&ChatId { server, chat_type: chat })?;

			let (utc_time, utc_to_local_offset) = Self::get_now();
			db.last_message_id = db.con.transaction::<_, diesel::result::Error, _>(|| {
				if can_be_duplicate {
					// Check if the message is already in the database
					let start_check_time = utc_time - Duration::seconds(1);
					let cmp = messages::chat
						.eq(chat)
						.and(messages::invoker.eq(&invoker_uid))
						.and(messages::invoker_name.eq(&invoker_name))
						.and(messages::content.eq(&message))
						.and(messages::time.gt(&start_check_time))
						.and(messages::id.gt(db.last_message_id));
					let id = messages::table
						.filter(cmp)
						.select(messages::id)
						.first::<i64>(&db.con)
						.optional()?;

					if let Some(id) = id {
						return Ok(id);
					}
				}

				let invoker_uid = invoker_uid.as_deref();
				if own_message {
					// Check if the message is already in the database
					let cmp = messages::chat
						.eq(chat)
						.and(messages::invoker.eq(invoker_uid))
						.and(messages::content.eq(&message))
						.and(messages::status.eq(MessageStatus::Sending));
					// Update status
					let res = diesel::update(messages::table.filter(cmp))
						.set(messages::status.eq(MessageStatus::Success))
						.execute(&db.con)?;

					if res != 0 {
						// Successfully updated
						return messages::table
							.order(messages::id.desc())
							.select(messages::id)
							.first::<i64>(&db.con);
					}
				}

				// Insert message
				let message = models::MessageInsert {
					chat,
					invoker: invoker_uid,
					invoker_name: invoker_name.as_deref(),
					content: &message,
					status: MessageStatus::Success,
					time: &utc_time,
					timezone: utc_to_local_offset,
				};
				diesel::insert_into(messages::table).values(&message).execute(&db.con)?;
				messages::table
					.order(messages::id.desc())
					.select(messages::id)
					.first::<i64>(&db.con)
			})?;
			Ok(())
		});
		Ok(())
	}

	/// On channellistfinished: Mark channels as removed which are no longer
	/// there.
	fn handle_channellistfinished(&self) -> Result<()> {
		let server = self.get_server_key()?;
		let channels = self.data.channels.keys().map(|id| id.0 as i64).collect::<Vec<_>>();

		self.run(move |db, _| {
			use schema::channels;

			diesel::update(
				channels::table
					.filter(channels::server.eq(&server).and(channels::id.ne_all(&channels))),
			)
			.set(channels::deleted.eq(true))
			.execute(&db.con)?;
			Ok(())
		});

		Ok(())
	}
}
