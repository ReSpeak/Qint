use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use actix_web::actix::*;
use failure::{format_err, Error};
use futures01::{Future, Sink};
use futures_spawn::SpawnHelper;
use futures_threadpool::ThreadPool;
use parking_lot::Mutex;
use slog::{debug, error, o, Logger};
use tsproto_packets::packets::{AudioData, CodecType, OutAudio};

use super::*;

pub struct SetListenerMsg {
	pub connection: tsclientlib::Connection,
}

pub struct RemoveListenerMsg;
pub struct SetVolumeMsg(pub f64);
pub struct SetPlayingMsg(pub bool);

pub struct AudioToTsSdl {
	listeners: Arc<Mutex<Vec<ConnectionSinkCreator>>>,

	logger: Logger,
	volume: Option<()>,
	is_playing: Arc<AtomicBool>,
}

struct ConnectionSinkCreator {
	con: tsproto::client::ClientConVal,
}

impl Actor for AudioToTsSdl {
	type Context = Context<Self>;
}

impl Message for SetListenerMsg { type Result = (); }

impl Message for RemoveListenerMsg {
	/// `true` if there was a listener registered before, `false` if not.
	type Result = bool;
}
impl Message for SetVolumeMsg { type Result = (); }
impl Message for SetPlayingMsg { type Result = Result<(), Error>; }

impl Handler<SetListenerMsg> for AudioToTsSdl {
	type Result = ();
	fn handle(&mut self, msg: SetListenerMsg, _: &mut Self::Context) -> Self::Result {
		let mut listeners = self.listeners.lock();
		*listeners = vec![ConnectionSinkCreator {
			con: msg.connection.get_tsproto_connection(),
		}];
	}
}

impl Handler<RemoveListenerMsg> for AudioToTsSdl {
	type Result = bool;
	fn handle(&mut self, _: RemoveListenerMsg, _: &mut Self::Context) -> Self::Result {
		let mut ls = self.listeners.lock();
		let res = !ls.is_empty();
		ls.clear();
		if let Err(e) = self.set_playing(false) {
			error!(self.logger, "Failed to stop playing"; "error" => ?e);
		}
		res
	}
}

impl Handler<SetVolumeMsg> for AudioToTsSdl {
	type Result = ();
	fn handle(&mut self, msg: SetVolumeMsg, _: &mut Self::Context) -> Self::Result {
		self.set_volume(msg.0);
	}
}

impl Handler<SetPlayingMsg> for AudioToTsSdl {
	type Result = Result<(), Error>;
	fn handle(&mut self, msg: SetPlayingMsg, _: &mut Self::Context) -> Self::Result {
		self.set_playing(msg.0)
	}
}

impl AudioToTsSdl {
	pub fn new(
		logger: Logger,
		executor: ThreadPool,
		uri: Option<&str>,
	) -> Result<Self, Error> {
		let logger = logger.new(o!("pipeline" => "audio-to-ts"));
		// Put everything into audio-to-ts bin and play/pause that
		
		let listeners = Arc::new(Mutex::new(Vec::<ConnectionSinkCreator>::new()));

		let logger2 = logger.clone();
		let executor2 = executor.clone();
		let listeners2 = listeners.clone();
		let is_playing = Arc::new(AtomicBool::new(false));

		Ok(Self {
			listeners,
			volume: Option::None,
			logger: logger2,
			is_playing,
		})
	}

	pub fn set_volume(&self, volume: f64) {
		if let Some(v) = &self.volume {
			
		}
		// TODO
	}

	pub fn set_playing(&self, playing: bool) -> Result<(), Error> {
		Ok(())
	}
}
