use futures::prelude::*;
use slog::{error, Logger};
use stdweb::web::event::IEvent;
use ts_bookkeeping::{Invoker, MessageTarget};
use ts_bookkeeping::data::Connection;
use yew::html;
use yew::prelude::*;

use crate::{Model, Msg};
use crate::connection::ConnectionMsg;
use super::{ConnectedMsg, MessageHandler};

pub struct Chat {
	logger: Logger,
	messages: Vec<Message>,
	composing: String,
}

pub struct Message {
	pub(super) invoker: Invoker,
	pub(super) message: String,
}

pub enum ChatMsg {
	Change(Box<FnOnce(&mut Chat)>),
	NewMessage(Message),
	Send,
}

impl Into<Msg> for ChatMsg {
	fn into(self) -> Msg {
		Msg::Connection(ConnectionMsg::Connected(
			ConnectedMsg::Chat(self)))
	}
}

impl Chat {
	pub fn new(logger: Logger) -> Self {
		Self {
			logger,
			messages: Default::default(),
			composing: Default::default(),
		}
	}

	pub fn update(&mut self, con: &Connection, msg_handler: &mut MessageHandler, msg: ChatMsg) -> ShouldRender {
		match msg {
			ChatMsg::Change(f) => {
				f(self);
				false
			}
			ChatMsg::NewMessage(msg) => {
				self.messages.push(msg);
				true
			}
			ChatMsg::Send => {
				let cmd = con.send_message(MessageTarget::Channel, &self.composing);
				let logger = self.logger.clone();
				stdweb::spawn_local(msg_handler.send_message(cmd).map(move |r| {
					if let Err(e) = r {
						error!(logger, "Failed to send message"; "error" => ?e);
					}
				}));
				self.composing.clear();
				true
			}
		}
	}

	fn view_message(&self, msg: &Message) -> Html<Model> {
		html! {
			<li>
				<div class="author",>{ &msg.invoker.name }</div>
				<div class="chat-message",>{ &msg.message }</div>
			</li>
		}
	}

	fn view_messages(&self) -> Html<Model> {
		html! {
			<ul class="chat-messages",>
				{ for self.messages.iter()
					.map(|m| self.view_message(m)) }
			</ul>
		}
	}

	pub fn view(&self, _con: &Connection) -> Html<Model> {
		slog::info!(self.logger, "Message"; "value" => &self.composing);
		html! {
			<div class="chat",>
				{ self.view_messages() }
				<form class="chat-form", onsubmit=|e| { e.prevent_default(); ChatMsg::Send.into() },>
					<input name="message", type="text",
						value=&self.composing,
						oninput=|e| ChatMsg::Change({
							Box::new(move |c| { c.composing = e.value; })
						}).into(), />
					<button name="send", type="submit",>
						{ "Send" }
					</button>
				</form>
			</div>
		}
	}
}
