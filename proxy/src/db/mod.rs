use std::fs;
use std::result;
use std::sync::Arc;

use actix::*;
use anyhow::{bail, format_err, Result};
use chrono::offset::{FixedOffset, TimeZone};
use chrono::{Duration, Local, NaiveDateTime, Utc};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use futures::prelude::*;
use slog::{error, info, trace, warn, Logger};
use tsclientlib::data::Client;
use tsclientlib::data::Connection as TsData;
use tsclientlib::events::{Event, PropertyId, PropertyValue};
use tsclientlib::Connection as TsConnection;
use tsclientlib::{ChannelId, ClientId, Identity, InMessage, Invoker, MessageTarget, UidBuf};
use tsproto_types::crypto::EccKeyPubP256;

use crate::filecache::FileCache;
use crate::search::Search;
use crate::secret::Secret;
use crate::{LaunchConfig, State};
use models::MessageStatus;

pub(crate) mod graphql;
mod models;
pub mod schema;

type DieselResult<T> = std::result::Result<T, diesel::result::Error>;

diesel_migrations::embed_migrations!();

pub struct DbHandler {
	logger: Logger,
	file_cache: Arc<FileCache>,
	search: Option<Arc<Search>>,
	secret: Secret,
	pub con: SqliteConnection,
	last_message_id: i64,
}

struct EventHandler<'a> {
	logger: &'a Logger,
	state: &'a State,
	con: &'a TsConnection,
	data: &'a TsData,
}

/// Identity id, `true` will create a new identity if this id does not exist.
#[derive(Clone, Debug)]
pub struct GetIdentityAndServerMsg {
	pub id: u64,
	pub create: bool,
	pub address: String,
}
#[derive(Clone, Debug)]
pub struct GetClientVolumeMsg(pub UidBuf);
#[derive(Clone, Debug)]
pub struct SetClientVolumeMsg(pub UidBuf, pub f32);
pub struct UpdateIdentityMsg(pub Identity);
pub struct RunOnDbMsg<I: 'static, E: 'static, F: FnOnce(&mut DbHandler) -> result::Result<I, E>>(
	pub F,
);

/// After we connected successfully to a server.
pub struct ConnectedMsg {
	pub bookmark: Option<i64>,
	pub username: String,
	pub address: String,
	pub channel: Option<String>,
	pub identity: i64,
	pub password: Option<String>,
	pub channel_password: Option<String>,
	pub server_key: EccKeyPubP256,
}

/// After all channels are available.
pub enum ChannelListMsg {
	/// Create the bookmark with the right channel reference.
	CreateBookmark(ConnectedMsg),
	/// Set the channel reference of the bookmark.
	UpdateChannel { bookmark: i64, server: Vec<u8>, channel: String },
}

pub struct ClientData {
	pub name: String,
	pub uid: UidBuf,
	pub icon: Option<i32>,
	pub avatar: Option<String>,
	pub phonetic_name: Option<String>,
	pub description: Option<String>,
}

