use std::io::{self, ErrorKind};
use std::ops::Range;
use std::path::Path;
use std::pin::Pin;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::task::{Context, Poll};

use anyhow::{Error, bail};
use futures::channel::mpsc::{Receiver, Sender, channel};
use futures::stream::StreamExt;
use qint_proxy::connection::{DownloadFileContext, UploadFileContext};
use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};

pub struct FiletransferManager {
	sender: Mutex<Sender<TransferAction>>,
	receiver: Mutex<Option<Receiver<TransferAction>>>,
	transfer_counter: AtomicU32,
}

pub enum TransferAction {
	Download(DownloadFileContext, DownloadPrepare),
	Upload(UploadFileContext, UploadPrepare),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferId(u32);

struct TransferContext {
	#[allow(unused)]
	id: TransferId,
	done: bool,
	#[allow(unused)]
	kind: TxDirection,
	src_stream: Pin<Box<dyn AsyncRead>>,
	dst_stream: Pin<Box<dyn AsyncWrite>>,
	transfer_target: u64,
	transfer_position: u64,
	buffer: Box<[u8]>,
	buffer_size: Range<usize>,
	//last_measurement: Option<Instant, u64> // measure timestamp, transfer_position
}

pub struct UploadPrepare {
	id: TransferId,
	file: Pin<Box<dyn AsyncRead + Send>>,
	size: u64,
}

pub struct DownloadPrepare {
	id: TransferId,
	file: Pin<Box<dyn AsyncWrite + Send>>,
}

enum TxTickStatus {
	Finished,
	Progress,
	Waiting,
}

#[derive(Debug, PartialEq, Eq)]
enum TxDirection {
	Download,
	Upload,
}

impl UploadPrepare {
	pub fn get_size(&self) -> u64 { self.size }
}

type FiletransferList = Vec<TransferContext>;

impl FiletransferManager {
	pub fn new() -> Self {
		let (sender, receiver) = channel(8);
		Self {
			sender: Mutex::new(sender),
			receiver: Mutex::new(Some(receiver)),
			transfer_counter: AtomicU32::new(0),
		}
	}

	pub fn next_transfer_id(&self) -> TransferId {
		TransferId(self.transfer_counter.fetch_add(1, Ordering::Relaxed))
	}

	pub fn add_download(&self, ctx: DownloadFileContext, download_prep: DownloadPrepare) {
		let mut sender = self.sender.lock().unwrap();
		sender.try_send(TransferAction::Download(ctx, download_prep)).unwrap();
	}

	pub async fn prepare_download(&self, local_path: &Path) -> Result<DownloadPrepare, Error> {
		let file_result = if local_path.exists() {
			println!("Opening existing file for resume");
			File::open(&local_path).await
		} else {
			println!("Crating new file to dl");
			File::create(&local_path).await
		};

		let local_stream = match file_result {
			Ok(local_stream) => local_stream,
			Err(err) => bail!("Failed to open file to write: {:?}", err),
		};

		// TODO RESUMING !!!
		// let new_pos = local_stream.seek(SeekFrom::Start(transfer_position)).await.unwrap();
		// if transfer_position != new_pos {
		// 	bail!("Failed to seek");
		// }

		Ok(DownloadPrepare { id: self.next_transfer_id(), file: Box::pin(local_stream) })
	}

	pub fn add_upload(&self, ctx: UploadFileContext, upload_prep: UploadPrepare) {
		let mut sender = self.sender.lock().unwrap();
		sender.try_send(TransferAction::Upload(ctx, upload_prep)).unwrap();
	}

	pub async fn prepare_upload(&self, local_path: &Path) -> Result<UploadPrepare, Error> {
		let file_result = if local_path.exists() {
			println!("Opening existing file for upload");
			File::open(&local_path).await
		} else {
			bail!("Local file not found");
		};

		let local_stream = match file_result {
			Ok(local_stream) => local_stream,
			Err(err) => bail!("Failed to open file to read: {:?}", err),
		};

		let meta = local_stream.metadata().await?;
		let size = meta.len();

		Ok(UploadPrepare { id: self.next_transfer_id(), file: Box::pin(local_stream), size })
	}

