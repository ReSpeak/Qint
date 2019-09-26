//! Cache downloaded files.
//!
//! Cache avatars and icons.
//! The avatars are stored per server and client.
//! Each pair of server- and client-uid has one avatar assigned.
//!
//! The icons are also stored per server, with the names the server assigns to
//! them.
//! There can be collisions as only CRC-32 is used, we may update them at some
//! time when the modification time on the server is newer than on the cached
//! file.

use std::path::{Path, PathBuf};

use actix_web::actix::*;
use failure::Error;
use futures01::Future;
use tokio::fs::{self, File};
use tokio::net::TcpStream;
use tsclientlib::Uid;

pub(crate) struct FileCache {
	cache_path: PathBuf,
	// TODO Lock single files?
}

pub enum CachedFile {
	Avatar {
		server: Uid,
		client: Uid,
	},
	Icon {
		server: Uid,
		name: String,
	},
}

pub struct GetFile(CachedFile);
pub struct AddFile(CachedFile, TcpStream);

impl Actor for FileCache {
	type Context = Context<Self>;
}

impl Message for GetFile { type Result = Result<File, Error>; }
// TODO Could return a copied stream here
impl Message for AddFile { type Result = Result<(), Error>; }

impl FileCache {
	pub(crate) fn new(cache_path: PathBuf) -> Result<Self, Error> {
		std::fs::create_dir_all(&cache_path)?;

		Ok(Self {
			cache_path,
		})
	}
}

impl CachedFile {
	fn get_path(&self, root: &Path) -> PathBuf {
		match self {
			CachedFile::Avatar { server, client } =>
				root.join("servers")
					.join(Self::uid_to_path(server))
					.join("avatars")
					.join(Self::uid_to_path(client)),
			CachedFile::Icon { server, name } =>
				root.join("servers")
					.join(Self::uid_to_path(server))
					.join("icons")
					.join(name),
		}
	}

	fn uid_to_path(uid: &Uid) -> String {
		hex::encode(&uid.0)
	}
}

impl Handler<GetFile> for FileCache {
	type Result = Box<dyn Future<Item=File, Error=Error>>;
	fn handle(&mut self, msg: GetFile, _: &mut Self::Context) -> Self::Result {
		let path = msg.0.get_path(&self.cache_path);
		Box::new(File::open(path).from_err())
	}
}

impl Handler<AddFile> for FileCache {
	type Result = Box<dyn Future<Item=(), Error=Error>>;
	fn handle(&mut self, msg: AddFile, _: &mut Self::Context) -> Self::Result {
		let path = msg.0.get_path(&self.cache_path);
		let parent = path.parent().unwrap().to_path_buf();
		Box::new(fs::create_dir_all(parent)
			.and_then(|_| {
				File::create(path)
			})
			.and_then(|f| tokio::io::copy(msg.1, f))
			.map(|_| ())
			.from_err())
	}
}