pub struct WriteMessageMsg {
	pub message: String,
	pub invoker_uid: UidBuf,
	pub chat: ChatId,
	pub client_data: Option<ClientData>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatId {
	/// The public key of the server.
	pub server: EccKeyPubP256,
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

impl Message for GetIdentityAndServerMsg {
	type Result = Result<(Identity, Option<UidBuf>)>;
}
impl Message for GetClientVolumeMsg {
	type Result = Result<Option<f32>>;
}
impl Message for SetClientVolumeMsg {
	type Result = Result<()>;
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
	type Result = Result<Option<ChannelListMsg>>;
}
impl Message for ChannelListMsg {
	type Result = Result<()>;
}
impl Message for RunMsg {
	type Result = Result<()>;
}

impl DbHandler {
	pub(crate) fn new(
		logger: Logger, file_cache: Arc<FileCache>, search: Option<Arc<Search>>,
		launch_config: &LaunchConfig, secret: Secret,
	) -> Result<Self> {
		let database_url = launch_config.config_path.join("storage.sqlite");
		let con = SqliteConnection::establish(database_url.to_str().unwrap())?;

		// The database can be opened successfully, create backup
		fs::copy(database_url, launch_config.config_path.join("storage.sqlite.bak"))?;

		// Enforce foreign keys constraints
		// Enable wal mode for more concurrency and faster writes
		// Use busy_timeout to retry operations when the database is locked (timeout in
		// milliseconds)
		con.batch_execute(
			"PRAGMA synchronous = NORMAL; PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; \
			 PRAGMA busy_timeout = 1000",
		)?;

		// Run migrations
		let mut s = Vec::new();
		embedded_migrations::run_with_output(&con, &mut s)?;
		let s = std::str::from_utf8(&s)?;
		if !s.is_empty() {
			info!(logger, "Run database migrations"; "output" => s);
		}

		Ok(Self { logger, file_cache, search, secret, con, last_message_id: 0 })
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
						.find(id.server.to_short().as_slice())
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
								server_chats::server.eq(id
									.server
									.to_short()
									.as_slice()),
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
						.find((
							id.server.to_short().as_slice(),
							*channel as i64,
						))
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
								channel_chats::server.eq(id
									.server
									.to_short()
									.as_slice()),
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
						.find((id.server.to_short().as_slice(), &client))
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
								client_chats::server.eq(id
									.server
									.to_short()
									.as_slice()),
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
						.find((id.server.to_short().as_slice(), &client))
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
								client_pokes::server.eq(id
									.server
									.to_short()
									.as_slice()),
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

	/// Name can be an id if it starts with `/`, otherwise a path to a channel, separated by `/`.
	fn find_channel(&self, server: &[u8], channel: &str) -> Result<i64> {
		use diesel::dsl::{exists, select};
		use schema::channels;

		if channel.starts_with('/') {
			let id: u64 = channel[1..].parse()?;
			let id = id as i64;
			// Check if we know this channel
			if select(exists(
				channels::table.filter(channels::server.eq(&server).and(channels::id.eq(id))),
			))
			.get_result(&self.con)?
			{
				Ok(id)
			} else {
				Err(format_err!("Channel not found"))
			}
		} else {
			// Traverse the channel tree
			let mut parent: Option<i64> = None;
			for part in channel.split('/') {
				let cmp = channels::server.eq(&server).and(channels::name.eq(part));
				if let Some(p) = parent {
					parent = Some(
						channels::table
							.filter(cmp.and(channels::parent.eq(p)))
							.select(channels::id)
							.first::<i64>(&self.con)?,
					);
				} else {
					parent = Some(
						channels::table
							.filter(cmp.and(channels::parent.is_null()))
							.select(channels::id)
							.first::<i64>(&self.con)?,
					);
				}
			}
			parent.ok_or_else(|| format_err!("Failed to find channel"))
		}
	}
}

impl Handler<GetIdentityAndServerMsg> for DbHandler {
	type Result = Result<(Identity, Option<UidBuf>)>;
	fn handle(&mut self, msg: GetIdentityAndServerMsg, _: &mut Self::Context) -> Self::Result {
		use schema::bookmarks;
		use schema::identities::dsl::*;

		// Search server
		let server = bookmarks::table
			.filter(bookmarks::address.eq(&msg.address))
			.select(bookmarks::server)
			.first::<Option<Vec<u8>>>(&self.con)
			.optional()?
			.flatten()
			.map(|key| {
				EccKeyPubP256::from_short(&key).and_then(|key| key.get_uid_no_base64()).map(UidBuf)
			})
			.transpose()?;

		// Search identity
		match identities.find(msg.id as i64).first::<models::Identity>(&self.con) {
			Ok(r) => r.into_identity(&self.secret).map(|i| (i, server)),
			Err(_) => {
				// Pick an existing identity if one exists
				if let Ok(r) = identities.order(id).first::<models::Identity>(&self.con) {
					return r.into_identity(&self.secret).map(|i| (i, server));
				}
				if !msg.create {
					bail!("No identity found");
				}

				// Create new identity
				let identity = tsclientlib::Identity::create()?;
				let pub_key = identity.key().to_pub();
				let uid = pub_key.get_uid_no_base64()?;
				let client_key = pub_key.to_short();

				let cli = models::ClientInsert {
					uid: &uid,
					name: "TeamSpeakUser",
					public_key: Some(client_key.as_slice()),
					custom_name: None,
					custom_phonetic_name: None,
				};
				diesel::insert_into(schema::clients::table).values(&cli).execute(&self.con)?;

				let new_identity = models::NewIdentity::new(&identity, &uid, &self.secret)?;
				diesel::insert_into(identities).values(&new_identity).execute(&self.con)?;

				Ok((identity, server))
			}
		}
	}
}

impl Handler<GetClientVolumeMsg> for DbHandler {
	type Result = Result<Option<f32>>;
	fn handle(&mut self, msg: GetClientVolumeMsg, _: &mut Self::Context) -> Self::Result {
		use schema::clients::dsl::*;
		Ok(clients.find(&msg.0.0).select(volume).first::<f32>(&self.con).optional()?)
	}
}

impl Handler<SetClientVolumeMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(&mut self, msg: SetClientVolumeMsg, _: &mut Self::Context) -> Self::Result {
		use schema::clients::dsl::*;
		let res =
			diesel::update(clients.find(&msg.0.0)).set(volume.eq(msg.1)).execute(&self.con)?;
		if res != 1 {
			bail!("Failed to find client in database");
		}
		Ok(())
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
		if diesel::update(identities.filter(client.eq(uid)))
			.set((
				counter.eq(identity.counter() as i64),
				max_counter.eq(identity.max_counter() as i64),
			))
			.execute(&self.con)?
			!= 1
		{
			bail!("Identity not found");
		}
		Ok(())
	}
}

impl Handler<WriteMessageMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(&mut self, message: WriteMessageMsg, _: &mut Self::Context) -> Self::Result {
		use schema::{chats, messages};

		let (utc_time, utc_to_local_offset) = EventHandler::get_now();

		if let Some(client) = &message.client_data {
			// Register client
			self.add_client_internal(true, &message.chat.server, client)?;
		}

		let chat = self.get_or_create_chat(&message.chat)?;
		// Insert message with state sending (except for pokes, which do not receive again)
		let status = if let ChatType::Poke(_) = message.chat.chat_type {
			MessageStatus::Success
		} else {
			MessageStatus::Sending
		};
		let msg = models::MessageInsert {
			chat,
			invoker: Some(&message.invoker_uid.0),
			invoker_name: None,
			content: &message.message,
			status,
			time: &utc_time,
			timezone: utc_to_local_offset,
		};
		let message_id = self.con.transaction::<_, diesel::result::Error, _>(|| {
			diesel::insert_into(messages::table).values(&msg).execute(&self.con)?;
			messages::table.order(messages::id.desc()).select(messages::id).first::<i64>(&self.con)
		})?;

		// Update last read from the chat
		diesel::update(chats::table.find(chat))
			.set((chats::last_read.eq(&utc_time), chats::timezone.eq(utc_to_local_offset)))
			.execute(&self.con)?;

		// Add to search db
		if let Some(search) = &self.search {
			search.add_message(message_id as u64, message.message)?;
		}

		Ok(())
	}
}

impl Handler<ConnectedMsg> for DbHandler {
	type Result = Result<Option<ChannelListMsg>>;
	/// Has to be called after the server was added in handle_connected.
	///
	/// Returns the id of a bookmark/recent connection if it should be updated later with the
	/// correct channel reference.
	fn handle(&mut self, msg: ConnectedMsg, _: &mut Self::Context) -> Self::Result {
		use schema::{bookmarks, identities};
		let server = msg.server_key.to_short();
		let (utc_time, utc_to_local_offset) = EventHandler::get_now();

		if let Some(id) = msg.bookmark {
			trace!(self.logger, "Connected: Update used bookmark"; "bookmark" => id);
			// Update
			if diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
				.set((
					bookmarks::last_used.eq(Some(utc_time)),
					bookmarks::timezone.eq(utc_to_local_offset),
				))
				.execute(&self.con)?
				!= 1
			{
				bail!("Failed to update time of bookmark {}, not found", id);
			}
			Ok(None)
		} else {
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

			let cmp = bookmarks::address
				.eq(&msg.address)
				.and(bookmarks::username.eq(&msg.username))
				.and(bookmarks::identity.eq(identity));

			let mut channel_id = None;
			let bookmark_id = if let Some(channel) = &msg.channel {
				match self.find_channel(server.as_slice(), channel) {
					Ok(id) => {
						channel_id = Some(id);
						trace!(self.logger, "Connected: Found channel for bookmark";
							"channel" => id);
						bookmarks::table
							.filter(cmp.and(bookmarks::channel.eq(id)))
							.select(bookmarks::id)
							.first::<i64>(&self.con)
							.optional()?
					}
					// Ignore missing channels, create a new bookmark for now
					Err(_) => {
						// Check if there exists a bookmark for the server without a channel
						if let Some(id) = bookmarks::table
							.filter(cmp.and(bookmarks::channel.is_null()))
							.select(bookmarks::id)
							.first::<i64>(&self.con)
							.optional()?
						{
							trace!(self.logger, "Connected: Did not find channel for \
								bookmark, but found bookmark without channel, updating later";
								"other_bookmark" => id);
							// Create or update later
							let mut msg = msg;
							msg.identity = identity;
							msg.bookmark = Some(id);
							return Ok(Some(ChannelListMsg::CreateBookmark(msg)));
						} else {
							trace!(
								self.logger,
								"Connected: Did not find channel for bookmark, creating without \
								 channel"
							);
							// Create without channel and update later
							None
						}
					}
				}
			} else {
				bookmarks::table
					.filter(cmp.and(bookmarks::channel.is_null()))
					.select(bookmarks::id)
					.first::<i64>(&self.con)
					.optional()?
			};

			if let Some(id) = bookmark_id {
				// Update
				trace!(self.logger, "Connected: Update existing bookmark"; "bookmark" => id);
				if diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
					.set((
						bookmarks::last_used.eq(Some(utc_time)),
						bookmarks::timezone.eq(utc_to_local_offset),
					))
					.execute(&self.con)? != 1
				{
					bail!("Failed to update time of bookmark {}, not found", id);
				}
				Ok(None)
			} else {
				let bookmark = models::BookmarkInsert {
					name: None,
					username: &msg.username,
					address: &msg.address,
					channel: channel_id,
					identity,
					bookmark: false,
					last_used: Some(utc_time),
					timezone: utc_to_local_offset,
					password: msg.password.as_deref(),
					channel_password: msg.channel_password.as_deref(),
					server: Some(server.as_slice()),
				};
				let id = self.con.transaction::<_, diesel::result::Error, _>(|| {
					diesel::insert_into(bookmarks::table).values(&bookmark).execute(&self.con)?;
					bookmarks::table
						.order(bookmarks::id.desc())
						.select(bookmarks::id)
						.first::<i64>(&self.con)
				})?;
				trace!(self.logger, "Connected: Created new bookmark"; "bookmark" => id,
					"channel_id" => ?channel_id, "channel" => ?msg.channel.as_ref());
				if msg.channel.is_some() && channel_id.is_none() {
					Ok(Some(ChannelListMsg::UpdateChannel {
						bookmark: id,
						server: server.as_slice().to_vec(),
						channel: msg.channel.unwrap(),
					}))
				} else {
					Ok(None)
				}
			}
		}
	}
}

impl Handler<ChannelListMsg> for DbHandler {
	type Result = Result<()>;
	fn handle(&mut self, msg: ChannelListMsg, _: &mut Self::Context) -> Self::Result {
		use schema::bookmarks;

		match msg {
			ChannelListMsg::CreateBookmark(data) => {
				let server = data.server_key.to_short();
				let (utc_time, utc_to_local_offset) = EventHandler::get_now();
				if let Some(channel) = &data.channel {
					match self.find_channel(server.as_slice(), channel) {
						Ok(channel) => {
							// Create new bookmark with channel
							trace!(self.logger, "ChannelList: Create new bookmark";
								"channel" => channel);
							let bookmark = models::BookmarkInsert {
								name: None,
								username: &data.username,
								address: &data.address,
								channel: Some(channel),
								identity: data.identity,
								bookmark: false,
								last_used: Some(utc_time),
								timezone: utc_to_local_offset,
								password: data.password.as_deref(),
								channel_password: data.channel_password.as_deref(),
								server: Some(server.as_slice()),
							};
							diesel::insert_into(bookmarks::table)
								.values(&bookmark)
								.execute(&self.con)?;
						}
						Err(_) => {
							if let Some(id) = data.bookmark {
								// Update existing bookmark without channel
								trace!(self.logger, "ChannelList: Update bookmark without \
									channel";
									"bookmark" => id, "channel" => channel);
								if diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
									.set((
										bookmarks::last_used.eq(Some(utc_time)),
										bookmarks::timezone.eq(utc_to_local_offset),
									))
									.execute(&self.con)? != 1
								{
									bail!("Failed to update time of bookmark {}, not found", id);
								}
							} else {
								bail!(
									"Bookmarks that are created after channellistfinished need an \
									 id"
								);
							}
						}
					}
				} else {
					bail!("Bookmarks that are created after channellistfinished need a channel");
				}
			}
			ChannelListMsg::UpdateChannel { bookmark, server, channel } => {
				let channel = self.find_channel(&server, &channel)?;
				if diesel::update(bookmarks::table.filter(bookmarks::id.eq(bookmark)))
					.set(bookmarks::channel.eq(channel))
					.execute(&self.con)? != 1
				{
					bail!("Failed to update channel of bookmark {}, not found", bookmark);
				}
			}
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
	pub(crate) fn handle_events(
		logger: &Logger, state: &State, con: &TsConnection, data: &TsData, events: &[Event],
		connected_msg: Option<ConnectedMsg>, ws: Addr<crate::websocket::Ws>,
	) -> Result<()> {
		let handler = EventHandler::new(logger, state, con, data);

		for e in events {
			let r = match e {
				Event::PropertyAdded { id, .. } => {
					match id {
						PropertyId::Server => handler.handle_connected(),
						PropertyId::Client(id) => {
							if let Some(client) = data.clients.get(id) {
								Self::register_client(&handler, e, data, Some(client));
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
						PropertyId::ServerIcon => handler.handle_server_icon(),

						PropertyId::ClientAvatarHash(id) => {
							Self::register_client(&handler, e, data, data.clients.get(id));
							handler.handle_client_avatar(*id)
						}
						PropertyId::ClientIcon(id) => {
							Self::register_client(&handler, e, data, data.clients.get(id));
							handler.handle_client_icon(*id)
						}
						PropertyId::ClientName(id) => handler.handle_client_name(*id),
						// TODO register_client for other changes
						PropertyId::ClientChannel(id) => {
							Self::register_client(&handler, e, data, data.clients.get(id));
							Ok(())
						}

						PropertyId::ChannelParent(id) => handler.handle_channel_parent(*id),
						PropertyId::ChannelOrder(id) => handler.handle_channel_order(*id),
						PropertyId::ChannelName(id) => handler.handle_channel_name(*id),
						PropertyId::ChannelIcon(id) => handler.handle_channel_icon(*id),
						_ => Ok(()),
					}
				}
				Event::PropertyRemoved { id, old, .. } => match id {
					PropertyId::Client(_) => {
						let client = match old {
							PropertyValue::Client(client) => client,
							_ => panic!("Property value should be a client but wasn't"),
						};
						Self::register_client(&handler, e, data, Some(client));
						handler.handle_remove_client(client)
					}
					PropertyId::Channel(id) => handler.handle_remove_channel(*id),
					_ => Ok(()),
				},
				Event::Message { target, invoker, message } => {
					if let MessageTarget::Client(id) = target {
						Self::register_client(&handler, e, data, data.clients.get(id));
					} else {
						Self::register_client(
							&handler,
							e,
							data,
							data.clients.get(&data.own_client),
						);
					}
					handler.handle_message(*target, invoker, message)
				}
			};

			if let Err(e) = r {
				error!(logger, "Failed to handle event for database"; "error" => %e);
			}
		}

		if let Some(msg) = connected_msg {
			let logger = logger.clone();
			actix::spawn(state.database.send(msg).map(move |r| match r {
				Err(e) => warn!(logger, "Failed to save connection in database"; "error" => %e),
				Ok(Err(e)) => warn!(logger, "Failed to save connection in database"; "error" => %e),
				Ok(Ok(Some(msg))) => {
					actix::spawn(ws.send(crate::websocket::SetChannelListMsgMsg(msg)).map(
						move |r| match r {
							Err(e) => warn!(logger, "Failed to set update bookmark message";
							"error" => %e),
							Ok(()) => {}
						},
					));
				}
				Ok(Ok(None)) => {}
			}));
		}

		Ok(())
	}

	pub(crate) fn handle_message(
		logger: &Logger, state: &State, con: &TsConnection, data: &TsData, msg: &InMessage,
	) -> Result<()> {
		let handler = EventHandler::new(logger, state, con, data);
		if let InMessage::ChannelListFinished(_) = msg {
			handler.handle_channellistfinished()
		} else {
			Ok(())
		}
	}

	pub fn create_client(
		logger: &Logger, state: &State, con: &TsConnection, data: &TsData, client: &Client,
	) -> Result<()> {
		let handler = EventHandler::new(logger, state, con, data);
		handler.handle_add_client(client, true)
	}

	/// Only add clients to database when we interact with them:
	/// We receive or write a message to them, we are modified with them
	/// as invoker or they are modified with us as invoker.
	fn register_client(handler: &EventHandler, e: &Event, data: &TsData, client: Option<&Client>) {
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
						error!(handler.logger, "Failed to handle event for database";
							"error" => %e);
					}
				}
			}
		}
	}

