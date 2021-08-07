use std::sync::Arc;

use actix::fut::wrap_future;
use actix::*;
use actix_web_actors::ws;
use futures::prelude::*;
use slog::Logger;

use crate::web::WebApp;
use qint_proxy::audio::audio_to_ts::LoudnessTrait;
use qint_proxy::connection::CaptureLoudnessMsg;
use qint_proxy::{audio, with_log, QintState};

pub(crate) struct LoudnessService {
	state: Arc<QintState>,
	listener_handle: Option<usize>,
}

impl LoudnessService {
	pub fn new(state: Arc<QintState>) -> Self {
		LoudnessService { state, listener_handle: None }
	}
}

struct LoudnessCallback {
	logger: Logger,
	addr: Addr<LoudnessService>,
}

impl LoudnessTrait for LoudnessCallback {
	fn send(&self, msg: CaptureLoudnessMsg) {
		actix::spawn(with_log!(
			self.addr.send(msg),
			self.logger.clone(),
			"Failed to send loudness"
		));
	}

	fn connected(&self) -> bool {
		self.addr.connected()
	}
}

impl Actor for LoudnessService {
	type Context = ws::WebsocketContext<Self>;
	fn started(&mut self, ctx: &mut Self::Context) {
		if let Some(ad) = &self.state.audio_data {
			ctx.spawn(
				wrap_future(ad.a2ts.send(audio::audio_to_ts::AddLoudnessListenerMsg(Box::new(
					LoudnessCallback { logger: self.state.logger.clone(), addr: ctx.address() },
				))))
				.map(move |handle, actor: &mut Self, _ctx| match handle {
					Ok(handle) => {
						actor.listener_handle = Some(handle);
					}
					Err(e) => {
						error!(actor.state.logger, "Failed to remove loudness listener"; "error" => %e);
					}
				}),
			);
		}
	}
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for LoudnessService {
	fn handle(
		&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>,
		ctx: &mut Self::Context,
	) {
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Close(_)) => {
				if let Some(handle) = self.listener_handle.take() {
					if let Some(ad) = &self.state.audio_data {
						actix::spawn(with_log!(
							ad.a2ts.send(audio::audio_to_ts::RemoveLoudnessListenerMsg(handle)),
							self.state.logger.clone(),
							"Failed to remove loudness listener"
						));
					}
				}
				ctx.close(None);
			}
			_ => {}
		}
	}
}

impl Handler<CaptureLoudnessMsg> for LoudnessService {
	type Result = ();
	fn handle(
		&mut self, CaptureLoudnessMsg(loudness, vad): CaptureLoudnessMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		let mut voice_data = Vec::with_capacity(16);
		voice_data.extend_from_slice(&loudness.to_be_bytes());
		voice_data.extend_from_slice(&vad.to_be_bytes());
		ctx.binary(voice_data);
	}
}

impl WebApp {}