	pub fn prepare_upload_from_bytes(&self, data: Vec<u8>) -> Result<UploadPrepare, Error> {
		let size = data.len() as u64;
		Ok(UploadPrepare {
			id: self.next_transfer_id(),
			file: Box::pin(AsyncBuffer(data, 0)),
			size,
		})
	}

	pub async fn transfer_loop(&self) {
		let mut transfer_list = FiletransferList::new();
		let mut receiver = {
			let mut receiver = self.receiver.lock().unwrap();
			receiver.take().expect("transfer_loop started twice")
		};

		loop {
			if transfer_list.len() == 0 {
				if let Some(action) = receiver.next().await {
					Self::handle_action(&mut transfer_list, action);
				} else {
					println!("FileTransfer channel closed");
					return;
				}
			} else {
				Self::transfer_all_ticks(&mut transfer_list).await;
			}

			while let Ok(action) = receiver.try_recv() {
				Self::handle_action(&mut transfer_list, action);
			}
		}
	}

	fn handle_action(list: &mut FiletransferList, action: TransferAction) {
		match action {
			TransferAction::Download(
				DownloadFileContext { stream, size },
				DownloadPrepare { id, file },
			) => {
				list.push(TransferContext::new(
					id,
					TxDirection::Download,
					Box::pin(stream),
					file,
					size,
				));
			}
			TransferAction::Upload(
				UploadFileContext { stream },
				UploadPrepare { id, file, size },
			) => {
				list.push(TransferContext::new(
					id,
					TxDirection::Upload,
					file,
					Box::pin(stream),
					size,
				));
			}
		}
	}

	async fn transfer_all_ticks(list: &mut FiletransferList) {
		for ftctx in list.iter_mut() {
			match ftctx.transfer_tick().await {
				Err(err) => {
					println!("error ft {:?}", err);
				}
				Ok(TxTickStatus::Finished) => {
					ftctx.done = true;
					// TODO nothify, i thought wrote this todo already somewhere (?)
				}
				Ok(TxTickStatus::Progress) | Ok(TxTickStatus::Waiting) => {
					// nothing
				}
			}
		}

		list.retain(|ftctx| !ftctx.done);
	}
}

impl TransferContext {
	pub fn new(
		id: TransferId, kind: TxDirection, src_stream: Pin<Box<dyn AsyncRead>>,
		dst_stream: Pin<Box<dyn AsyncWrite>>, transfer_target: u64,
	) -> Self {
		Self {
			id,
			done: false,
			kind,
			src_stream,
			dst_stream,
			transfer_target,
			transfer_position: 0,
			buffer: Box::new([0u8; 1 << 16]),
			buffer_size: 0..0,
		}
	}

	pub async fn transfer_tick(&mut self) -> Result<TxTickStatus, Error> {
		if self.buffer_size.len() == 0 {
			match self.src_stream.read(&mut self.buffer).await {
				Ok(n) => {
					self.buffer_size = 0..n;
				}
				Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
					return Ok(TxTickStatus::Waiting);
				}
				Err(e) => return Err(e.into()),
			}
		}

		if self.buffer_size.len() > 0 {
			match self.dst_stream.write(&self.buffer[self.buffer_size.clone()]).await {
				Ok(written) => {
					self.buffer_size = (self.buffer_size.start + written)..self.buffer_size.end;
					self.transfer_position += written as u64;

					if self.transfer_position == self.transfer_target {
						Ok(TxTickStatus::Finished)
					} else {
						Ok(TxTickStatus::Progress)
					}
				}
				Err(ref e) if e.kind() == ErrorKind::WouldBlock => Ok(TxTickStatus::Waiting),
				Err(e) => Err(e.into()),
			}
		} else {
			Ok(TxTickStatus::Waiting)
		}
	}
}

struct AsyncBuffer(Vec<u8>, usize);

impl AsyncRead for AsyncBuffer {
	fn poll_read(
		mut self: Pin<&mut Self>, _cx: &mut Context<'_>, buf: &mut ReadBuf<'_>,
	) -> Poll<io::Result<()>> {
		let remaining_len = self.0.len() - self.1;
		let amt = std::cmp::min(remaining_len, buf.remaining());
		let read_to = self.1 + amt;
		buf.put_slice(&self.0[self.1..read_to]);
		self.1 = read_to;
		Poll::Ready(Ok(()))
	}
}