	fn add_client_internal(
		&self, create: bool, server: &EccKeyPubP256, client: &ClientData,
	) -> Result<()> {
		use schema::clients::dsl::*;
		use schema::servers_clients;

		// Check if we already know this client, the update will return 1 changed row on success
		if diesel::update(clients.filter(uid.eq(&client.uid.0)))
			.set(name.eq(&client.name))
			.execute(&self.con)?
			!= 1
		{
			if !create {
				return Ok(());
			}

			let client = models::ClientInsert {
				uid: &client.uid.0,
				name: &client.name,
				public_key: None,
				custom_name: None,
				custom_phonetic_name: None,
			};
			diesel::insert_into(schema::clients::table).values(&client).execute(&self.con)?;
		}

		// Add to search db
		if let Some(search) = &self.search {
			search.add_client(
				&client.uid,
				client.name.clone(),
				client.phonetic_name.clone(),
				None,
				None,
				client.description.clone(),
			)?;
		}

		let (utc_time, utc_to_local_offset) = EventHandler::get_now();
		let server_key = server.to_short();
		let server_key = server_key.as_slice();
		let server_client = models::ServersClientsInsert {
			server: server_key,
			client: &client.uid.0,
			icon: client.icon,
			avatar: client.avatar.as_deref(),
			last_seen: utc_time,
			timezone: utc_to_local_offset,
		};

		// Check if the avatar changed
		let prev_avatar = servers_clients::table
			.filter(
				servers_clients::server
					.eq(server_key)
					.and(servers_clients::client.eq(&client.uid.0)),
			)
			.select(servers_clients::avatar)
			.first::<Option<String>>(&self.con)
			.optional()?;

		if let Some(prev_avatar) = prev_avatar {
			if prev_avatar != client.avatar {
				// Remove cached avatar
				if let Err(e) = self.file_cache.delete_file(
					server,
					ChannelId(0),
					&format!("avatar_{}", client.uid.as_avatar()),
				) {
					warn!(self.logger, "Failed to delete cached file"; "error" => %e);
				}
			}
		}

		diesel::replace_into(servers_clients::table).values(&server_client).execute(&self.con)?;
		Ok(())
	}
}

impl<'a> EventHandler<'a> {
	fn new(logger: &'a Logger, state: &'a State, con: &'a TsConnection, data: &'a TsData) -> Self {
		Self { logger, state, con, data }
	}

