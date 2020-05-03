// Broken warning with juniper derive
#![allow(unused_braces)]

use std::convert::TryInto;
use std::sync::Arc;

use actix_web::*;
use anyhow::format_err;
use chrono::NaiveDateTime;
use diesel::prelude::*;
use futures::prelude::*;
use juniper::http::graphiql::graphiql_source;
use juniper::http::GraphQLRequest;
use juniper::{EmptySubscription, FieldError, RootNode, ID};
use slog::error;
use tsclientlib::Uid;

use super::models::MessageStatus;
use super::schema::bookmarks;
use super::{models, schema, RunOnDbMsg};
use crate::websocket::{SaveClientMsg, SetVolumeMsg};
use crate::{ConnectionId, State};

const BOOKMARKS_LIMIT: i64 = 20;
const MESSAGES_LIMIT: i64 = 50;

#[derive(Clone)]
pub struct Query;
#[derive(Clone)]
pub struct Mutation;

#[derive(juniper::GraphQLObject)]
struct Void {
	void: bool,
}

pub(crate) type Schema = RootNode<'static, Query, Mutation, EmptySubscription<State>>;
type GResult<T> = std::result::Result<T, FieldError>;

struct Bookmark(models::Bookmark);
struct Channel(models::Channel);
struct Chat(models::Chat);
struct Client(models::Client);
struct Identity(models::Identity);
struct Message(models::Message);
struct Server(models::Server);
struct ServerClient(models::ServersClients);

#[derive(Clone, Copy, Debug, Eq, PartialEq, juniper::GraphQLEnum)]
enum GMessageTarget {
	Server,
	Channel,
	Client,
	Poke,
}

#[derive(Debug, juniper::GraphQLInputObject)]
struct UpdateBookmark {
	id: ID,
	name: Option<String>,
	username: Option<String>,
	channel: Option<ID>,
	bookmark: Option<bool>,
}

#[derive(Debug, AsChangeset)]
#[table_name = "bookmarks"]
struct UpdateBookmarkDb {
	name: Option<String>,
	username: Option<String>,
	channel: Option<i64>,
	bookmark: Option<bool>,
}

#[get("/graphiql")]
pub async fn graphiql() -> impl Responder {
	HttpResponse::Ok().content_type("text/html; charset=utf-8").body(graphiql_source("/db", None))
}

#[post("/db")]
pub(crate) async fn db_graphql(
	state: web::Data<State>, data: web::Json<GraphQLRequest>,
) -> Result<impl Responder> {
	let res = data.execute(&state.graphql_schema, &*state).await;
	let json_res = serde_json::to_string(&res)?;
	let mut resp = if res.is_ok() { HttpResponse::Ok() } else { HttpResponse::BadRequest() };
	Ok(resp.content_type("application/json").body(json_res))
}

impl Void {
	fn new() -> Self { Self { void: true } }
}

