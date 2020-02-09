#![recursion_limit="512"]

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

use failure::Error;
use qint_shared::*;
use slog::{error, o, warn, Drain, Logger};
use wasm_bindgen::prelude::*;
use yew::{html, Component, ComponentLink, Html, ShouldRender};
use yew::format::{Binary, MsgPack, Nothing, Text};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};
use yew::services::websocket::WebSocketStatus;

use crate::connect::Connect;
use crate::connected::Connected;
use crate::connection_service::{ConnectionId, ConnectionService, FrontendConnectionState};

mod connect;
mod connected;
mod connection_service;
mod controls;
mod notifications;
mod plugins;

const SERVER_ICON: &str = "server";
const CHANNEL_ICON: &str = "chat-outline";
const CLIENT_ICON: &str = "account-outline";

pub struct Model {
	link: ComponentLink<Model>,
	logger: Logger,
	/// The currently selected connection if there is one.
	con: Option<ConnectionId>,
	is_talking: bool,
	_set_talking_fetch_task: Option<FetchTask>,
}

pub enum Msg {
	Ignore,
	Connect(ConnectOptions),
	Connected,
	Disconnected,
	Message(MessageP2F),
	Send(MessageF2P),
	SetTalking(bool),
}

impl Model {
	fn get_http_domain() -> String {
		web_sys::window().and_then(|w| w.location().origin().ok())
			.and_then(|l| if l.starts_with("http") {
				Some(l)
			} else {
				None
			}).unwrap_or_else(|| "http://localhost".into())
	}

	fn get_ws_domain() -> String {
		web_sys::window().and_then(|w| w.location().origin().ok())
			.and_then(|l| if l.starts_with("http") {
				Some(format!("ws{}", &l[4..]))
			} else {
				None
			}).unwrap_or_else(|| "ws://localhost".into())
	}

	fn connect(&mut self, options: ConnectOptions) -> Result<(),Error> {
		let logger = self.logger.clone();
		let callback = self.link.callback(move |data: WsMsg| {
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
				_ => {
					error!(logger, "Got unknown data on websocket connection");
					Msg::Ignore
				}
			}
		});
		let notification = self.link.callback(|status| {
			match status {
				WebSocketStatus::Opened => Msg::Connected,
				WebSocketStatus::Closed | WebSocketStatus::Error => Msg::Disconnected,
			}
		});

		// Create id
		self.con = Some(ConnectionService::add(&self.logger, options, callback, notification)?);
		Ok(())
	}
}

impl Component for Model {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let logger = slog::Logger::root(slog_stdlog::StdLog.fuse(), o!());

		Self {
			link,
			logger,
			con: None,
			is_talking: false,
			_set_talking_fetch_task: None,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Connect(options) => {
				if let Err(e) = self.connect(options) {
					error!(self.logger, "Failed to connect to proxy"; "error" => ?e);
				}
				true
			}
			Msg::Connected => {
				let logger = &self.logger;
				ConnectionService::with_mut(self.con.as_ref().unwrap(), move |con| {
					if let FrontendConnectionState::Connecting(options, _, _) = &mut con.state {
						let options = options.clone();
						if let Err(e) = con.send_ws_message(&MessageF2P::Connect(options)) {
							error!(logger, "Failed to send message"; "error" => ?e);
						}
					} else {
						error!(logger, "Wrong state"; "expected" => "connecting");
					}
				}, || panic!("Should be in connecting state"));
				false
			}
			Msg::Disconnected => {
				let state = ConnectionService::remove(&self.con.take().unwrap());
				if let Some(connection_service::FrontendConnection {
					state: FrontendConnectionState::Connecting(_options, _, _), ..
				}) = state {
					// TODO Show options
				}
				true
			}
			Msg::Message(msg) => {
				match msg {
					MessageP2F::ConnectFailed() => {
						warn!(self.logger, "Connect failed; trying next address");
						false
					}
					MessageP2F::ServerKey(key) => {
						ConnectionService::with_mut_unwrap(self.con.as_ref().unwrap(), move |con| {
							if let FrontendConnectionState::Connecting(_, _, k) = &mut con.state {
								*k = Some(key);
								Some(())
							} else { None }
						}, "Should be in connecting state");
						false
					}
					MessageP2F::Packet(packet) => {
						match ConnectionService::with_mut(self.con.as_ref().unwrap(), move |con|
							con.handle_packet(packet),
							|| panic!("Connection not found")) {
							Ok(r) => r,
							Err(e) => {
								error!(self.logger, "Failed to handle packet";
									"error" => ?e);
								false
							}
						}
					}
					MessageP2F::TalkersChanged(_talkers) => {
						// TODO Animate here
						false
					}
					// TODO Remove
					MessageP2F::Webrtc(_) => false,
				}
			}
			Msg::SetTalking(talk) => {
				let mut fetch = FetchService::new();
				let request = Request::post(&format!("{}/audiosend/{}", Self::get_http_domain(), talk))
					.body(Nothing)
					.unwrap();
				let fetch_task = fetch.fetch(request, self.link
					.callback(|resp: Response<Result<String, Error>>| {
						match resp.into_body() {
							Ok(_) => Msg::Ignore,
							Err(e) => {
								// TODO Display error message
								log::error!("Failed to set talking state: {:?}", e);
								Msg::Ignore
							}
						}
					}));
				self._set_talking_fetch_task = Some(fetch_task);

				self.is_talking = talk;
				true
			}
			Msg::Send(msg) => {
				let logger = &self.logger;
				ConnectionService::with_mut(self.con.as_ref().expect("Connection not found"), move |con| {
					if let Err(e) = con.send_ws_message(&msg) {
						error!(logger, "Failed to send message"; "error" => ?e);
					}
				}, || panic!("Connection not found"));
				false
			}
		}
	}

	fn view(&self) -> Html {
		let is_connected = self.con.as_ref().map(|con| ConnectionService::with(
			con,
			|c| c.is_connected(),
			|| false,
		)).unwrap_or_default();
		let is_talking = self.is_talking;
		let talking = if self.is_talking {
			"Stop talking"
		} else {
			"Start talking"
		};

		if !is_connected {
			let onconnect = self.link.callback(Msg::Connect);
			html! {
				<>
					<plugins::Plugins />
					<Connect onconnect=onconnect />
				</>
			}
		} else {
			let switchtalking = self.link.callback(move |_| Msg::SetTalking(!is_talking));
			html! {
				<>
					<plugins::Plugins connection=&self.con />
					<Connected connection=self.con.as_ref().unwrap() />
					<button style="position:absolute; right: 0" onclick=switchtalking>{ talking }</button>
					<notifications::Notifications />
				</>
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

#[wasm_bindgen]
pub fn main() {
	std::panic::set_hook(Box::new(console_error_panic_hook::hook));
	console_log::init_with_level(log::Level::Debug).unwrap();
	yew::start_app::<Model>();
}