	fn run<
		F: FnOnce(&mut DbHandler, &mut <DbHandler as Actor>::Context) -> Result<()> + Send + 'static,
	>(
		&self, f: F,
	) {
		let logger = self.logger.clone();
		actix::spawn(self.state.database.send(RunMsg(Box::new(f))).map(move |r| match r {
			Err(e) => {
				error!(logger, "Failed to send to database"; "error" => %e);
			}
			Ok(Err(e)) => {
				error!(logger, "Failed to write to database"; "error" => %e);
			}
			_ => {}
		}))
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
		let search = self.state.search.clone();
		let key = self.con.get_server_key()?;
		let icon_id =
			if self.data.server.icon.0 != 0 { Some(self.data.server.icon.0 as i32) } else { None };
		let server_name = self.data.server.name.clone();
		let addr = self.con.get_options().get_address().to_string();
		let host_msg = self.data.server.hostmessage.clone();
		let welcome_msg = self.data.server.welcome_message.clone();

		self.run(move |db, _| {
			use schema::servers::dsl::*;

			// Check if we already know that server, the update will return 1 changed row on success
			if diesel::update(
				servers.filter(public_key.eq(key.to_short().as_slice())),
			)
			.set((name.eq(&server_name), address.eq(&addr), icon.eq(&icon_id)))
			.execute(&db.con)?
				!= 1
			{
				let server_key = key.to_short();
				let server = models::ServerInsert {
					public_key: server_key.as_slice(),
					name: &server_name,
					address: &addr,
					icon: icon_id,
				};
				diesel::insert_into(schema::servers::table).values(&server).execute(&db.con)?;
			}
			// Add to search db
			if let Some(search) = search {
				search.add_server(key, addr, server_name, Some(host_msg), Some(welcome_msg))?;
			}

			Ok(())
		});
		Ok(())
	}

