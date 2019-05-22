use qint_shared::*;
use slog::{error, warn, Logger};
use stdweb::web::event::IEvent;
use ts_bookkeeping::Uid;
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::messages::s2c::{InMessage, InMessages};
use yew::html;
use yew::format::MsgPack;
use yew::prelude::*;
use yew::services::websocket::{WebSocketService, WebSocketTask, WebSocketStatus};

use crate::{Model, Msg, WsMsg};
use crate::connected::{Connected, ConnectedMsg};

pub struct WsConnection {
	state: ConnectionState,
	ws_service: WebSocketService,
	ws: Option<WebSocketTask>,
	logger: Logger,
}

pub enum ConnectionMsg {
	Connect,
	WsConnected,
	WsDisconnected,
	Disconnected(DisconnectedMsg),
	Connected(ConnectedMsg),
	Message(MessageP2F),
}

enum ConnectionState {
	Disconnected(Disconnected),
	Connected(Connected),
}

/// Shows the login form
struct Disconnected {
	options: ConnectOptions,
}

pub enum DisconnectedMsg {
	Change(Box<FnOnce(&mut ConnectOptions)>),
}

impl WsConnection {
	pub fn new(logger: Logger) -> Self {
		WsConnection {
			state: ConnectionState::Disconnected(Disconnected::default()),
			ws_service: WebSocketService::new(),
			ws: None,
			logger,
		}
	}

	pub fn send_message(&mut self, msg: &MessageF2P) {
		self.ws.as_mut().unwrap().send_binary(MsgPack(msg));
	}

	pub fn update(&mut self, msg: ConnectionMsg, link: &mut ComponentLink<Model>) -> ShouldRender {
		match msg {
			ConnectionMsg::Connect => {
				let logger = self.logger.clone();
				let callback = link.send_back(move |data: WsMsg| {
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

							Msg::Connection(ConnectionMsg::Message(data))
						}
						t => {
							error!(logger, "Got unknown data"; "data" => ?t);
							Msg::Ignore
						}
					}
				});
				let notification = link.send_back(|status| {
					match status {
						WebSocketStatus::Opened => Msg::Connection(ConnectionMsg::WsConnected),
						WebSocketStatus::Closed | WebSocketStatus::Error => ConnectionMsg::WsDisconnected.into(),
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
				self.ws = Some(task);
				true
			}
			ConnectionMsg::WsConnected => {
				if let ConnectionState::Disconnected(state) = &mut self.state {
					let options = state.options.clone();
					self.send_message(&MessageF2P::Connect(options));
				} else {
					error!(self.logger, "Wrong state"; "expected" => "Disconnected");
				}
				false
			}
			ConnectionMsg::WsDisconnected => {
				self.ws = None;
				if let ConnectionState::Disconnected(_) = self.state {
				} else {
					self.state = ConnectionState::Disconnected(Disconnected::default());
				}
				true
			}
			ConnectionMsg::Disconnected(dm) => match &mut self.state {
				ConnectionState::Disconnected(s) => s.update(dm),
				_ => {
					error!(self.logger, "Wrong state"; "expected" => "Disconnected");
					false
				}
			}
			ConnectionMsg::Connected(dm) => match &mut self.state {
				ConnectionState::Connected(s) => {
					let (packets, should_render) = s.update(dm);
					for p in packets {
						self.send_message(&MessageF2P::Packet(p));
					}
					should_render
				}
				_ => {
					error!(self.logger, "Wrong state"; "expected" => "Connected");
					false
				}
			}
			ConnectionMsg::Message(msg) => {
				match msg {
					MessageP2F::ConnectFailed() => {
						warn!(self.logger, "Connect failed; trying next address");
						false
					}
					MessageP2F::Packet(packet) => {
						match &mut self.state {
							ConnectionState::Connected(s) => {
								let (packets, should_render) = s.update(ConnectedMsg::Packet(packet));
								for p in packets {
									self.send_message(&MessageF2P::Packet(p));
								}
								should_render
							}
							ConnectionState::Disconnected(_) => {
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
								self.state = ConnectionState::Connected(
									Connected::new(Connection::new(
										Uid("".into()),
										&msg,
									), self.logger.clone()));
								true
							}
						}
					}
				}
			}
		}
	}
}

impl Component for WsConnection {
	type Message = ConnectionMsg;
	type Properties = ();

	fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
		panic!("Should not be called");
	}

	fn update(&mut self, _msg: Self::Message) -> ShouldRender {
		panic!("Should not be called");
	}
}

impl Renderable<Model> for WsConnection {
	fn view(&self) -> Html<Model> {
		match &self.state {
			ConnectionState::Disconnected(s) => s.view(),
			ConnectionState::Connected(s) => s.view(),
		}
	}
}

impl Into<Msg> for ConnectionMsg {
	fn into(self) -> Msg { Msg::Connection(self) }
}

impl Default for Disconnected {
	fn default() -> Self {
		Self {
			options: ConnectOptions::new("localhost".into()),
		}
	}
}

impl Disconnected {
	fn update(&mut self, msg: DisconnectedMsg) -> ShouldRender {
		match msg {
			DisconnectedMsg::Change(f) => f(&mut self.options),
		}
		false
	}
}

fn checkbox_value(e: &ChangeData) -> bool {
	if let ChangeData::Value(v) = e {
		v == "true"
	} else {
		false
	}
}

impl Renderable<Model> for Disconnected {
	fn view(&self) -> Html<Model> {
		html! {
			<div class="connect-container",>
			<form class="connect-form", onsubmit=|e| { e.prevent_default(); ConnectionMsg::Connect.into() },>
				<div class="connect-item",>
					<input name="username", type="text", placeholder="Username",
						value=&self.options.name,
						oninput=|e| DisconnectedMsg::Change({
							Box::new(move |o| { o.name(e.value); })
						}).into(), />
				</div>
				<div class="connect-item",>
					<input name="server", type="text", placeholder="Server",
						value=&self.options.address,
						oninput=|e| DisconnectedMsg::Change({
							Box::new(move |o| { o.address(e.value); })
						}).into(), />
				</div>
				<div class="connect-item",>
					<label>
						<input name="log-commands", type="checkbox", value="true",
							onchange=|e| DisconnectedMsg::Change({
								Box::new(move |o| { o.log_commands(checkbox_value(&e)); })
							}).into(), />
						{ "Log commands" }
					</label>
				</div>
				<div class="connect-item",>
					<label>
						<input name="log-packets", type="checkbox", value="true",
							onchange=|e| DisconnectedMsg::Change({
								Box::new(move |o| { o.log_packets(checkbox_value(&e)); })
							}).into(), />
						{ "Log packets" }
					</label>
				</div>
				<div class="connect-item",>
					<button name="connect", type="submit",>
						{ "Connect" }
					</button>
				</div>
			</form>
			</div>
		}
	}
}

impl Into<Msg> for DisconnectedMsg {
	fn into(self) -> Msg { Msg::Connection(ConnectionMsg::Disconnected(self)) }
}