#[juniper::graphql_object(Context = State)]
/// A previously visited server.
impl Bookmark {
	/// The internal id.
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }
	/// The name of the bookmark if it has a custom name.
	fn name(&self) -> Option<&str> { self.0.name.as_ref().map(|s| s.as_str()) }
	/// The name that was used to connect.
	fn username(&self) -> &str { &self.0.username }
	/// The server address.
	fn address(&self) -> &str { &self.0.address }

	async fn channel(&self, state: &State) -> GResult<Option<Channel>> {
		if let Some(server) = self.0.server.clone() {
			if let Some(id) = self.0.channel {
				let res = state
					.database
					.send(RunOnDbMsg(move |db| {
						use schema::channels;

						let query = channels::table
							.filter(channels::server.eq(server).and(channels::id.eq(id)));
						GResult::Ok(Channel(query.first::<models::Channel>(&db.con)?))
					}))
					.await??;
				Ok(Some(res))
			} else {
				Ok(None)
			}
		} else {
			Ok(None)
		}
	}

	/// The identity that is used with this bookmark.
	async fn identity(&self, state: &State) -> GResult<Identity> {
		let id = self.0.identity;
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::identities;

				let query = identities::table.filter(identities::id.eq(id));
				GResult::Ok(Identity(query.first::<models::Identity>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}

	/// `true` if it this object is saved as a bookmark or `false`, if it is a
	/// server that we recently connected to.
	fn bookmark(&self) -> bool { self.0.bookmark }
	/// The time when this bookmark was last used
	fn last_used(&self) -> Option<&NaiveDateTime> { self.0.last_used.as_ref() }
	fn timezone(&self) -> i32 { self.0.timezone }

	async fn server(&self, state: &State) -> GResult<Option<Server>> {
		if let Some(id) = self.0.server.clone() {
			let res = state
				.database
				.send(RunOnDbMsg(|db| {
					use schema::servers;

					let query = servers::table.filter(servers::public_key.eq(id));
					GResult::Ok(Server(query.first::<models::Server>(&db.con)?))
				}))
				.await??;
			Ok(Some(res))
		} else {
			Ok(None)
		}
	}
}

#[juniper::graphql_object(Context = State)]
impl Channel {
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }

	async fn server(&self, state: &State) -> GResult<Option<Server>> {
		let id = self.0.server.clone();
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				use schema::servers;

				let query = servers::table.filter(servers::public_key.eq(id));
				GResult::Ok(Server(query.first::<models::Server>(&db.con)?))
			}))
			.await??;
		Ok(Some(res))
	}
	fn parent(&self) -> Option<ID> { self.0.parent.map(|i| ID::new(i.to_string())) }
	/// References the channel above this one (zero if this is the first
	/// channel).
	fn order_id(&self) -> Option<ID> { self.0.order_id.map(|i| ID::new(i.to_string())) }
	fn name(&self) -> &str { &self.0.name }
	fn icon(&self) -> Option<ID> { self.0.icon.map(|i| ID::new((i as u32).to_string())) }
	fn deleted(&self) -> bool { self.0.deleted }

	/// The channel chat.
	async fn chat(&self, state: &State) -> GResult<Option<Chat>> {
		let server = self.0.server.clone();
		let id = self.0.id;
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{channel_chats, chats};

				let query = channel_chats::table
					.filter(channel_chats::server.eq(server).and(channel_chats::channel.eq(id)))
					.inner_join(chats::table)
					.select(chats::all_columns);
				GResult::Ok(query.first::<models::Chat>(&db.con).optional()?.map(Chat))
			}))
			.await??;
		Ok(res)
	}
}

#[juniper::graphql_object(Context = State)]
impl Chat {
	/// The internal id.
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }
	fn last_read(&self) -> &NaiveDateTime { &self.0.last_read }
	fn timezone(&self) -> i32 { self.0.timezone }

	/// Fetches 50 messages, older than the given start time and id.
	///
	/// If no start is given, the latest messages are returned.
	/// If `before_start` is `true`, get messages older than the start if it is
	/// `false`, get messages that were sent after the start.
	async fn messages(
		&self, state: &State, start_time: Option<NaiveDateTime>, start_id: Option<ID>,
		before_start: Option<bool>,
	) -> GResult<Vec<Message>>
	{
		let start_id = start_id.map(|i| i.parse::<u64>().map(|i| i as i64)).transpose()?;
		let start = match (start_time, start_id, before_start) {
			(Some(t), Some(i), Some(b)) => Some((t, i, b)),
			(None, None, None) => None,
			_ => {
				return Err(format_err!(
					"start_time, start_id and before_start need to be all set or unset"
				)
				.into());
			}
		};
		let id = self.0.id;
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::messages;

				let query = messages::table.filter(messages::chat.eq(id)).limit(MESSAGES_LIMIT);
				let res = if let Some((t, i, true)) = start {
					query
						.filter(messages::time.lt(t).and(messages::id.lt(i)))
						.order((messages::time.desc(), messages::id.desc()))
						.load::<models::Message>(&db.con)
						.map(|mut m| {
							m.reverse();
							m
						})
				} else if let Some((t, i, false)) = start {
					query
						.filter(messages::time.gt(t).and(messages::id.gt(i)))
						.order((messages::time, messages::id))
						.load::<models::Message>(&db.con)
				} else {
					query
						.order((messages::time.desc(), messages::id.desc()))
						.load::<models::Message>(&db.con)
						.map(|mut m| {
							m.reverse();
							m
						})
				};

				GResult::Ok(res?.into_iter().map(Message).collect())
			}))
			.await??;
		Ok(res)
	}

	/// How many messages in this chat were not yet read.
	async fn unread_count(&self, state: &State) -> GResult<i32> {
		let id = self.0.id;
		let res: i64 = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{chats, messages};

				let query = messages::table
					.inner_join(chats::table)
					.filter(chats::id.eq(id).and(messages::time.gt(chats::last_read)))
					.count();

				query.get_result(&db.con)
			}))
			.await??;
		Ok(res.try_into()?)
	}
}

