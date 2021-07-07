use std::sync::Arc;

use actix::*;
use actix_web_actors::ws;
use futures::prelude::*;
use qint_proxy::audio::audio_to_ts::LoudnessTrait;
use slog::{warn, Logger};

use crate::web::WebApp;
use qint_proxy::connection::CaptureLoudnessMsg;
use qint_proxy::{audio, QintState};

pub(crate) struct LoudnessService {
	state: Arc<QintState>,
}

impl LoudnessService {
	pub fn new(state: Arc<QintState>) -> Self {
		LoudnessService { state }
	}
}

struct LoudnessCallback {
	logger: Logger,
	addr: Addr<LoudnessService>,
}

impl LoudnessTrait for LoudnessCallback {
	fn send(&self, msg: CaptureLoudnessMsg) {
		let logger = self.logger.clone();
		tokio::spawn(self.addr.send(msg).map(move |r| {
			if let Err(e) = r {
				warn!(logger, "Failed to send loudness"; "error" => %e);
			}
		}));
	}

	fn connected(&self) -> bool {
		self.addr.connected()
	}
}

impl Actor for LoudnessService {
	type Context = ws::WebsocketContext<Self>;
	fn started(&mut self, ctx: &mut Self::Context) {
		if let Some(ad) = &self.state.audio_data {
			let logger = self.state.logger.clone();
			actix::spawn(
				ad.a2ts
					.send(audio::audio_to_ts::AddLoudnessListenerMsg(Box::new(LoudnessCallback {
						logger: logger.clone(),
						addr: ctx.address(),
					})))
					.map(move |r| {
						if let Err(e) = r {
							warn!(logger, "Failed to add loudness listener: {}", e);
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
			_ => {}
		}
	}
}

impl Handler<CaptureLoudnessMsg> for LoudnessService {
	type Result = ();
	fn handle(
		&mut self, CaptureLoudnessMsg(loudness): CaptureLoudnessMsg, ctx: &mut Self::Context,
	) -> Self::Result {
		ctx.binary(loudness.to_be_bytes().to_vec());
	}
}

impl WebApp {}
