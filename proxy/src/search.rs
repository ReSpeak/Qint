use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use anyhow::format_err;
use diesel::prelude::*;
use futures::FutureExt;
use meilisearch_core::{Database, DatabaseOptions, Schema};
use meilisearch_core::settings::{SettingsUpdate, UpdateState};
use meilisearch_core::store::Index;
use serde::{Deserialize, Serialize};
use slog::{debug, error, trace, warn, Logger};

use crate::{db, Result, State};

const MESSAGES_INDEX: &str = "messages";
/// Add documents in batches when creating the database.
const INIT_BATCH_SIZE: usize = 1000;

pub struct Search {
	logger: Logger,
	database: Database,
	index: Index,
}

#[derive(Clone, Debug, Hash)]
pub struct SearchResult {
	pub id: String,
	pub num_id: u64,
	pub author_highlights: Vec<Range<usize>>,
	pub content_highlights: Vec<Range<usize>>,
}

#[derive(Clone, Debug, Hash)]
pub struct SearchResults {
	pub results: Vec<SearchResult>,
	pub count: usize,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct MessageDocument {
	/// Needs to be "m<database u64 id>"
	pub id: String,
	pub author: String,
	pub content: String,
}

pub fn char_to_byte_range(index: usize, length: usize, text: &str) -> Range<usize> {
	let mut byte_index = 0;
	let mut byte_length = 0;

	for (n, (i, c)) in text.char_indices().enumerate() {
		if n == index {
			byte_index = i;
		}

		if n + 1 == index + length {
			byte_length = i - byte_index + c.len_utf8();
			break;
		}
	}

	Range { start: byte_index, end: byte_index + byte_length }
}

impl Search {
	/// Creates a search databe and returns if it was newly created or not.
	pub fn new(logger: Logger, path: &Path) -> Result<(Self, bool)> {
		let database = Database::open_or_create(path, DatabaseOptions::default())?;
		let (index, is_new) = match database.open_index(MESSAGES_INDEX) {
			Some(index) => (index, false),
			None => {
				let schema = Schema::with_primary_key("id");
				let index = database.create_index(MESSAGES_INDEX)?;
				database.main_write(|w| index.main.put_schema(w, &schema))?;
				let settings = SettingsUpdate {
					primary_key: UpdateState::Update("id".into()),
					.. Default::default()
				};
				database.update_write(|w| index.settings_update(w, settings))?;
				(index, true)
			}
		};

		let logger2 = logger.clone();
		database.set_update_callback(Box::new(move |_name, res| {
			if let Some(e) = res.error {
				error!(logger2, "Search db update failed"; "error" => e,
					"code" => ?res.error_code, "link" => ?res.error_link);
			}
		}));

		Ok((Self {
			logger,
			database,
			index,
		}, is_new))
	}

	/// Setup default database settings.
	pub fn start_setup(state: &Arc<State>) {
		let logger = state.logger.clone();
		let state2 = state.clone();
		actix::spawn(state.database.send(db::RunOnDbMsg(move |db| -> Result<()> {
			use db::schema::{clients, messages};

			let search = &state2.search;
			// TODO Use some stop words and synonyms
			//search.database.update_write(|w| search.index.settings_update(w, settings))?;

			// Fetch all messages from the database
			let mut offset = 0;
			loop {
				let query = messages::table
					.left_outer_join(clients::table)
					.select((messages::id, clients::name.nullable(), messages::invoker_name, messages::content))
					.order(messages::id)
					.offset(offset)
					.limit(INIT_BATCH_SIZE as i64);
				let res = query.load::<(i64, Option<String>, Option<String>, String)>(&db.con)?;
				let len = res.len();

				// Insert into search database
				let mut additions = search.index.documents_addition();
				for r in res {
					if let Some(author) = r.1.or(r.2) {
						let doc = MessageDocument {
							id: format!("m{}",  r.0 as u64),
							author,
							content: r.3,
						};
						additions.update_document(doc);
					} else {
						warn!(state2.logger, "Neither invoker nor invoker_name are set for message";
							"id" => r.0 as u64);
					}
				}
				search.database.update_write(|w| additions.finalize(w))?;

				debug!(state2.logger, "Writing messages into search db";
					"count" => offset as usize + len);
				if len < INIT_BATCH_SIZE {
					break;
				}
				offset += INIT_BATCH_SIZE as i64;
			}

			Ok(())
		})).map(move |r| if let Err(e) = r {
			warn!(logger, "Failed to setup search database"; "error" => %e);
		} else if let Ok(Err(e)) = r {
			warn!(logger, "Failed to fill search database"; "error" => %e);
		}));
	}

	pub fn add_message(&self, msg: MessageDocument) -> Result<u64> {
		let mut additions = self.index.documents_addition();
		additions.update_document(msg);
		let update_id = self.database.update_write(|w| additions.finalize(w))?;
		Ok(update_id)
	}

	pub fn search(&self, query: &str, range: Range<usize>) -> Result<SearchResults> {
		let reader = self.database.main_read_txn()?;
		let builder = self.index.query_builder();
		trace!(self.logger, "Search query"; "query" => query, "range" => ?range);
		let result = builder.query(&reader, Some(query), range)?;
		let mut attrs = HashSet::new();
		attrs.insert("id");

		let schema = self.index.main.schema(&reader)?
			.ok_or_else(|| format_err!("Schema not found"))?;
		let content_attr = schema.id("content")
			.ok_or_else(|| format_err!("Content not found in schema"))?.0;

		let mut res = SearchResults {
			results: Vec::new(),
			count: result.nb_hits,
		};
		for r in result.documents {
			#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
			struct IdDocument {
				id: String,
			}

			if let Some(id) = self.index.document::<IdDocument>(&reader, Some(&attrs), r.id)? {
				let num_id = id.id[1..].parse::<u64>()?;
				let author_highlights = Vec::new();
				let mut content_highlights = Vec::new();

				for h in &r.highlights {
					if h.attribute == content_attr {
						//content_highlights.push(h.char_index as usize..h.char_index as usize + h.char_length as usize);
						content_highlights.push(h.char_index as usize..h.char_length as usize);
					}
				}

				res.results.push(SearchResult {
					id: id.id,
					num_id,
					author_highlights,
					content_highlights,
				});
			} else {
				warn!(self.logger, "Search document not found"; "id" => ?r.id);
			}
		}

		trace!(self.logger, "Search query result"; "result" => ?res);
		Ok(res)
	}
}