#[juniper::graphql_object(Context = State)]
impl Client {
	/// The uid of the client.
	fn uid(&self) -> ID { ID::new(base64::encode(&self.0.uid)) }
	fn name(&self) -> &str { &self.0.name }
	/// The base64 encoded public key of the client if we have it.
	fn public_key(&self) -> Option<String> {
		self.0.public_key.as_ref().map(|p| base64::encode(&p))
	}
	/// The custom name of the client if we assigned one.
	fn custom_name(&self) -> Option<&str> { self.0.custom_name.as_ref().map(|s| s.as_str()) }
	fn volume(&self) -> f64 { self.0.volume as f64 }

	/// The chat with this client on the specified server.
	async fn chat(&self, state: &State, server: ID) -> GResult<Option<Chat>> {
		let server = base64::decode(server.as_bytes())?;
		let id = self.0.uid.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{chats, client_chats};

				let query = client_chats::table
					.filter(client_chats::server.eq(server).and(client_chats::client.eq(id)))
					.inner_join(chats::table)
					.select(chats::all_columns);
				GResult::Ok(query.first::<models::Chat>(&db.con).optional()?.map(Chat))
			}))
			.await??;
		Ok(res)
	}

	/// The chat with this client on the specified server.
	async fn pokes(&self, state: &State, server: ID) -> GResult<Option<Chat>> {
		let server = base64::decode(server.as_bytes())?;
		let id = self.0.uid.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{chats, client_pokes};

				let query = client_pokes::table
					.filter(client_pokes::server.eq(server).and(client_pokes::client.eq(id)))
					.inner_join(chats::table)
					.select(chats::all_columns);
				GResult::Ok(query.first::<models::Chat>(&db.con).optional()?.map(Chat))
			}))
			.await??;
		Ok(res)
	}
}

