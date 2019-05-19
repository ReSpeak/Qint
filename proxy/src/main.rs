use actix::*;
use actix::fut::wrap_future;
use actix_web::*;
use failure::Error;
use futures::prelude::*;
use qint_shared::{InCommandMsg, MessageF2P, MessageP2F};
use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};
use tsclientlib::Connection;
use tsproto::handler_data::InCommandObserver;
use tsproto_packets::packets::InCommand;

/// Define http actor
struct Ws {
	connection: Option<Connection>,
}

#[derive(Clone)]
struct ProxyCommandObserver {
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
						let addr = ctx.address();
						let con = Connection::new(tsclientlib::ConnectOptions::new(o.address)
							.name(o.name)
							.log_commands(o.log_commands)
							.log_packets(o.log_packets)
							.log_udp_packets(o.log_udp_packets)
							.prepare_client(Box::new(move |client| {
								client.lock().add_in_command_observer(
									"Qint".into(),
									Box::new(ProxyCommandObserver {
										addr: addr.clone(),
									}),
								);
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

impl<T> InCommandObserver<T> for ProxyCommandObserver {
	fn observe(&self, _: &mut (T, tsproto::connection::Connection), cmd: &InCommand) {
		actix::spawn(self.addr.send(WsMessage::Packet(cmd.into())).map_err(|_| {
			eprintln!("Failed to send packet");
		}));
	}
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
