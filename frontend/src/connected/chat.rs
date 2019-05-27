use futures::prelude::*;
use slog::error;
use stdweb::web::event::IEvent;
use ts_bookkeeping::MessageTarget;
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

pub struct Chat {
	con: ConnectionId,
	callback: Callback<()>,
}

pub enum Msg {
	Ignore,
	Change(Box<FnOnce(&mut Connected)>),
	NewMessage,
	Send,
}

#[derive(Clone, Default, PartialEq)]
pub struct Props {
	pub connection: Option<ConnectionId>,
}

impl Component for Chat {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
		let con = props.connection.expect("Chat needs a connection id");

		let callback = link.send_back(|_| Msg::NewMessage);

		let res = Self {
			con,
			callback,
		};
		res.add_listener();
		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Change(f) => {
				ConnectionService::with_mut_con(self.con, |con| if let
					FrontendConnectionState::Connected(c) = &mut con.state {
					f(c);
				} else {
					panic!("Should be in connected state");
				}, || panic!("Should be in connected state"));
				true
			}
			Msg::NewMessage => true,
			Msg::Send => {
				ConnectionService::with_mut_con(self.con, |con| if let
					FrontendConnectionState::Connected(c) = &mut con.state {
					let cmd = c.con.send_message(MessageTarget::Channel, &c.composing);
					c.composing.clear();
					let logger = con.logger.clone();
					stdweb::spawn_local(con.send_message(cmd).map(move |r| {
						if let Err(e) = r {
							// TODO Display notification
							error!(logger, "Failed to send message"; "error" => ?e);
						}
					}));
				} else {
					panic!("Should be in connected state");
				}, || panic!("Should be in connected state"));
				true
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let con = props.connection.expect("Connect needs a connection id");
		if self.con != con {
			// Remove and add listener
			ConnectionService::with_mut_con(con, |con| {
				con.packet_listeners.remove("chat");
			}, || {});

			self.con = con;
			self.add_listener();
			true
		} else {
			false
		}
	}
}

impl Chat {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut_con(self.con, |con| {
			let callback = self.callback.clone();
			con.packet_listeners.insert("chat".into(), Box::new(move |_, msg| {
				if msg.name() == "notifytextmessage" {
					callback.emit(());
				}
			}));
		}, || panic!("Should be in connected state"));

	}

	fn view_message(&self, msg: &Message) -> Html<Self> {
		html! {
			<li>
				<div class="author",>{ &msg.invoker.name }</div>
				<div class="chat-message",>{ &msg.message }</div>
			</li>
		}
	}

	fn view_messages(&self, con: &Connected) -> Html<Self> {
		html! {
			<ul class="chat-messages",>
				{ for con.messages.iter()
					.map(|m| self.view_message(m)) }
				<span class="chat-end",></span>
			</ul>
		}
		// TODO Use document.querySelectorAll('.chat-end')[0].scrollIntoView({behavior: "smooth"})
	}
}

impl Renderable<Self> for Chat {
	fn view(&self) -> Html<Self> {
		ConnectionService::with_con(self.con, |con| if let
			FrontendConnectionState::Connected(c) = &con.state {
			html! {
				<div class="chat",>
					{ self.view_messages(c) }
					<form class="chat-form", onsubmit=|e| { e.prevent_default(); Msg::Send.into() },>
						<input name="message", type="text",
							value=&c.composing,
							oninput=|e| Msg::Change({
								Box::new(move |c| { c.composing = e.value; })
							}).into(), />
						<button name="send", type="submit",>
							{ "Send" }
						</button>
					</form>
				</div>
			}
		} else {
			panic!("Should be in connected state");
		}, || panic!("Should be in connected state"))
	}
}
