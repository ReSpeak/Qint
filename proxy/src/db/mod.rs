use actix_web::actix::*;
use diesel::prelude::*;
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use failure::Error;
use slog::{info, Logger};
use tsclientlib::Identity;

use crate::Settings;

mod models;
mod schema;

diesel_migrations::embed_migrations!();

pub struct DbHandler {
	logger: Logger,
	con: SqliteConnection,
}

/// Identity id, `true` will create a new identity if this id does not exist.
#[derive(Clone, Debug)]
pub struct GetIdentityMsg(pub u64, pub bool);

impl Actor for DbHandler {
	type Context = Context<Self>;
}

impl Message for GetIdentityMsg { type Result = Result<Identity, Error>; }

impl DbHandler {
	pub(crate) fn new(logger: Logger, settings: &Settings) -> Result<Self, Error> {
		let database_url = settings.config_path.join("storage.sqlite");
		let con = SqliteConnection::establish(database_url.to_str().unwrap())?;
		// Enable foreign keys
		con.batch_execute("PRAGMA foreign_keys = ON")?;

		// Run migrations
		let mut s = Vec::new();
		embedded_migrations::run_with_output(&con, &mut s)?;
		let s = std::str::from_utf8(&s)?;
		if !s.is_empty() {
			info!(logger, "Run migrations"; "output" => s);
		}

		Ok(Self {
			logger,
			con,
		})
	}
}

impl Handler<GetIdentityMsg> for DbHandler {
	type Result = Result<Identity, Error>;
	fn handle(&mut self, msg: GetIdentityMsg, _: &mut Self::Context) -> Self::Result {
		use schema::identities::dsl::*;

		match identities.find(msg.0 as i64).first::<models::Identity>(&self.con) {
			Ok(r) => r.into_identity(vec![]),
			Err(_) => {
				// Create new identity
				panic!()
			}
		}
	}
}