	fn handle_server_name(&self) -> Result<()> {
		let key = self.con.get_server_key()?;
		let server_name = self.data.server.name.clone();
		self.run(move |db, _| {
			use schema::servers::dsl::*;
			if diesel::update(
				servers.filter(public_key.eq(key.to_short().as_slice())),
			)
			.set(name.eq(&server_name))
			.execute(&db.con)?
				!= 1
			{
				bail!(
					"Failed to update server name to {:?}, server {:?} not found",
					server_name,
					key
				);
			}
			Ok(())
		});
		Ok(())
	}

	fn handle_server_icon(&self) -> Result<()> {
		let key = self.con.get_server_key()?;
		let icon_id =
			if self.data.server.icon.0 != 0 { Some(self.data.server.icon.0 as i32) } else { None };
		self.run(move |db, _| {
			use schema::servers::dsl::*;
			if diesel::update(
				servers.filter(public_key.eq(key.to_short().as_slice())),
			)
			.set(icon.eq(&icon_id))
			.execute(&db.con)?
				!= 1
			{
				bail!("Failed to update server icon to {:?}, server {:?} not found", icon_id, key);
			}
			Ok(())
		});
		Ok(())
	}

	fn handle_add_invoker(&self, invoker: &Invoker) -> Result<()> {
		match self.data.clients.get(&invoker.id) {
			Some(client) => self.handle_add_client(client, true),
			None => {
				if let Some(uid) = &invoker.uid {
					self.handle_add_client_internal(true, ClientData {
						name: invoker.name.clone(),
						uid: uid.clone(),
						icon: None,
						avatar: None,
						phonetic_name: None,
						description: None,
					})
				} else {
					Ok(())
				}
			}
		}
	}

