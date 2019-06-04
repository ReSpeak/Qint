use diesel::prelude::*;
use diesel::connection::SimpleConnection;
use diesel::sqlite::SqliteConnection;
use failure::Error;
use slog::{info, Logger};

use crate::Settings;

mod models;
mod schema;

diesel_migrations::embed_migrations!();

pub(crate) fn connect(logger: &Logger, settings: &Settings) -> Result<SqliteConnection, Error> {
	let database_url = settings.config_path.join("storage.sqlite");
	let con = SqliteConnection::establish(database_url.to_str().unwrap())?;
	con.batch_execute("PRAGMA foreign_keys = ON")?;

	// Run migrations
	let mut s = Vec::new();
	embedded_migrations::run_with_output(&con, &mut s)?;
	let s = std::str::from_utf8(&s)?;
	if !s.is_empty() {
		info!(logger, "Run migrations"; "output" => s);
	}

	Ok(con)
}