#[juniper::graphql_object(Context = State)]
impl Identity {
	/// The internal id.
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }
	fn name(&self) -> &str { &self.0.name }

	fn level(&self, state: &State) -> GResult<i32> {
		Ok(i32::from(self.0.clone().into_identity(&state.secret)?.level()?))
	}

	/// The publicly visible client associated with this identity.
	async fn client(&self, state: &State) -> GResult<Client> {
		let id = self.0.client.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::clients;

				let query = clients::table.filter(clients::uid.eq(id));
				GResult::Ok(Client(query.first::<models::Client>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}
}

#[juniper::graphql_object(Context = State)]
impl Message {
	/// The internal id.
	fn id(&self) -> ID { ID::new(self.0.id.to_string()) }

	async fn chat(&self, state: &State) -> GResult<Chat> {
		let id = self.0.chat;
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::chats;

				let query = chats::table.filter(chats::id.eq(id));
				GResult::Ok(Chat(query.first::<models::Chat>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}

	/// The send of the message or `None` if we got the message from the server.
	async fn invoker(&self, state: &State) -> GResult<Option<ServerClient>> {
		if let Some(id) = self.0.invoker.clone() {
			let chat = self.0.chat;
			let res = state
				.database
				.send(RunOnDbMsg(move |db| {
					use schema::{channel_chats, client_chats, server_chats, servers_clients};

					let query = servers_clients::table
						.filter(
							servers_clients::client.eq(id).and(
								servers_clients::server
									.eq_any(
										channel_chats::table
											.filter(channel_chats::chat.eq(&chat))
											.select(channel_chats::server),
									)
									.or(servers_clients::server.eq_any(
										client_chats::table
											.filter(client_chats::chat.eq(&chat))
											.select(client_chats::server),
									))
									.or(servers_clients::server.eq_any(
										server_chats::table
											.filter(server_chats::chat.eq(&chat))
											.select(server_chats::server),
									)),
							),
						)
						.select(servers_clients::all_columns);
					GResult::Ok(ServerClient(query.first::<models::ServersClients>(&db.con)?))
				}))
				.await??;
			Ok(Some(res))
		} else {
			Ok(None)
		}
	}

	/// Html of rendered markdown and bb code.
	fn rendered(&self) -> String { crate::markdown::markdown(&self.0.content) }

	/// Name of the invoker if we don't have their uid.
	fn invoker_name(&self) -> Option<&str> { self.0.invoker_name.as_ref().map(|s| s.as_str()) }
	fn content(&self) -> &str { &self.0.content }
	fn status(&self) -> MessageStatus { self.0.status }
	fn time(&self) -> &NaiveDateTime { &self.0.time }
	fn timezone(&self) -> i32 { self.0.timezone }
}

#[juniper::graphql_object(Context = State)]
impl Server {
	/// The public key of the server, base64 encoded.
	fn public_key(&self) -> ID { ID::new(base64::encode(&self.0.public_key)) }
	fn name(&self) -> &str { &self.0.name }
	/// The last used address to connect to this server.
	fn address(&self) -> &str { &self.0.address }
	fn icon(&self) -> Option<ID> { self.0.icon.map(|i| ID::new((i as u32).to_string())) }

	/// The server chat.
	async fn chat(&self, state: &State) -> GResult<Option<Chat>> {
		let id = self.0.public_key.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{chats, server_chats};

				let query = server_chats::table
					.filter(server_chats::server.eq(id))
					.inner_join(chats::table)
					.select(chats::all_columns);
				GResult::Ok(query.first::<models::Chat>(&db.con).optional()?.map(Chat))
			}))
			.await??;
		Ok(res)
	}

	/// The channels on this server.
	// TODO Pagination
	async fn channels(&self, state: &State, include_deleted: bool) -> GResult<Vec<Channel>> {
		let id = self.0.public_key.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::channels;

				let query = channels::table.filter(channels::server.eq(id));
				let res = if include_deleted {
					query.load::<models::Channel>(&db.con)
				} else {
					query.filter(channels::deleted.eq(false)).load::<models::Channel>(&db.con)
				};
				GResult::Ok(res?.into_iter().map(Channel).collect())
			}))
			.await??;
		Ok(res)
	}

	/// The clients that we saw on this server.
	// TODO Pagination
	async fn clients(&self, state: &State) -> GResult<Vec<ServerClient>> {
		let id = self.0.public_key.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::servers_clients;

				let query = servers_clients::table.filter(servers_clients::server.eq(id));
				GResult::Ok(
					query
						.load::<models::ServersClients>(&db.con)?
						.into_iter()
						.map(ServerClient)
						.collect(),
				)
			}))
			.await??;
		Ok(res)
	}
}

