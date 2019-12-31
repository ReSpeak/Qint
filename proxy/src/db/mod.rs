use std::fs;

use actix::*;
use diesel::prelude::*;
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use failure::Error;
use slog::{info, Logger};
use tsclientlib::Identity;

use crate::secret::Secret;
use crate::Settings;
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

impl Actor for DbHandler {
	type Context = Context<Self>;
}

impl Message for GetIdentityMsg { type Result = Result<Identity, Error>; }
impl Message for EventMsg { type Result = Result<(), Error>; }

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
			}
			EventMsg::Connected(addr, con) => {
				use schema::servers::dsl::*;

				let key = con.get_server_key()?;
				let key = key.to_short();
				let con = con.lock();

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
