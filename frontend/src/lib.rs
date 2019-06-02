#![feature(async_await)]
#![recursion_limit="256"]

use qint_shared::*;
use slog::{error, o, warn, Drain, Logger};
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};
use yew::format::{Binary, MsgPack, Text};
use yew::services::websocket::{WebSocketService, WebSocketStatus};

use crate::connect::Connect;
use crate::connected::Connected;
use crate::connection_service::{ConnectionId, ConnectionService, FrontendConnectionState};

mod connect;
mod connected;
mod connection_service;
mod webrtc;

pub struct Model {
	ws_service: WebSocketService,
	link: ComponentLink<Model>,
	logger: Logger,
	rtc: Option<webrtc::Webrtc>,
	rtc_queue: Vec<WebrtcMsg>,
	/// The currently selected connection.
	con: ConnectionId,
	is_talking: bool,
}

pub enum Msg {
	Ignore,
	Connect,
	Connected,
	Disconnected,
	Message(MessageP2F),
	WebrtcReady,
	Send(MessageF2P),
	SetTalking(bool),
}

impl Model {
	fn connect(&mut self) {
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
				_ => {
					error!(logger, "Got unknown data on websocket connection");
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
			FrontendConnectionState::Disconnected(_, ws)
			= &mut con.state {
				*ws = Some(task);
			} else {
			}, || panic!("Should be in disconnected state"));
	}
}

impl Component for Model {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let logger = slog::Logger::root(slog_stdlog::StdLog.fuse(), o!());
		let con = ConnectionService::add_connection(&logger);

		// TODO Create webrtc connection
		// For some reason it does not work if we do it afterwards
		/*let callback = link.send_back(|data: Option<WebrtcMsg>| {
			if let Some(data) = data {
				Msg::Send(MessageF2P::Webrtc(data))
			} else {
				Msg::WebrtcReady
			}
		});
		let rtc = Some(webrtc::Webrtc::new(callback));*/

		Self {
			ws_service: WebSocketService::new(),
			link,
			rtc: None,
			rtc_queue: Default::default(),
			logger,
			con,
			is_talking: false,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Connect => {
				self.connect();
				true
			}
			Msg::Connected => {
				ConnectionService::with_mut_con(self.con, move |con| {
					if let FrontendConnectionState::Disconnected(options, _) = &mut con.state {
						let options = options.clone();
						if let Err(e) = con.send_ws_message(&MessageF2P::Connect(options)) {
							error!(self.logger, "Failed to send message"; "error" => ?e);
						}
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
						match ConnectionService::with_mut_con(self.con, move |con|
							con.handle_packet(packet),
							|| panic!("Should be in disconnected state")) {
							Ok(r) => r,
							Err(e) => {
								error!(self.logger, "Failed to handle packet";
									"error" => ?e);
								false
							}
						}
					}
					MessageP2F::Webrtc(msg) => {
						if self.rtc.is_none() {
							// Create webrtc connection
							let callback = self.link.send_back(|data: Option<WebrtcMsg>| {
								if let Some(data) = data {
									Msg::Send(MessageF2P::Webrtc(data))
								} else {
									Msg::WebrtcReady
								}
							});
							self.rtc = Some(webrtc::Webrtc::new(callback));
							self.rtc_queue.push(msg);
						} else if self.rtc_queue.is_empty() {
							// Webrtc is ready
							self.rtc.as_mut().unwrap().handle(msg);
						} else {
							self.rtc_queue.push(msg);
						}
						false
					}
				}
			}
			Msg::SetTalking(talk) => {
				ConnectionService::with_mut_con(self.con, |con| {
					if let Err(e) = con.send_ws_message(&MessageF2P::SetTalking(talk)) {
						error!(con.logger, "Failed to send websocket message"; "error" => ?e);
					}
				}, || panic!("Should be in connected state"));
				self.is_talking = talk;
				if let Some(rtc) = &mut self.rtc {
					rtc.set_talking(talk);
				}
				true
			}
			Msg::WebrtcReady => {
				for msg in std::mem::replace(&mut self.rtc_queue, Vec::new()) {
					self.rtc.as_mut().unwrap().handle(msg);
				}
				false
			}
			Msg::Send(msg) => {
				ConnectionService::with_mut_con(self.con, move |con| {
					if let Err(e) = con.send_ws_message(&msg) {
						error!(self.logger, "Failed to send message"; "error" => ?e);
					}
				}, || panic!("Connection not found"));
				false
			}
		}
	}
}

impl Renderable<Self> for Model {
	fn view(&self) -> Html<Self> {
		let is_connected = ConnectionService::with_con(
			self.con,
			|c| c.is_connected(),
			|| false,
		);
		let con = Some(self.con);
		let is_talking = self.is_talking;
		let talking = if self.is_talking {
			"Stop talking"
		} else {
			"Start talking"
		};

		if !is_connected {
			html! {
				<>
				<audio id="audio-playback", autoplay="autoplay", />
				<Connect: connection=con, onconnect=|_| Msg::Connect, />
				</>
			}
		} else {
			html! {
				<>
				<audio id="audio-playback", autoplay="autoplay", />
				<Connected: connection=con, />
				<button style="position:absolute; right: 0", onclick=|_| Msg::SetTalking(!is_talking).into(),>{ talking }</button>
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