#[juniper::graphql_object(Context = State)]
impl ServerClient {
	async fn server(&self, state: &State) -> GResult<Server> {
		let id = self.0.server.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::servers;

				let query = servers::table.filter(servers::public_key.eq(id));
				GResult::Ok(Server(query.first::<models::Server>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}

	async fn client(&self, state: &State) -> GResult<Client> {
		let id = self.0.client.clone();
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::clients;

				let query = clients::table.filter(clients::uid.eq(id));
				GResult::Ok(Client(query.first::<models::Client>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}

	/// The icon of this client on this server.
	fn icon(&self) -> Option<ID> { self.0.icon.map(|i| ID::new((i as u32).to_string())) }
	/// The avatar of this client on this server.
	fn avatar(&self) -> Option<&str> { self.0.avatar.as_ref().map(|s| s.as_str()) }
	/// When we saw this client last on this server
	fn last_seen(&self) -> &NaiveDateTime { &self.0.last_seen }
	fn timezone(&self) -> i32 { self.0.timezone }
}

#[juniper::graphql_object(Context = State)]
impl Query {
	// TODO Support pagination: https://relay.dev/graphql/connections.htm
	async fn bookmarks(state: &State) -> GResult<Vec<Bookmark>> {
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				let query = bookmarks::table
					.order((bookmarks::bookmark.desc(), bookmarks::last_used.desc()))
					.limit(BOOKMARKS_LIMIT);
				let result = /*if let Some((book, last)) = msg.start {
				// (bookmark == book AND last_used > last) OR (!bookmark AND book)
				query
					.filter(
						bookmarks::bookmark
							.eq(book)
							.and(bookmarks::last_used.gt(Some(last.naive_utc()))),
					)
					.or_filter(not(bookmarks::bookmark).and(book))
					.load::<Bookmark>(&db.con)
				} else*/ {
					query.load::<models::Bookmark>(&db.con)
				}?.into_iter().map(Bookmark).collect();

				GResult::Ok(result)
			}))
			.await??;
		Ok(res)
	}

	/// The most recently used connection
	async fn most_recent_bookmark(state: &State) -> GResult<Option<Bookmark>> {
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				let query = bookmarks::table.order(bookmarks::last_used.desc());
				let result = query.first::<models::Bookmark>(&db.con).optional()?.map(Bookmark);

				GResult::Ok(result)
			}))
			.await??;
		Ok(res)
	}

	async fn chat(
		state: &State, typ: GMessageTarget, server: ID, id: Option<ID>,
	) -> GResult<Option<Chat>> {
		let server = base64::decode(server.as_bytes())?;
		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::{channel_chats, chats, client_chats, client_pokes, server_chats};

				let res = match typ {
					GMessageTarget::Server => {
						if id.is_some() {
							return Err(format_err!("Server message target needs no id").into());
						}
						let query = server_chats::table
							.filter(server_chats::server.eq(server))
							.inner_join(chats::table)
							.select(chats::all_columns);
						query.first::<models::Chat>(&db.con)
					}
					GMessageTarget::Channel => {
						let id = if let Some(id) = id {
							id.parse::<u64>()? as i64
						} else {
							return Err(format_err!("Channel message target needs id").into());
						};
						let query = channel_chats::table
							.filter(
								channel_chats::server.eq(server).and(channel_chats::channel.eq(id)),
							)
							.inner_join(chats::table)
							.select(chats::all_columns);
						query.first::<models::Chat>(&db.con)
					}
					GMessageTarget::Client => {
						let id = if let Some(id) = id {
							base64::decode(id.as_bytes())?
						} else {
							return Err(format_err!("Poke message target needs id").into());
						};
						let query = client_chats::table
							.filter(
								client_chats::server.eq(server).and(client_chats::client.eq(id)),
							)
							.inner_join(chats::table)
							.select(chats::all_columns);
						query.first::<models::Chat>(&db.con)
					}
					GMessageTarget::Poke => {
						let id = if let Some(id) = id {
							base64::decode(id.as_bytes())?
						} else {
							return Err(format_err!("Poke message target needs id").into());
						};
						let query = client_pokes::table
							.filter(
								client_pokes::server.eq(server).and(client_pokes::client.eq(id)),
							)
							.inner_join(chats::table)
							.select(chats::all_columns);
						query.first::<models::Chat>(&db.con)
					}
				};

				GResult::Ok(res.optional()?.map(Chat))
			}))
			.await??;
		Ok(res)
	}

	async fn server(state: &State, server: ID) -> GResult<Server> {
		let server = base64::decode(server.as_bytes())?;
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				use schema::servers;

				let query = servers::table.filter(servers::public_key.eq(server));
				GResult::Ok(Server(query.first::<models::Server>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}

	async fn client(state: &State, uid: ID) -> GResult<Client> {
		let client = base64::decode(uid.as_bytes())?;
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				use schema::clients;

				let query = clients::table.filter(clients::uid.eq(client));
				GResult::Ok(Client(query.first::<models::Client>(&db.con)?))
			}))
			.await??;
		Ok(res)
	}
}

