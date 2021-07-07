use actix::*;
use actix_web_actors::ws;

use proxy_codegen::markdown::markdown;

pub(crate) struct MarkdownService {}

impl Actor for MarkdownService {
	type Context = ws::WebsocketContext<Self>;
}

impl MarkdownService {
	pub fn new() -> Self { MarkdownService {} }
}

impl StreamHandler<std::result::Result<ws::Message, ws::ProtocolError>> for MarkdownService {
	fn handle(
		&mut self, msg: std::result::Result<ws::Message, ws::ProtocolError>,
		ctx: &mut Self::Context,
	) {
		match msg {
			Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
			Ok(ws::Message::Text(msg)) => {
				let rendered = markdown(&msg);
				ctx.text(rendered);
			}
			_ => {}
		}
	}
}
