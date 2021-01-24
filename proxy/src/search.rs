use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{format_err, Error};
use diesel::prelude::*;
use futures::FutureExt;
use meilisearch_core::settings::{SettingsUpdate, UpdateState};
use meilisearch_core::store::Index;
use meilisearch_core::Error as MError;
use meilisearch_core::{Database, DatabaseOptions, Schema};
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, trace, warn, Logger};
use tsclientlib::Uid;
use tsproto_types::crypto::EccKeyPubP256;

use crate::{db, Result, State};

const MESSAGES_INDEX: &str = "messages";
/// Add documents in batches when creating the database.
const INIT_BATCH_SIZE: usize = 1000;
const CHANNEL_PREFIX: &str = "ch";
const CLIENT_PREFIX: &str = "cl";
const MESSAGES_PREFIX: &str = "m";
const SERVER_PREFIX: &str = "s";

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub enum SearchResultId {
	Channel { server: Vec<u8>, id: u64 },
	Client { id: Vec<u8> },
	Message { id: u64 },
	Server { id: Vec<u8> },
}

pub struct Search {
	logger: Logger,
	database: Database,
	index: Index,
}

#[derive(Clone, Debug)]
pub struct SearchResult {
	pub id: SearchResultId,
	/// Maps attribute name to highlights for this field.
	pub highlights: HashMap<&'static str, Vec<Range<usize>>>,
}

