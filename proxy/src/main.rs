use actix::*;
use actix::fut::wrap_future;
use actix_web::*;
use failure::Error;
use futures::prelude::*;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use tsclientlib::{Connection, PacketHandler, PHBox};
use tsproto_packets::packets::{InAudio, InCommand};

/// Define http actor
struct Ws {
	connection: Option<Connection>,
}

#[derive(Clone)]
struct ProxyPacketHandler {
	addr: Addr<Ws>,
}

enum WsMessage {
	Packet(InCommandMsg),
}

impl Actor for Ws {
	type Context = ws::WebsocketContext<Self>;
}

impl Message for WsMessage {
	type Result = ();
}

impl Handler<WsMessage> for Ws {
	type Result = ();
	fn handle(&mut self, msg: WsMessage, ctx: &mut Self::Context) -> Self::Result {
		match msg {
			WsMessage::Packet(packet) => {
				let mut buf = Vec::new();
				let mut ser = Serializer::new(&mut buf);
				MessageP2F::Packet(packet).serialize(&mut ser).unwrap();
				ctx.binary(buf);
			}
		}
	}
}

/// Handler for ws::Message message
impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
	fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
		match msg {
			ws::Message::Ping(msg) => ctx.pong(&msg),
			ws::Message::Text(text) => ctx.text(text),
			ws::Message::Binary(bin) => {
				let mut de = Deserializer::new(bin.as_ref());
				let msg: MessageF2P = match Deserialize::deserialize(&mut de) {
					Ok(r) => r,
					Err(e) => {
						// TODO log
						eprintln!("Error deserializing message: {:?}", e);
						return;
					}
				};

				match msg {
					MessageF2P::Connect(o) => {
						let con = Connection::new(tsclientlib::ConnectOptions::new(o.address)
							.name(o.name)
							.log_commands(o.log_commands)
							.log_packets(o.log_packets)
							.log_udp_packets(o.log_udp_packets)
							.handle_packets(Box::new(ProxyPacketHandler {
								addr: ctx.address(),
							}))
						);

						ctx.spawn(wrap_future(con).map(|con, actor: &mut Ws, _ctx| {
							actor.connection = Some(con);
						})
						.map_err(|_e, _actor, ctx| {
							let val = MessageP2F::ConnectFailed();
							let mut buf = Vec::new();
							let mut ser = Serializer::new(&mut buf);
							val.serialize(&mut ser).unwrap();
							ctx.binary(buf);
						}));
					}
					MessageF2P::Packet(packet) => {
						if let Some(con) = &mut self.connection {
							ctx.spawn(wrap_future(con.send_packet(packet))
								.map_err(|e, _actor: &mut Ws, _ctx| {
									// TODO Return
									eprintln!("Failed to send packet: {:?}", e);
								}));
						}
					}
				}

				ctx.binary(bin);
			}
			_ => (),
		}
	}
}

impl PacketHandler for ProxyPacketHandler {
	fn new_connection(
		&mut self,
		command_stream: Box<
			Stream<Item = InCommand, Error = tsproto::Error> + Send,
		>,
		audio_stream: Box<
			Stream<Item = InAudio, Error = tsproto::Error> + Send,
		>,
	) {
		let addr = self.addr.clone();
		actix::spawn(command_stream.from_err::<Error>().for_each(move |p| {
			addr.send(WsMessage::Packet((&p).into())).from_err()
		}).map_err(move |e| {
			// This happens when the websocket connection is lost.
			// The ts connection will be closed automatically.
			eprintln!("Command stream exited");
			// TODO
			//error!(logger, "Command stream exited with error ({:?})", e);
		}));
		actix::spawn(audio_stream.for_each(|_| Ok(())).map_err(move |e| {
			eprintln!("Audio stream exited");
			// TODO
			//error!(logger, "Audio stream exited with error ({:?})", e);
		}));
	}

	/// Clone into a box.
	fn clone(&self) -> PHBox { Box::new(Clone::clone(self)) }
}

fn main() {
	server::new(|| App::new()
		.resource("/ws", |r| r.f(|req| ws::start(req, Ws {
			connection: None,
		})))
		.finish())
		.bind("127.0.0.1:4422")
		.unwrap()
		.run();
}
