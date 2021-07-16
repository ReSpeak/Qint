use std::ops::Range;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use chrono::NaiveDateTime;
use diesel::prelude::*;
use futures::FutureExt;
use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::{FromPrimitive, ToPrimitive};
use serde::{Deserialize, Serialize};
use slog::{debug, error, info, warn, Logger};
use tantivy::schema::Schema;
use tantivy::{Document, Index, IndexReader, IndexWriter, ReloadPolicy, SnippetGenerator};
use tsclientlib::Uid;
use tsproto_types::crypto::EccKeyPubP256;

use crate::{db, QintState, Result};

/// Add documents in batches when creating the database.
const INIT_BATCH_SIZE: usize = 1000;

#[derive(
	Clone, Copy, Debug, Deserialize, Eq, FromPrimitive, Hash, PartialEq, Serialize, ToPrimitive,
)]
enum IndexEntryType {
	Message,
	Channel,
	Client,
	Server,
}

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub enum SearchResultId {
	Channel { server: Vec<u8>, id: u64 },
	Client(Vec<u8>),
	Message(u64),
	Server(Vec<u8>),
}

pub struct Search {
	logger: Logger,
	index: Index,
	schema: Schema,
	reader: IndexReader,
	writer: Arc<Mutex<Option<IndexWriter>>>,
	/// If a commit is curently scheduled for execution.
	will_commit: Arc<AtomicUsize>,
}

pub struct SearchResults {
	pub results: Vec<SearchResultId>,
	/// To generate highlights for message content
	pub content_snippet_generator: SnippetGenerator,
	pub name_snippet_generator: SnippetGenerator,
	pub address_snippet_generator: SnippetGenerator,
}

impl Search {
	/// Creates a search databe and returns if it was newly created or not.
	pub fn new(logger: Logger, path: &Path) -> Result<(Self, bool)> {
		let schema = Self::schema();
		let mut index = None;
		let mut is_new = false;
		if let Ok(dir) = tantivy::directory::MmapDirectory::open(path) {
			if Index::exists(&dir).unwrap_or_default() {
				let i = Index::open_in_dir(path)?;
				// Check if the schema matches the current schema
				if i.schema() == schema {
					index = Some(i);
				}
			}
		}

		let index = index.map(Ok).unwrap_or_else(|| {
			// Create new index
			is_new = true;
			let _ = std::fs::create_dir_all(path);
			Index::create_in_dir(path, schema.clone())
		})?;

		let reader = index.reader_builder().reload_policy(ReloadPolicy::OnCommit).try_into()?;

		Ok((
			Self {
				logger,
				index,
				schema,
				reader,
				writer: Default::default(),
				will_commit: Default::default(),
			},
			is_new,
		))
	}

	fn schema() -> Schema {
		use tantivy::schema::*;

		let mut schema_builder = Schema::builder();
		// IndexEntryType
		schema_builder.add_u64_field("type", STORED | INDEXED);
		schema_builder.add_u64_field("message_id", STORED);
		schema_builder.add_u64_field("channel_id", STORED);
		schema_builder.add_bytes_field("server_key", STORED);
		schema_builder.add_bytes_field("client_uid", STORED);

		// Server address
		schema_builder.add_text_field("address", TEXT);
		// Message content
		schema_builder.add_text_field("content", TEXT);
		// Channel/Client description
		schema_builder.add_text_field("description", TEXT);
		schema_builder.add_text_field("name", TEXT);
		// Message time
		schema_builder.add_u64_field("time", FAST);
		// Channel topic
		schema_builder.add_text_field("topic", TEXT);
		// Server/Client uid
		schema_builder.add_text_field("uid", STRING);

		schema_builder.build()
	}

