/// A general file cache for files from a TeamSpeak server.
///
/// This is used for caching icons and avatars for offline usage.
// Icons: There can be collisions as only CRC-32 is used, we may update
// them at some time when the modification time on the server is newer
// than on the cached file.
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Buf, Bytes, BytesMut};
use anyhow::Result;
use futures::prelude::*;
use futures::stream::Peekable;
use slog::{debug, error, Logger};
use tokio::fs;
use tokio::io::AsyncWrite;
use tokio_util::codec::{BytesCodec, FramedRead};
use tsclientlib::ChannelId;
use tsproto_types::crypto::EccKeyPubP256;

/// Files are stored in
/// `<cache_path>/files/<base64 of server uid>/<channel_id>/<base64 of path>`.
pub struct FileCache {
	logger: Logger,
	cache_path: PathBuf,
}

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
	pub fn new(logger: Logger, cache_path: PathBuf) -> Self {
		Self { logger, cache_path }
	}

	fn path_encode(data: &[u8]) -> String {
		base64::encode_config(data, base64::URL_SAFE_NO_PAD)
	}

	fn get_path(&self, server: &EccKeyPubP256, channel: ChannelId, path: &str) -> PathBuf {
		let mut p = self.cache_path.clone();
		p.push(Self::path_encode(&server.get_uid_no_base64()));
		p.push(channel.0.to_string());
		p.push(Self::path_encode(path.as_bytes()));
		p
	}

	pub async fn cache_file(
		&self, server: &EccKeyPubP256, channel: ChannelId, path: &str,
		file: impl Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
	) -> impl Stream<Item = Result<Bytes, std::io::Error>> {
		let path = self.get_path(server, channel, path);
		if let Err(e) = fs::create_dir_all(&path.parent().unwrap()).await {
			error!(self.logger, "Failed to create cache directory"; "error" => %e);
			return file.left_stream();
		}

		match fs::File::create(&path).await {
			Ok(r) => FileWriter::new(file, r, path).right_stream(),
			Err(e) => {
				error!(self.logger, "Failed to create cache file"; "error" => %e);
				file.left_stream()
			}
		}
	}

	/// Deletes a file from the cache.
	///
	/// Returns `true` if a file was deleted or `false` if it did not exist.
	pub fn delete_file(
		&self, server: &EccKeyPubP256, channel: ChannelId, path: &str,
	) -> Result<bool> {
		let path = self.get_path(server, channel, path);
		if !path.exists() {
			return Ok(false);
		}
		Ok(std::fs::remove_file(path).map(|()| true)?)
	}

	/// Returns length and stream if the file is cached.
	pub async fn get_cached_file(
		&self, server: &EccKeyPubP256, channel: ChannelId, path: &str,
	) -> Option<(u64, impl Stream<Item = Result<Bytes, std::io::Error>>)> {
		let path = self.get_path(server, channel, path);
		let meta = match fs::metadata(&path).await {
			Ok(r) => r,
			Err(e) => {
				debug!(self.logger, "File not in cache"; " path" => ?path, "error" => %e);
				return None;
			}
		};
		match fs::File::open(&path).await {
			Err(e) => {
				error!(self.logger, "Failed to open cached file"; "error" => %e);
				None
			}
			Ok(file) => {
				let stream =
					FramedRead::new(file, BytesCodec::new()).map(|r| r.map(BytesMut::freeze));
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
				match Pin::new(this.file.as_mut().unwrap()).poll_write(cx, buf) {
					Poll::Pending => return Poll::Pending,
					Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
					Poll::Ready(Ok(n)) => buf.advance(n),
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

pub async fn guess_content_type<S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin + 'static>(
	stream: S,
) -> (Peekable<S>, Option<&'static str>) {
	let mut stream = stream.peekable();
	let mime = if let Some(Ok(r)) = Pin::new(&mut stream).peek().await {
		// https://en.wikipedia.org/wiki/List_of_file_signatures
		if r.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
			Some("image/png")
		} else if r.starts_with(&[0xFF, 0xD8, 0xFF, 0xDB])
			|| r.starts_with(&[0xFF, 0xD8, 0xFF, 0xE0])
			|| r.starts_with(&[0xFF, 0xD8, 0xFF, 0xEE])
		{
			Some("image/jpeg")
		} else if r.windows(3).any(|w| w == b"svg") {
			Some("image/svg+xml")
		} else if r.starts_with(&[0x42, 0x4D]) {
			Some("image/bmp")
		} else if r.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x37, 0x61])
			|| r.starts_with(&[0x47, 0x49, 0x46, 0x38, 0x39, 0x61])
		{
			Some("image/gif")
		} else if r
			.starts_with(&[0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70, 0x69, 0x73, 0x6F, 0x6D])
		{
			Some("video/mp4")
		} else {
			None
		}
	} else {
		None
	};
	(stream, mime)
}