	pub fn handle_add_client(&self, client: &Client, create: bool) -> Result<()> {
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.clone(),
			None => bail!("Client has no uid"),
		};

		let icon = if client.icon.0 == 0 { None } else { Some(client.icon.0 as i32) };
		let avatar =
			if client.avatar_hash.is_empty() { None } else { Some(client.avatar_hash.clone()) };
		self.handle_add_client_internal(create, ClientData {
			name: client.name.clone(),
			uid: client_uid,
			icon,
			avatar,
			phonetic_name: if client.phonetic_name != "" {
				Some(client.phonetic_name.clone())
			} else {
				None
			},
			description: if client.description != "" {
				Some(client.description.clone())
			} else {
				None
			},
		})
	}

	fn handle_add_client_internal(&self, create: bool, client: ClientData) -> Result<()> {
		let server = self.con.get_server_key()?;
		self.run(move |db, _| db.add_client_internal(create, &server, &client));
		Ok(())
	}

	fn handle_remove_client(&self, client: &Client) -> Result<()> {
		// If own client removed, handle for all other clients
		if client.id == self.data.own_client {
			self.handle_disconnect()?;
		}

		let server = self.con.get_server_key()?;
		let client_uid = match &client.uid {
			Some(uid) => uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		self.run(move |db, _| {
			use schema::servers_clients;

			// Update last seen
			let (utc_time, utc_to_local_offset) = Self::get_now();

			diesel::update(
				servers_clients::table.filter(
					servers_clients::server
						.eq(server.to_short().as_slice())
						.and(servers_clients::client.eq(&client_uid)),
				),
			)
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
		let server = self.con.get_server_key()?;
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

			diesel::update(
				servers_clients::table.filter(
					servers_clients::server
						.eq(server.to_short().as_slice())
						.and(servers_clients::client.eq_any(&uids)),
				),
			)
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
		let server_id = self.con.get_server_key()?;
		let client = if let Some(r) = self.data.clients.get(&id) {
			r
		} else {
			bail!("Client not found");
		};
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.clone(),
			None => bail!("Client has no uid"),
		};

		// Remove cached avatar
		if let Err(e) = self.state.file_cache.delete_file(
			&server_id,
			ChannelId(0),
			&format!("avatar_{}", client_uid.as_avatar()),
		) {
			warn!(self.logger, "Failed to delete cached file"; "error" => %e);
		}

		let client_avatar =
			if client.avatar_hash.is_empty() { None } else { Some(client.avatar_hash.clone()) };
		self.run(move |db, _| {
			use schema::servers_clients::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				servers_clients.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(client.eq(&client_uid.0)),
				),
			)
			.set(avatar.eq(&client_avatar))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_client_icon(&self, id: ClientId) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let client = if let Some(r) = self.data.clients.get(&id) {
			r
		} else {
			bail!("Client not found");
		};
		let client_uid = match &client.uid {
			Some(client_uid) => client_uid.0.clone(),
			None => bail!("Client has no uid"),
		};

		let client_icon = if client.icon.0 == 0 { None } else { Some(client.icon.0 as i32) };
		self.run(move |db, _| {
			use schema::servers_clients::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				servers_clients.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(client.eq(&client_uid)),
				),
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
		let search = self.state.search.clone();
		let ch_server = self.con.get_server_key()?;
		let channel = match self.data.channels.get(&ch_id) {
			Some(c) => c,
			None => bail!("Failed to find channel"),
		};
		let ch_parent = if channel.parent.0 == 0 { None } else { Some(channel.parent.0 as i64) };
		let icon_id = channel.icon.and_then(|i| if i.0 == 0 { None } else { Some(i.0 as i32) });
		let ch_name = channel.name.clone();
		let ch_order = if channel.order.0 == 0 { None } else { Some(channel.order.0 as i64) };
		let ch_topic = channel.topic.clone();

		self.run(move |db, _| {
			use schema::channels::dsl::*;

			// Check if we already know this channel, the update will return 1 changed row on
			// success
			if diesel::update(
				channels
					.filter(server.eq(ch_server.to_short().as_slice()))
					.filter(id.eq(ch_id.0 as i64)),
			)
			.set((
				parent.eq(ch_parent),
				name.eq(&ch_name),
				order_id.eq(&ch_order),
				icon.eq(&icon_id),
				deleted.eq(false),
			))
			.execute(&db.con)?
				!= 1
			{
				let server_key = ch_server.to_short();
				let channel = models::ChannelInsert {
					server: server_key.as_slice(),
					id: ch_id.0 as i64,
					parent: ch_parent,
					order_id: ch_order,
					name: &ch_name,
					icon: icon_id,
					deleted: false,
				};
				diesel::replace_into(schema::channels::table).values(&channel).execute(&db.con)?;
			}

			// Add to search db
			if let Some(search) = search {
				search.add_channel(ch_server, ch_id.0, ch_name, ch_topic, None)?;
			}

			Ok(())
		});
		Ok(())
	}

	fn handle_remove_channel(&self, ch_id: ChannelId) -> Result<()> {
		let ch_server = self.con.get_server_key()?;

		self.run(move |db, _| {
			use schema::channels::dsl::*;

			// Mark channel as deleted
			if diesel::update(
				channels.filter(
					server
						.eq(ch_server.to_short().as_slice())
						.and(id.eq(ch_id.0 as i64)),
				),
			)
			.set(deleted.eq(true))
			.execute(&db.con)?
				!= 1
			{
				bail!(
					"Failed to mark channel as deleted, channel ({:?}, {}) not found",
					ch_server,
					ch_id.0
				);
			}
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_parent(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_parent = channel.parent;
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				channels.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(id.eq(ch_id.0 as i64)),
				),
			)
			.set(parent.eq(ch_parent.0 as i64))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_order(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_order = channel.order;
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				channels.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(id.eq(ch_id.0 as i64)),
				),
			)
			.set(order_id.eq(ch_order.0 as i64))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_name(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_name = channel.name.clone();
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				channels.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(id.eq(ch_id.0 as i64)),
				),
			)
			.set(name.eq(&ch_name))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_channel_icon(&self, ch_id: ChannelId) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let channel = if let Some(r) = self.data.channels.get(&ch_id) {
			r
		} else {
			bail!("Channel not found");
		};
		let ch_icon = channel.icon.and_then(|i| if i.0 == 0 { None } else { Some(i.0 as i32) });
		self.run(move |db, _| {
			use schema::channels::dsl::*;
			// Update, ignored if not exists
			diesel::update(
				channels.filter(
					server
						.eq(server_id.to_short().as_slice())
						.and(id.eq(ch_id.0 as i64)),
				),
			)
			.set(icon.eq(&ch_icon))
			.execute(&db.con)?;
			Ok(())
		});
		Ok(())
	}

	fn handle_message(
		&self, target: MessageTarget, invoker: &Invoker, message: &str,
	) -> Result<()> {
		let search = self.state.search.clone();
		let server = self.con.get_server_key()?;

		let invoker_uid = if let Some(uid) = &invoker.uid { Some(uid.0.clone()) } else { None };
		let invoker_name = if invoker_uid.is_none() { Some(invoker.name.clone()) } else { None };

		let own_message = invoker.id == self.data.own_client;

		// Make sure the chat exists
		let chat;
		// If it is possible that the message is inserted by
		// multiple clients which are connected to the same server.
		// We have to make sure that only one instance of a message
		// is inserted into the database.
		match target {
			MessageTarget::Server => {
				chat = ChatType::Server;
			}
			MessageTarget::Channel => {
				let client = self.data.clients.get(&self.data.own_client);
				let own_client = if let Some(client) = client {
					client
				} else {
					bail!("Failed to find own client");
				};
				chat = ChatType::Channel(own_client.channel.0);
			}
			MessageTarget::Client(id) => {
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
			use diesel::dsl::not;
			use schema::messages;

			let chat = db.get_or_create_chat(&ChatId { server, chat_type: chat })?;

			let (utc_time, utc_to_local_offset) = Self::get_now();
			db.last_message_id = db.con.transaction::<_, diesel::result::Error, _>(|| {
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

				// Check if the message is already in the database
				let start_check_time = utc_time - Duration::seconds(1);

				// Compare uid and name: messages::invoker == invoker_uid
				// But with null == null
				//
				// https://stackoverflow.com/questions/10416789/how-to-rewrite-is-distinct-from-and-is-not-distinct-from
				// a IS NOT DISTINCT FROM b can be rewritten as:
				// (NOT (a <> b OR a IS NULL OR b IS NULL) OR (a IS NULL AND b IS NULL))
				let invoker_cmp = not(messages::invoker
					.ne(&invoker_uid)
					.or(messages::invoker.is_null())
					.or(invoker_uid.is_none()))
				.or(messages::invoker.is_null().and(invoker_uid.is_none()));
				let name_cmp = not(messages::invoker_name
					.ne(&invoker_name)
					.or(messages::invoker_name.is_null())
					.or(invoker_name.is_none()))
				.or(messages::invoker_name.is_null().and(invoker_name.is_none()));

				let cmp = messages::chat
					.eq(chat)
					.and(invoker_cmp)
					.and(name_cmp)
					.and(messages::content.eq(&message))
					.and(messages::time.gt(&start_check_time))
					.and(messages::id.ge(db.last_message_id));
				let id = messages::table
					.filter(cmp)
					.select(messages::id)
					.first::<i64>(&db.con)
					.optional()?;

				if let Some(id) = id {
					return Ok(id);
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
			// Add to search db
			if let Some(search) = search {
				search.add_message(db.last_message_id as u64, message)?;
			}

			Ok(())
		});
		Ok(())
	}

	/// On channellistfinished: Mark channels as removed which are no longer
	/// there.
	fn handle_channellistfinished(&self) -> Result<()> {
		let server_id = self.con.get_server_key()?;
		let channels = self.data.channels.keys().map(|id| id.0 as i64).collect::<Vec<_>>();

		self.run(move |db, _| {
			use schema::channels;

			diesel::update(
				channels::table.filter(
					channels::server
						.eq(server_id.to_short().as_slice())
						.and(channels::id.ne_all(&channels)),
				),
			)
			.set(channels::deleted.eq(true))
			.execute(&db.con)?;
			Ok(())
		});

		Ok(())
	}
}
