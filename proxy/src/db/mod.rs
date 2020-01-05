use std::fs;

use actix::*;
use actix_web::*;
use chrono::{DateTime, Local, Utc};
use chrono::offset::{FixedOffset, TimeZone};
use diesel::prelude::*;
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use failure::Error;
use qint_shared::models::Bookmark;
use rmp_serde::Serializer;
use serde::Serialize;
use slog::{info, Logger};
use tsclientlib::Identity;
use tsproto::crypto::EccKeyPubP256;

use crate::secret::Secret;
use crate::{Settings, State};
use event_handler::EventHandler;

mod models;
mod event_handler;
mod schema;

diesel_migrations::embed_migrations!();

pub struct DbHandler {
	logger: Logger,
	secret: Secret,
	con: SqliteConnection,
}

/// Identity id, `true` will create a new identity if this id does not exist.
#[derive(Clone, Debug)]
pub struct GetIdentityMsg(pub u64, pub bool);

#[derive(Clone)]
pub enum EventMsg {
	Events(tsclientlib::Connection, Vec<tsclientlib::events::Event>),
	/// The connection and the address that was used.
	Connected(String, tsclientlib::Connection),
	UpdateIdentity(Identity),
}

pub struct ConnectedMsg {
	pub bookmark: Option<i64>,
	pub username: String,
	pub address: String,
	pub channel: Option<i64>,
	pub identity: i64,
	pub server_key: EccKeyPubP256,
}

struct GetBookmarksMsg {
	/// The start for paging.
	/// (bookmark, last_used) have to be greater than this.
	start: Option<(bool, DateTime<Utc>)>,
}

#[get("/bookmarks")]
pub(crate) async fn bookmarks(state: web::Data<State>) -> Result<HttpResponse, Error> {
	let msg = GetBookmarksMsg {
		start: None,
	};
	let bookmarks = state.database.send(msg).await??;
	let mut buf = Vec::new();
	let mut ser = Serializer::new(&mut buf);
	bookmarks.serialize(&mut ser).unwrap();

	Ok(HttpResponse::Ok().body(buf))
}

impl Actor for DbHandler {
	type Context = Context<Self>;
}

impl Message for GetIdentityMsg { type Result = Result<Identity, Error>; }
impl Message for EventMsg { type Result = Result<(), Error>; }
impl Message for GetBookmarksMsg { type Result = Result<Vec<Bookmark>, Error>; }
impl Message for ConnectedMsg { type Result = Result<(), Error>; }

impl DbHandler {
	pub(crate) fn new(logger: Logger, settings: &Settings, secret: Secret) -> Result<Self, Error> {
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

		Ok(Self {
			logger,
			secret,
			con,
		})
	}
}

impl Handler<GetIdentityMsg> for DbHandler {
	type Result = Result<Identity, Error>;
	fn handle(&mut self, msg: GetIdentityMsg, _: &mut Self::Context) -> Self::Result {
		use schema::identities::dsl::*;

		match identities.find(msg.0 as i64).first::<models::Identity>(&self.con) {
			Ok(r) => r.into_identity(&self.secret),
			Err(_) => {
				// Pick an existing identity if one exists
				if let Ok(r) = identities.order_by(id).first::<models::Identity>(&self.con) {
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
				diesel::insert_into(schema::clients::table)
					.values(&cli)
					.execute(&self.con)?;

				let new_identity = models::NewIdentity::new(&identity,
					&uid, &self.secret)?;
				diesel::insert_into(identities)
					.values(&new_identity)
					.execute(&self.con)?;

				Ok(identity)
			}
		}
	}
}

impl Handler<EventMsg> for DbHandler {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: EventMsg, _: &mut Self::Context) -> Self::Result {
		match msg {
			EventMsg::Events(con, es) => {
				self.handle_events(&con, &es)?;
				// TODO On channellistfinished: Mark channels as removed which
				// are no longer there
			}
			EventMsg::Connected(addr, con) => {
				use schema::servers::dsl::*;

				let key = con.get_server_key()?;
				let key = key.to_short();
				let con = con.lock();
				let icon_id = if con.server.icon_id.0 != 0 {
					Some(con.server.icon_id.0 as i32)
				} else {
					None
				};

				// Check if we already know that address
				if diesel::select(diesel::dsl::exists(servers.filter(
					public_key.eq(&key)))).get_result(&self.con)? {
					// Update
					diesel::update(servers.filter(public_key.eq(&key)))
						.set((
							name.eq(&con.server.name),
							address.eq(&addr),
						))
						.execute(&self.con)?;
				} else {
					let server = models::ServerInsert {
						public_key: &key,
						name: &con.server.name,
						address: &addr,
						icon: icon_id,
					};
					diesel::insert_into(schema::servers::table)
						.values(&server)
						.execute(&self.con)?;
				}
			}
			EventMsg::UpdateIdentity(identity) => {
				use schema::identities::dsl::*;

				let pub_key = identity.key().to_pub();
				let uid = pub_key.get_uid_no_base64()?;
				diesel::update(identities.filter(client.eq(uid)))
					.set((
						counter.eq(identity.counter() as i64),
						max_counter.eq(identity.max_counter() as i64),
					))
					.execute(&self.con)?;
			}
		}
		Ok(())
	}
}