#[juniper::graphql_object(Context = State)]
impl Mutation {
	async fn update_bookmark(state: &State, update: UpdateBookmark) -> GResult<Void> {
		let res = state
			.database
			.send(RunOnDbMsg(|db| {
				use schema::channels;

				let id: i64 = update.id.parse::<u64>()? as i64;

				let ch = if let Some(c) = update.channel {
					// Search server
					let server = bookmarks::table
						.filter(bookmarks::id.eq(id))
						.select(bookmarks::server)
						.first::<Option<Vec<u8>>>(&db.con)?;

					let server = if let Some(s) = server {
						s
					} else {
						Err(format_err!("Cannot set channel: Bookmark needs a server"))?
					};

					// Search channel
					let ch_id: i64 = c.parse::<u64>()? as i64;
					let res = channels::table
						.filter(channels::id.eq(ch_id).and(channels::server.eq(server)))
						.select(diesel::dsl::count_star())
						.execute(&db.con)?;
					if res == 0 {
						Err(format_err!("Cannot set channel: Does not exist"))?;
					}

					Some(ch_id)
				} else {
					None
				};

				let db_update = UpdateBookmarkDb {
					name: update.name,
					username: update.username,
					channel: ch,
					bookmark: update.bookmark,
				};

				let res = diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
					.set(db_update)
					.execute(&db.con)?;

				GResult::Ok(res)
			}))
			.await??;

		if res == 0 {
			Err(format_err!("Bookmark not found"))?;
		}

		Ok(Void::new())
	}

	/// Connection is the websocket uuid, client is the client uid.
	async fn set_client_volume(
		state: &State, connection: ID, client: ID, volume: f64,
	) -> GResult<Void> {
		let connection = connection.parse()?;
		let client = base64::decode(client.as_bytes())?;
		let uid = Uid(client.clone());
		let volume = volume as f32;

		let con;
		{
			let cons = state.connections.lock().unwrap();
			if let Some(c) = cons.get(&ConnectionId(connection)) {
				con = c.clone();
			} else {
				return Err(format_err!("Connection not found").into());
			}
		}
		let logger = state.logger.clone();
		actix::spawn(con.send(SetVolumeMsg(uid.clone(), volume)).map(move |r| {
			if let Err(e) = r {
				error!(logger, "Failed to set volume"; "error" => %e);
			}
		}));
		con.send(SaveClientMsg(uid)).await??;

		let res = state
			.database
			.send(RunOnDbMsg(move |db| {
				use schema::clients;

				let res = diesel::update(clients::table.filter(clients::uid.eq(&client)))
					.set(clients::volume.eq(volume))
					.execute(&db.con)?;

				GResult::Ok(res)
			}))
			.await??;

		if res == 0 {
			Err(format_err!("Client not found"))?;
		}

		Ok(Void::new())
	}
}

pub(crate) fn create_schema() -> Arc<Schema> {
	Arc::new(Schema::new(Query, Mutation, EmptySubscription::<State>::new()))
}
