/// A general file cache for files from a TeamSpeak server.
///
/// This is used for caching icons and avatars for offline usage.
// Icons: There can be collisions as only CRC-32 is used, we may update
// them at some time when the modification time on the server is newer
// than on the cached file.
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use actix_web::web::{self, Bytes};
use futures::prelude::*;
use slog::{debug, error};
use tokio::fs;
use tokio::io::AsyncWrite;
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::{ChannelId, Uid};

use crate::State;

/// Files are stored in
/// `<cache_path>/files/<base32 of server uid>/<channel_id>/<base32 of path>`.
pub struct FileCache {}

/// A struct that writes into files.
///
/// When it is dropped and not the whole content was written, the file is
/// deleted.
struct FileWriter<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin> {
	stream: S,
	orig_buf: Option<Bytes>,
	buf: Option<Bytes>,
	file: Option<fs::File>,
	path: PathBuf,
	finished: bool,
}

impl FileCache {
	fn path_encode(data: &[u8]) -> String {
		base32::encode(base32::Alphabet::RFC4648 { padding: false }, data)
	}

	fn get_path(state: &State, server: Uid, channel: ChannelId, path: &str) -> PathBuf {
		let mut p = state.settings.cache_path.clone();
		p.push(Self::path_encode(&server.0));
		p.push(channel.0.to_string());
		p.push(Self::path_encode(path.as_bytes()));
		p
	}

	pub async fn cache_file(
		state: &State, server: Uid, channel: ChannelId, path: &str,
		file: impl Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
	) -> impl Stream<Item = Result<Bytes, std::io::Error>>
	{
		let path = Self::get_path(state, server, channel, path);
		if let Err(e) = fs::create_dir_all(&path.parent().unwrap()).await {
			error!(state.logger, "Failed to create cache directory";
				"error" => %e);
			return file.left_stream();
		}

		match fs::File::create(&path).await {
			Ok(r) => FileWriter::new(file, r, path).right_stream(),
			Err(e) => {
				error!(state.logger, "Failed to create cache file";
					"error" => %e);
				return file.left_stream();
			}
		}
	}

	/// Returns length and stream if the file is cached.
	pub async fn get_cached_file(
		state: &State, server: Uid, channel: ChannelId, path: &str,
	) -> Option<(u64, impl Stream<Item = Result<Bytes, std::io::Error>>)> {
		let path = Self::get_path(state, server, channel, path);
		let meta = match fs::metadata(&path).await {
			Ok(r) => r,
			Err(e) => {
				debug!(state.logger, "File not in cache"; " path" => ?path,
					"error" => %e);
				return None;
			}
		};
		match fs::File::open(&path).await {
			Err(e) => {
				error!(state.logger, "Failed to open cached file";
					"error" => %e);
				None
			}
			Ok(file) => {
				let stream =
					FramedRead::new(file, BytesCodec::new()).map(|r| r.map(web::BytesMut::freeze));
				Some((meta.len(), stream))
			}
		}
	}
}

impl<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin> FileWriter<S> {
	fn new(stream: S, file: fs::File, path: PathBuf) -> Self {
		Self {
			stream,
			file: Some(file),
			orig_buf: Default::default(),
			buf: Default::default(),
			path,
			finished: false,
		}
	}
}

impl<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin> Drop for FileWriter<S> {
	fn drop(&mut self) {
		if !self.finished {
			self.file = None;
			let _ = std::fs::remove_file(&self.path);
		}
	}
}

impl<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin> Stream for FileWriter<S> {
	type Item = Result<Bytes, std::io::Error>;
	fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
		loop {
			let this = &mut *self;
			while let Some(buf) = &mut this.buf {
				if buf.is_empty() {
					this.buf = None;
					break;
				}
				match Pin::new(&mut this.file.as_mut().unwrap()).poll_write_buf(cx, buf) {
					Poll::Pending => return Poll::Pending,
					Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
					Poll::Ready(Ok(_)) => {}
				}
			}

			if let Some(buf) = self.orig_buf.take() {
				return Poll::Ready(Some(Ok(buf)));
			}

			match self.stream.poll_next_unpin(cx) {
				Poll::Pending => return Poll::Pending,
				Poll::Ready(Some(Err(e))) => return Poll::Ready(Some(Err(e))),
				Poll::Ready(Some(Ok(buf))) => {
					self.orig_buf = Some(buf.clone());
					self.buf = Some(buf);
				}
				Poll::Ready(None) => {
					self.finished = true;
					return Poll::Ready(None);
				}
			}
		}
	}
}
