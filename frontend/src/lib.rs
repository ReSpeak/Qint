#![feature(async_await)]
#![recursion_limit="128"]

use failure::Error;
use qint_shared::*;
use slog::{error, o, warn, Drain, Logger};
use ts_bookkeeping::Uid;
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::messages::s2c::{InMessage, InMessages};
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};
use yew::format::{Binary, MsgPack, Text};
use yew::services::websocket::{WebSocketService, WebSocketTask, WebSocketStatus};

use crate::connect::Connect;
use crate::connection_service::{Connected, ConnectionId, ConnectionService, FrontendConnectionState};

mod connect;
//mod connected;
//mod connection;
mod connection_service;

pub struct Model {
	ws_service: WebSocketService,
	link: ComponentLink<Model>,
	logger: Logger,
	/// The currently selected connection.
	con: ConnectionId,
}

pub enum Msg {
	Ignore,
	Connect(ConnectOptions),
	Connected,
	Disconnected,
	Message(MessageP2F),
}

impl Component for Model {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let logger = slog::Logger::root(slog_stdlog::StdLog.fuse(), o!());
		let con = ConnectionService::add_connection(&logger);

		Model {
			ws_service: WebSocketService::new(),
			link,
			logger,
			con,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Connect(options) => {
				let logger = self.logger.clone();
				let callback = self.link.send_back(move |data: WsMsg| {
					match data {
						WsMsg::Binary(data) => {
							let MsgPack(data) = data.into();
							let data = match data {
								Ok(r) => r,
								Err(e) => {
									error!(logger, "Error parsing data"; "error" => ?e);
									return Msg::Ignore;
								}
							};

							Msg::Message(data)
						}
						t => {
							error!(logger, "Got unknown data");
							Msg::Ignore
						}
					}
				});
				let notification = self.link.send_back(|status| {
					match status {
						WebSocketStatus::Opened => Msg::Connected,
						WebSocketStatus::Closed | WebSocketStatus::Error => Msg::Disconnected,
					}
				});

				// Get url
				let url = stdweb::web::window()
					.location()
					.and_then(|l| l.origin().ok())
					.and_then(|l| if l.starts_with("http") {
						Some(format!("ws{}/ws", &l[4..]))
					} else {
						None
					}).unwrap_or_else(|| "ws://localhost/ws".into());

				let task = self.ws_service.connect(&url, callback, notification);
				ConnectionService::with_mut_con(self.con, move |con| if let
					FrontendConnectionState::Disconnected(options, ws)
					= &mut con.state {
						*ws = Some(task);
					} else {
					}, || panic!("Should be in disconnected state"));
				true
			}
			Msg::Connected => {
				ConnectionService::with_mut_con(self.con, move |con| {
					if let FrontendConnectionState::Disconnected(options, _) = &mut con.state {
						let options = options.clone();
						con.send_ws_message(&MessageF2P::Connect(options));
					} else {
						error!(self.logger, "Wrong state"; "expected" => "Disconnected");
					}
				}, || panic!("Should be in disconnected state"));
				false
			}
			Msg::Disconnected => {
				ConnectionService::with_mut_con(self.con, move |con| {
					con.state = FrontendConnectionState::default();
				}, || panic!("Should be in disconnected state"));
				true
			}
			Msg::Message(msg) => {
				match msg {
					MessageP2F::ConnectFailed() => {
						warn!(self.logger, "Connect failed; trying next address");
						false
					}
					MessageP2F::Packet(packet) => {
						ConnectionService::with_mut_con(self.con, move |con| match &mut con.state {
							FrontendConnectionState::Disconnected(_, ws) => {
								let msg = match InMessage::new(packet.into()) {
									Ok(r) => r,
									Err(e) => {
										error!(self.logger, "Failed to parse packet"; "error" => ?e);
										return false;
									}
								};
								if let InMessages::InitServer(_) = msg.msg() {
								} else if let InMessages::InitIvExpand2(_) = msg.msg() {
									return false;
								} else {
									error!(self.logger, "Got no initserver as first packet";
										"packet" => ?msg);
									return false;
								}

								// TODO Uid
								con.state = FrontendConnectionState::Connected(
									Connected::new(ws.take().unwrap(), Connection::new(
										Uid("".into()),
										&msg,
									)));
								true
							}
							FrontendConnectionState::Connected(c) => {
								// TODO
								/*let (packets, should_render) = s.update(Msg::Packet(packet));
								should_render*/
								false
							}
						}, || panic!("Should be in disconnected state"))
					}
				}
			}
			/*Msg::Connection(cm) => {
				return self.connections[self.con].update(cm, &mut self.link);
			}*/
		}
	}
}

impl Renderable<Model> for Model {
	fn view(&self) -> Html<Self> {
		let is_connected = ConnectionService::with_con(
			self.con,
			|c| c.is_connected(),
			|| false,
		);
		if !is_connected {
			let con = Some(self.con);
			html! {
				<Connect: connection=con, onconnect=|o| Msg::Connect(o), />
			}
		} else {
			// TODO
			html! {
				<div>
				</div>
			}
		}
	}
}

pub enum WsMsg {
	Text(Text),
	Binary(Binary),
}

impl From<Text> for WsMsg {
	fn from(t: Text) -> WsMsg { WsMsg::Text(t) }
}

impl From<Binary> for WsMsg {
	fn from(b: Binary) -> WsMsg { WsMsg::Binary(b) }
}