impl Handler<GetBookmarksMsg> for DbHandler {
	type Result = Result<Vec<Bookmark>, Error>;
	fn handle(&mut self, msg: GetBookmarksMsg, _: &mut Self::Context) -> Self::Result {
		use diesel::dsl::not;
		use schema::{bookmarks, channels, servers};

		// Order by (bookmark, last_used)
		// Select id, name, address, bookmark, last_used, timezone
		// Join channel.name
		// Join server.icon

		let query = bookmarks::table
			.left_outer_join(servers::table)
			.left_outer_join(channels::table.on(
					bookmarks::server.eq(channels::server.nullable())
					.and(bookmarks::channel.eq(channels::id.nullable()))))
			.order((bookmarks::bookmark, bookmarks::last_used))
			.limit(20)
			.select((bookmarks::id, bookmarks::name, bookmarks::username,
				bookmarks::address, bookmarks::bookmark, bookmarks::last_used,
				bookmarks::timezone, channels::name.nullable(),
				servers::icon.nullable()));
		let result = if let Some((book, last)) = msg.start {
			// (bookmark == book AND last_used > last) OR (!bookmark AND book)
			query.filter(bookmarks::bookmark.eq(book)
					.and(bookmarks::last_used.gt(Some(last.naive_utc()))))
				.or_filter(not(bookmarks::bookmark).and(book))
				.load::<Bookmark>(&self.con)
		} else {
			query.load::<Bookmark>(&self.con)
		}?;

		Ok(result)
	}
}

impl Handler<ConnectedMsg> for DbHandler {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: ConnectedMsg, _: &mut Self::Context) -> Self::Result {
		use diesel::dsl::not;
		use schema::{bookmarks, identities};
		let server = msg.server_key.to_short();

		// Find identity
		let identity = match identities::table.find(msg.identity as i64)
			.select(identities::id).first::<i64>(&self.con) {
			Ok(r) => r,
			Err(_) => {
				// Pick an existing identity
				identities::table.order_by(identities::id).select(identities::id)
					.first::<i64>(&self.con)?
			}
		};

		let utc_time = Utc::now().naive_utc();
		let dummy_offset = FixedOffset::east(0);
		let local_zone = Local::from_offset(&dummy_offset);
		let utc_to_local_offset = local_zone.offset_from_utc_datetime(&utc_time).local_minus_utc();

		// Compare channel
		// https://stackoverflow.com/questions/10416789/how-to-rewrite-is-distinct-from-and-is-not-distinct-from
		// a IS NOT DISTINCT FROM b can be rewritten as:
		// (NOT (a <> b OR a IS NULL OR b IS NULL) OR (a IS NULL AND b IS NULL))
		let cmp = not(bookmarks::channel.ne(msg.channel).or(bookmarks::channel.is_null()).or(msg.channel.is_none()))
			.or(bookmarks::channel.is_null().and(msg.channel.is_none()));

		// Check if we already know that address
		let id = msg.bookmark.map(Ok).or_else(|| {
			bookmarks::table.filter(cmp
				.and(bookmarks::address.eq(&msg.address))
				.and(bookmarks::identity.eq(identity))
				.and(bookmarks::server.eq(&server)))
				.select(bookmarks::id)
				.first::<i64>(&self.con)
				.optional()
				.transpose()
		}).transpose()?;
		if let Some(id) = id {
			// Update
			diesel::update(bookmarks::table.filter(bookmarks::id.eq(id)))
				.set((
					bookmarks::username.eq(&msg.username),
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
			diesel::insert_into(bookmarks::table)
				.values(&bookmark)
				.execute(&self.con)?;
		}
		Ok(())
	}
}
