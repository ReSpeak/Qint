use qint_shared::*;
use stdweb::web::event::IEvent;
use ts_bookkeeping::Uid;
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::messages::s2c::{InMessage, InMessages};
use yew::{html};
use yew::format::MsgPack;
use yew::prelude::*;
use yew::services::websocket::{WebSocketService, WebSocketTask, WebSocketStatus};

use crate::{Model, Msg, WsAction, WsMsg};

pub struct WsConnection {
	state: ConnectionState,
	ws_service: WebSocketService,
	ws: Option<WebSocketTask>,
}

pub enum ConnectionMsg {
	Connect,
	WsConnected,
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

struct Connected {
	connection: Connection,
}

pub enum ConnectedMsg {
	Packet(InCommandMsg),
}

impl WsConnection {
	pub fn new() -> Self {
		WsConnection {
			state: ConnectionState::Disconnected(Disconnected::default()),
			ws_service: WebSocketService::new(),
			ws: None,
		}
	}

	pub fn update(&mut self, msg: ConnectionMsg, link: &mut ComponentLink<Model>) -> ShouldRender {
		match msg {
			ConnectionMsg::Connect => {
				let callback = link.send_back(|data: WsMsg| {
					match data {
						WsMsg::Binary(data) => {
							let MsgPack(data) = data.into();
							let data = match data {
								Ok(r) => r,
								Err(e) => {
									// TODO Log
									eprintln!("Error parsing data {:?}", e);
									return Msg::Ignore;
								}
							};

							Msg::Connection(ConnectionMsg::Message(data))
						}
						t => {
							eprintln!("Got unknown data {:?}", t);
							Msg::Ignore
						}
					}
				});
				let notification = link.send_back(|status| {
					match status {
						WebSocketStatus::Opened => Msg::Connection(ConnectionMsg::WsConnected),
						WebSocketStatus::Closed | WebSocketStatus::Error => WsAction::Lost.into(),
					}
				});
				let task = self.ws_service.connect("ws://localhost:4422/ws", callback, notification);
				self.ws = Some(task);
				true
			}
			ConnectionMsg::WsConnected => {
				if let ConnectionState::Disconnected(state) = &mut self.state {
					self.ws.as_mut().unwrap().send_binary(MsgPack(
						&MessageF2P::Connect(state.options.clone())));
				} else {
					eprintln!("Wrong state");
				}
				false
			}
			ConnectionMsg::Disconnected(dm) => match &mut self.state {
				ConnectionState::Disconnected(s) => s.update(dm),
				_ => {
					eprintln!("Wrong state");
					false
				}
			}
			ConnectionMsg::Connected(dm) => match &mut self.state {
				ConnectionState::Connected(s) => s.update(dm),
				_ => {
					eprintln!("Wrong state");
					false
				}
			}
			ConnectionMsg::Message(msg) => {
				match msg {
					MessageP2F::ConnectFailed() => {
						eprintln!("Failed to connect");
						false
					}
					MessageP2F::Packet(packet) => {
						match &mut self.state {
							ConnectionState::Connected(s) =>
								s.update(ConnectedMsg::Packet(packet)),
							ConnectionState::Disconnected(_) => {
								let msg = match InMessage::new(packet.into()) {
									Ok(r) => r,
									Err(e) => {
										eprintln!("Failed to parse packet: {:?}", e);
										return false;
									}
								};
								if let InMessages::InitServer(_) = msg.msg() {
								} else {
									eprintln!("Got not an initserver packet");
									console!(log, "Got no initserver");
									return false;
								}
								console!(log, "Got initserver");

								// TODO Uid
								self.state = ConnectionState::Connected(Connected {
									connection: Connection::new(Uid("".into()),
										&msg),
								});
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

impl Component for Disconnected {
	type Message = DisconnectedMsg;
	type Properties = ();

	fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
		panic!("Should not be called");
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			DisconnectedMsg::Change(f) => f(&mut self.options),
		}
		false
	}
}

impl Component for Connected {
	type Message = ConnectedMsg;
	type Properties = ();

	fn create(_: Self::Properties, _: ComponentLink<Self>) -> Self {
		panic!("Should not be called");
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			ConnectedMsg::Packet(packet) => {
				let packet = packet.into();
				/*let msg = match InMessage::new(packet) {
					Ok(r) => r,
					Err(e) => {
						eprintln!("Failed to parse packet: {:?}", e);
						return false;
					}
				};*/

				match self.connection.handle_command(&packet) {
					Ok(events) => {
						// TODO
						console!(log, "Got event");
						true
					}
					Err(e) => {
						console!(log, "Failed to handle");
						eprintln!("Failed to handle command: {:?}", e);
						false
					}
				}
			}
		}
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

impl Renderable<Model> for Connected {
	fn view(&self) -> Html<Model> {
		html! {
			<div class="connected-container",>
			</div>
		}
	}
}

impl Into<Msg> for ConnectedMsg {
	fn into(self) -> Msg { Msg::Connection(ConnectionMsg::Connected(self)) }
}