#[derive(Clone, Debug)]
pub struct SearchResults {
	pub results: Vec<SearchResult>,
	pub count: usize,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct ChannelDocument {
	/// Needs to be "<prefix><database u64 id>"
	pub id: String,
	pub name: String,
	pub topic: Option<String>,
	pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct ClientDocument {
	pub id: String,
	pub uid: String,
	pub name: String,
	pub phonetic_name: Option<String>,
	pub custom_name: Option<String>,
	pub custom_phonetic_name: Option<String>,
	pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct MessageDocument {
	pub id: String,
	pub content: String,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct ServerDocument {
	pub id: String,
	pub uid: String,
	pub address: String,
	pub name: String,
	pub host_message: Option<String>,
	pub welcome_message: Option<String>,
}

/// The highlight ranges of meilisearch are not with respect to the real source string, so try to
/// guess the right range.
pub fn meili_to_byte_range(index: usize, length: usize, text: &str) -> Range<usize> {
	let mut start = index;
	let mut end = index + length;

	if start > text.len() {
		start = text.len();
	}
	if end > text.len() {
		end = text.len();
	}

	// Round down to char boundaries
	while !text.is_char_boundary(start) {
		start -= 1;
	}
	while !text.is_char_boundary(end) {
		end -= 1;
	}

	Range { start, end }
}

impl FromStr for SearchResultId {
	type Err = Error;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		if s.starts_with(MESSAGES_PREFIX) {
			Ok(SearchResultId::Message { id: s[1..].parse()? })
		} else if s.starts_with(CHANNEL_PREFIX) {
			let split =
				s.find('_').ok_or_else(|| format_err!("No '_' found in channel search id"))?;
			let channel = &s[2..split];
			let server = &s[split + 1..];
			Ok(SearchResultId::Channel {
				server: base64::decode_config(server, base64::URL_SAFE_NO_PAD)?,
				id: channel.parse()?,
			})
		} else if s.starts_with(CLIENT_PREFIX) {
			Ok(SearchResultId::Client {
				id: base64::decode_config(&s[2..], base64::URL_SAFE_NO_PAD)?,
			})
		} else if s.starts_with(SERVER_PREFIX) {
			Ok(SearchResultId::Server {
				id: base64::decode_config(&s[1..], base64::URL_SAFE_NO_PAD)?,
			})
		} else {
			Err(format_err!("Unknown search result id type {:?}", s))
		}
	}
}

impl Search {
	/// Creates a search databe and returns if it was newly created or not.
	pub fn new(logger: Logger, path: &Path) -> Result<(Self, bool)> {
		let database = match Database::open_or_create(path, DatabaseOptions::default()) {
			Ok(r) => r,
			Err(MError::VersionMismatch(msg)) => {
				info!(logger, "Search database version mismatch, recreating database";
					"old_version" => msg, "path" => ?path);
				std::fs::remove_dir_all(path)?;
				Database::open_or_create(path, DatabaseOptions::default())?
			}
			Err(e) => return Err(e.into()),
		};
		let (index, is_new) = match database.open_index(MESSAGES_INDEX) {
			Some(index) => (index, false),
			None => {
				let schema = Schema::with_primary_key("id");
				let index = database.create_index(MESSAGES_INDEX)?;
				database.main_write(|w| index.main.put_schema(w, &schema))?;
				let settings = SettingsUpdate {
					primary_key: UpdateState::Update("id".into()),
					..Default::default()
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

		Ok((Self { logger, database, index }, is_new))
	}

	/// Setup default database settings.
	pub fn start_setup(state: &Arc<State>) {
		let logger = state.logger.clone();
		let state2 = state.clone();
		actix::spawn(
			state
				.database
				.send(db::RunOnDbMsg(move |db| -> Result<()> {
					let search = &state2.search;
					// TODO Use some stop words and synonyms
					//search.database.update_write(|w| search.index.settings_update(w, settings))?;

					// Fetch all messages from the database
					let mut offset = 0;
					loop {
						use db::schema::messages;

						let query = messages::table
							.select((messages::id, messages::content))
							.order(messages::id)
							.offset(offset)
							.limit(INIT_BATCH_SIZE as i64);
						let res = query.load::<(i64, String)>(&db.con)?;
						let len = res.len();

						// Insert into search database
						let mut additions = search.index.documents_addition();
						for r in res {
							let doc = MessageDocument {
								id: format!("{}{}", MESSAGES_PREFIX, r.0 as u64),
								content: r.1,
							};
							additions.update_document(doc);
						}
						search.database.update_write(|w| additions.finalize(w))?;

						debug!(state2.logger, "Writing messages into search db";
							"count" => offset as usize + len);
						if len < INIT_BATCH_SIZE {
							break;
						}
						offset += INIT_BATCH_SIZE as i64;
					}

					// Fetch all channels from the database
					offset = 0;
					loop {
						use db::schema::channels;

						let query = channels::table
							.select((channels::server, channels::id, channels::name))
							.order(channels::id)
							.offset(offset)
							.limit(INIT_BATCH_SIZE as i64);
						let res = query.load::<(Vec<u8>, i64, String)>(&db.con)?;
						let len = res.len();

						// Insert into search database
						let mut additions = search.index.documents_addition();
						for r in res {
							let server = base64::encode_config(&r.0, base64::URL_SAFE_NO_PAD);
							let doc = ChannelDocument {
								id: format!("{}{}_{}", CHANNEL_PREFIX, r.1 as u64, server),
								name: r.2,
								topic: None,
								description: None,
							};
							additions.update_document(doc);
						}
						search.database.update_write(|w| additions.finalize(w))?;

						debug!(state2.logger, "Writing channels into search db";
							"count" => offset as usize + len);
						if len < INIT_BATCH_SIZE {
							break;
						}
						offset += INIT_BATCH_SIZE as i64;
					}

					// Fetch all clients from the database
					offset = 0;
					loop {
						use db::schema::clients;

						let query = clients::table
							.select((
								clients::uid,
								clients::name,
								clients::custom_name.nullable(),
								clients::custom_phonetic_name.nullable(),
							))
							.order(clients::uid)
							.offset(offset)
							.limit(INIT_BATCH_SIZE as i64);
						let res = query
							.load::<(Vec<u8>, String, Option<String>, Option<String>)>(&db.con)?;
						let len = res.len();

						// Insert into search database
						let mut additions = search.index.documents_addition();
						for r in res {
							let uid = base64::encode_config(&r.0, base64::URL_SAFE_NO_PAD);
							let doc = ClientDocument {
								id: format!("{}{}", CLIENT_PREFIX, uid),
								uid: uid,
								name: r.1,
								phonetic_name: None,
								custom_name: r.2,
								custom_phonetic_name: r.3,
								description: None,
							};
							additions.update_document(doc);
						}
						search.database.update_write(|w| additions.finalize(w))?;

						debug!(state2.logger, "Writing clients into search db";
							"count" => offset as usize + len);
						if len < INIT_BATCH_SIZE {
							break;
						}
						offset += INIT_BATCH_SIZE as i64;
					}

					// Fetch all servers from the database
					offset = 0;
					loop {
						use db::schema::servers;

						let query = servers::table
							.select((servers::public_key, servers::address, servers::name))
							.order(servers::public_key)
							.offset(offset)
							.limit(INIT_BATCH_SIZE as i64);
						let res = query.load::<(Vec<u8>, String, String)>(&db.con)?;
						let len = res.len();

						// Insert into search database
						let mut additions = search.index.documents_addition();
						for r in res {
							let str_key = base64::encode_config(&r.0, base64::URL_SAFE_NO_PAD);
							let public_key = EccKeyPubP256::from_short(r.0);
							let uid = public_key.get_uid()?;
							let doc = ServerDocument {
								id: format!("{}{}", SERVER_PREFIX, str_key),
								uid: uid,
								address: r.1,
								name: r.2,
								host_message: None,
								welcome_message: None,
							};
							additions.update_document(doc);
						}
						search.database.update_write(|w| additions.finalize(w))?;

						debug!(state2.logger, "Writing servers into search db";
							"count" => offset as usize + len);
						if len < INIT_BATCH_SIZE {
							break;
						}
						offset += INIT_BATCH_SIZE as i64;
					}

					Ok(())
				}))
				.map(move |r| {
					if let Err(e) = r {
						warn!(logger, "Failed to setup search database"; "error" => %e);
					} else if let Ok(Err(e)) = r {
						warn!(logger, "Failed to fill search database"; "error" => %e);
					}
				}),
		);
	}

	pub fn add_channel(
		&self, server: EccKeyPubP256, id: u64, name: String, topic: Option<String>,
		description: Option<String>,
	) -> Result<u64> {
		let server = base64::encode_config(server.to_short(), base64::URL_SAFE_NO_PAD);
		let mut additions = self.index.documents_addition();
		additions.update_document(ChannelDocument {
			id: format!("{}{}_{}", CHANNEL_PREFIX, id, server),
			name,
			topic,
			description,
		});
		let update_id = self.database.update_write(|w| additions.finalize(w))?;
		Ok(update_id)
	}

	pub fn add_client(
		&self, uid: &Uid, name: String, phonetic_name: Option<String>, custom_name: Option<String>,
		custom_phonetic_name: Option<String>, description: Option<String>,
	) -> Result<u64> {
		let uid = base64::encode_config(&uid.0, base64::URL_SAFE_NO_PAD);
		let mut additions = self.index.documents_addition();
		additions.update_document(ClientDocument {
			id: format!("{}{}", CLIENT_PREFIX, uid),
			uid,
			name,
			phonetic_name,
			custom_name,
			custom_phonetic_name,
			description,
		});
		let update_id = self.database.update_write(|w| additions.finalize(w))?;
		Ok(update_id)
	}

	pub fn add_message(&self, id: u64, content: String) -> Result<u64> {
		let mut additions = self.index.documents_addition();
		additions
			.update_document(MessageDocument { id: format!("{}{}", MESSAGES_PREFIX, id), content });
		let update_id = self.database.update_write(|w| additions.finalize(w))?;
		Ok(update_id)
	}

	pub fn add_server(
		&self, public_key: EccKeyPubP256, address: String, name: String,
		host_message: Option<String>, welcome_message: Option<String>,
	) -> Result<u64> {
		let uid = public_key.get_uid()?;
		let server = base64::encode_config(public_key.to_short(), base64::URL_SAFE_NO_PAD);
		let mut additions = self.index.documents_addition();
		additions.update_document(ServerDocument {
			id: format!("{}{}", SERVER_PREFIX, server),
			uid,
			address,
			name,
			host_message,
			welcome_message,
		});
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

		let schema =
			self.index.main.schema(&reader)?.ok_or_else(|| format_err!("Schema not found"))?;
		let mut attr_map = HashMap::<u16, &'static str>::new();
		// All attributes of search structs except id
		for a in &[
			"name",
			"topic",
			"description",
			"uid",
			"phonetic_name",
			"custom_name",
			"custom_phonetic_name",
			"content",
			"address",
			"name",
			"host_message",
			"welcome_message",
		] {
			// Attributes do not exist if there is no entry using it
			if let Some(attr) = schema.id(a) {
				attr_map.insert(attr.0, a);
			} else {
				debug!(self.logger, "Attribute not found in search schema"; "attribute" => a);
			}
		}

		let mut res = SearchResults { results: Vec::new(), count: result.nb_hits };
		for r in result.documents {
			#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
			struct IdDocument {
				id: String,
			}

			if let Some(id) = self.index.document::<IdDocument>(&reader, Some(&attrs), r.id)? {
				let id: SearchResultId = id.id.parse()?;
				let mut highlights = HashMap::new();

				for h in &r.highlights {
					if let Some(attr) = attr_map.get(&h.attribute) {
						let hs = highlights.entry(*attr).or_insert_with(Vec::new);
						hs.push(h.char_index as usize..h.char_length as usize);
					} else if h.attribute != 0 {
						// 0 is id
						warn!(self.logger, "Unknown attribute in search"; "attr" => h.attribute);
					}
				}

				if !highlights.is_empty() {
					// Only add if there are highlights in known attributes (ignore id)
					res.results.push(SearchResult { id, highlights });
				}
			} else {
				warn!(self.logger, "Search document not found"; "id" => ?r.id);
			}
		}

		trace!(self.logger, "Search query result"; "result" => ?res);
		Ok(res)
	}
}