	/// Setup default database settings.
	pub fn start_setup(state: &Arc<QintState>) {
		let logger = state.logger.clone();
		let state2 = state.clone();
		let search = if let Some(search) = state.search.clone() {
			search
		} else {
			error!(state.logger, "Cannot setup database if it is not connected");
			return;
		};
		actix::spawn(
			state
				.database
				.send(db::RunOnDbMsg(move |db| -> Result<()> {
					info!(state2.logger, "Setup search db");
					// TODO Use some stop words and synonyms
					//search.database.update_write(|w| search.index.settings_update(w, settings))?;

					{
						if let Some(mut writer) = search.writer.lock().unwrap().take() {
							writer.commit()?;
						}
					}

					// Use 50 MB for indexing
					let mut index_writer = search.index.writer(50_000_000)?;

					let typ = search.schema.get_field("type").unwrap();
					let message_id = search.schema.get_field("message_id").unwrap();
					let channel_id = search.schema.get_field("channel_id").unwrap();
					let server_key = search.schema.get_field("server_key").unwrap();
					let client_uid = search.schema.get_field("client_uid").unwrap();

					let address = search.schema.get_field("address").unwrap();
					let content = search.schema.get_field("content").unwrap();
					//let description = search.schema.get_field("description").unwrap();
					let name = search.schema.get_field("name").unwrap();
					let time = search.schema.get_field("time").unwrap();
					//let topic = search.schema.get_field("topic").unwrap();
					let uid_field = search.schema.get_field("uid").unwrap();

					// Fetch all messages from the database
					let mut offset = 0;
					loop {
						use db::schema::messages;

						let query = messages::table
							.select((messages::id, messages::time, messages::content))
							.order(messages::id)
							.offset(offset)
							.limit(INIT_BATCH_SIZE as i64);
						let res = query.load::<(i64, NaiveDateTime, String)>(&db.con)?;
						let len = res.len();

						// Insert into search database
						for r in res {
							let mut doc = Document::default();
							doc.add_u64(typ, IndexEntryType::Message.to_u64().unwrap());
							doc.add_u64(message_id, r.0 as u64);
							doc.add_u64(time, r.1.timestamp() as u64);
							doc.add_text(content, r.2);

							index_writer.add_document(doc);
						}

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
						for r in res {
							let mut doc = Document::default();
							doc.add_u64(typ, IndexEntryType::Channel.to_u64().unwrap());
							doc.add_u64(channel_id, r.1 as u64);
							doc.add_bytes(server_key, r.0);
							doc.add_text(name, r.2);

							index_writer.add_document(doc);
						}

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
						for r in res {
							let uid = base64::encode_config(&r.0, base64::URL_SAFE_NO_PAD);
							let mut doc = Document::default();
							doc.add_u64(typ, IndexEntryType::Client.to_u64().unwrap());
							doc.add_bytes(client_uid, r.0);
							doc.add_text(uid_field, uid);
							doc.add_text(name, r.1);

							index_writer.add_document(doc);
						}

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
						for r in res {
							let uid = EccKeyPubP256::from_short(&r.0)?.get_uid();
							let mut doc = Document::default();
							doc.add_u64(typ, IndexEntryType::Server.to_u64().unwrap());
							doc.add_bytes(server_key, r.0);
							doc.add_text(uid_field, uid);
							doc.add_text(name, r.2);
							doc.add_text(address, r.1);

							index_writer.add_document(doc);
						}

						debug!(state2.logger, "Writing servers into search db";
							"count" => offset as usize + len);
						if len < INIT_BATCH_SIZE {
							break;
						}
						offset += INIT_BATCH_SIZE as i64;
					}

					index_writer.commit()?;

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

	fn write(&self, doc: Document) {
		// This potentially blocks for a while, so start a new thread.
		let index = self.index.clone();
		let writer = self.writer.clone();
		let logger = self.logger.clone();
		let will_commit = self.will_commit.clone();
		std::thread::spawn(move || {
			will_commit.fetch_add(1, Ordering::Relaxed);
			let mut w = writer.lock().unwrap();
			let w2 = if let Some(w) = &mut *w {
				w
			} else {
				let writer = match index.writer_with_num_threads(1, 3_000_000) {
					Ok(r) => r,
					Err(e) => {
						error!(logger, "Failed to create search database writer"; "error" => %e);
						return;
					}
				};
				*w = Some(writer);
				w.as_mut().unwrap()
			};
			w2.add_document(doc);
			drop(w);
			std::thread::sleep(std::time::Duration::from_secs(1));
			if will_commit.fetch_add(1, Ordering::Relaxed) == 0 {
				// This thread is the last, so commit and free the writer
				let mut w = writer.lock().unwrap();
				if let Some(mut w) = w.take() {
					if let Err(e) = w.commit() {
						error!(logger, "Failed to commit to search database"; "error" => %e);
					}
				}
			}
		});
	}

	pub fn add_channel(
		&self, server: EccKeyPubP256, id: u64, name: String, topic: Option<String>,
		description: Option<String>,
	) -> Result<()> {
		let mut doc = Document::default();
		doc.add_u64(
			self.schema.get_field("type").unwrap(),
			IndexEntryType::Channel.to_u64().unwrap(),
		);
		doc.add_u64(self.schema.get_field("channel_id").unwrap(), id);
		doc.add_bytes(self.schema.get_field("server_key").unwrap(), server.to_short());
		doc.add_text(self.schema.get_field("name").unwrap(), name);
		if let Some(s) = description {
			doc.add_text(self.schema.get_field("description").unwrap(), s);
		}
		if let Some(s) = topic {
			doc.add_text(self.schema.get_field("topic").unwrap(), s);
		}

		self.write(doc);
		Ok(())
	}

	pub fn add_client(
		&self, uid: &Uid, name: String, _phonetic_name: Option<String>,
		_custom_name: Option<String>, _custom_phonetic_name: Option<String>,
		description: Option<String>,
	) -> Result<()> {
		let mut doc = Document::default();
		doc.add_u64(
			self.schema.get_field("type").unwrap(),
			IndexEntryType::Client.to_u64().unwrap(),
		);
		doc.add_bytes(self.schema.get_field("client_uid").unwrap(), &uid.0);
		doc.add_text(self.schema.get_field("uid").unwrap(), uid);
		doc.add_text(self.schema.get_field("name").unwrap(), name);
		if let Some(s) = description {
			doc.add_text(self.schema.get_field("description").unwrap(), s);
		}

		self.write(doc);
		Ok(())
	}

	pub fn add_message(&self, id: u64, time: NaiveDateTime, content: String) -> Result<()> {
		let mut doc = Document::default();
		doc.add_u64(
			self.schema.get_field("type").unwrap(),
			IndexEntryType::Message.to_u64().unwrap(),
		);
		doc.add_u64(self.schema.get_field("message_id").unwrap(), id);
		doc.add_u64(self.schema.get_field("time").unwrap(), time.timestamp() as u64);
		doc.add_text(self.schema.get_field("content").unwrap(), content);

		self.write(doc);
		Ok(())
	}

	pub fn add_server(
		&self, public_key: EccKeyPubP256, address: String, name: String,
		_host_message: Option<String>, _welcome_message: Option<String>,
	) -> Result<()> {
		let uid = public_key.get_uid();
		let mut doc = Document::default();
		doc.add_u64(
			self.schema.get_field("type").unwrap(),
			IndexEntryType::Server.to_u64().unwrap(),
		);
		doc.add_bytes(self.schema.get_field("server_key").unwrap(), public_key.to_short());
		doc.add_text(self.schema.get_field("uid").unwrap(), uid);
		doc.add_text(self.schema.get_field("name").unwrap(), name);
		doc.add_text(self.schema.get_field("address").unwrap(), address);

		self.write(doc);
		Ok(())
	}

	/// Returns a list of message ids.
	pub fn search(
		&self, query: &str, range: Range<usize>, messages: bool,
	) -> Result<SearchResults> {
		use tantivy::collector::TopDocs;
		use tantivy::query::*;
		use tantivy::schema::{IndexRecordOption, Term, Value};

		debug!(self.logger, "Search"; "query" => query, "messages" => ?messages, "range" => ?range);
		let mut time_reporter = slog_perf::TimeReporter::new_with_level(
			"Search",
			self.logger.clone(),
			slog::Level::Debug,
		);
		time_reporter.start("");

		let typ = self.schema.get_field("type").unwrap();
		let channel_id = self.schema.get_field("channel_id").unwrap();
		let client_uid = self.schema.get_field("client_uid").unwrap();
		let message_id = self.schema.get_field("message_id").unwrap();
		let server_key = self.schema.get_field("server_key").unwrap();

		let address = self.schema.get_field("address").unwrap();
		let content = self.schema.get_field("content").unwrap();
		let description = self.schema.get_field("description").unwrap();
		let name = self.schema.get_field("name").unwrap();
		let time = self.schema.get_field("time").unwrap();
		let topic = self.schema.get_field("topic").unwrap();
		let uid = self.schema.get_field("uid").unwrap();

		let searcher = self.reader.searcher();
		let query_parser = QueryParser::for_index(&self.index, vec![
			address,
			content,
			description,
			name,
			topic,
			uid,
		]);
		let query = query_parser.parse_query(query)?;
		// TODO Use prefix FuzzyTermQueries
		// Build cross product of fields and items in the query

		let query: Box<dyn Query> = if messages {
			Box::new(BooleanQuery::new(vec![
				(
					Occur::Must,
					Box::new(TermQuery::new(
						Term::from_field_u64(typ, IndexEntryType::Message.to_u64().unwrap()),
						IndexRecordOption::Basic,
					)),
				),
				(Occur::Must, query),
			]))
		} else {
			Box::new(BooleanQuery::new(vec![
				(
					Occur::MustNot,
					Box::new(TermQuery::new(
						Term::from_field_u64(typ, IndexEntryType::Message.to_u64().unwrap()),
						IndexRecordOption::Basic,
					)),
				),
				(Occur::Must, query),
			]))
		};

		let top_docs = searcher.search(
			&query,
			&TopDocs::with_limit(range.end - range.start)
			.and_offset(range.start)
			// Sort descreasing by time
			.order_by_u64_field(time),
		)?;

		let mut res = Vec::new();
		for (_score, doc_address) in top_docs {
			let doc = searcher.doc(doc_address)?;
			let t = doc.get_first(typ).and_then(|v| {
				if let Value::U64(v) = v { IndexEntryType::from_u64(*v) } else { None }
			});
			let t = if let Some(t) = t {
				t
			} else {
				warn!(self.logger, "Search found entry without type");
				continue;
			};

			match t {
				IndexEntryType::Channel => {
					if let (Some(Value::U64(id)), Some(Value::Bytes(server))) =
						(doc.get_first(channel_id), doc.get_first(server_key))
					{
						res.push(SearchResultId::Channel { server: server.clone(), id: *id });
					} else {
						warn!(self.logger, "Search found entry without id"; "type" => ?t);
					}
				}
				IndexEntryType::Client => {
					if let Some(Value::Bytes(id)) = doc.get_first(client_uid) {
						res.push(SearchResultId::Client(id.clone()));
					} else {
						warn!(self.logger, "Search found entry without id"; "type" => ?t);
					}
				}
				IndexEntryType::Message => {
					if let Some(Value::U64(id)) = doc.get_first(message_id) {
						res.push(SearchResultId::Message(*id));
					} else {
						warn!(self.logger, "Search found entry without id"; "type" => ?t);
					}
				}
				IndexEntryType::Server => {
					if let Some(Value::Bytes(id)) = doc.get_first(server_key) {
						res.push(SearchResultId::Server(id.clone()));
					} else {
						warn!(self.logger, "Search found entry without id"; "type" => ?t);
					}
				}
			}
		}

		let mut content_snippet_generator = SnippetGenerator::create(&searcher, &query, content)?;
		content_snippet_generator.set_max_num_chars(5000);
		let mut name_snippet_generator = SnippetGenerator::create(&searcher, &query, name)?;
		name_snippet_generator.set_max_num_chars(1000);
		let mut address_snippet_generator = SnippetGenerator::create(&searcher, &query, address)?;
		address_snippet_generator.set_max_num_chars(1000);

		time_reporter.finish();

		Ok(SearchResults {
			results: res,
			content_snippet_generator,
			name_snippet_generator,
			address_snippet_generator,
		})
	}
}
