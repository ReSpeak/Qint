use chrono::NaiveDateTime;
use failure::Error;
use qint_shared::{ChatId, ChatType, MessagesRequest};
use qint_shared::models::Message;
use std::borrow::Cow;
use stdweb::web::event::IEvent;
use ts_bookkeeping::MessageTarget;
use tsproto_packets::packets::{Direction, Flags, OutCommand, OutPacket, PacketType};
use yew::format::MsgPack;
use yew::html;
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use crate::connection_service::*;

pub struct Chat {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: ChatId,
	// TODO More than one
	fetch_task: Option<FetchTask>,

	messages: Vec<Message>,
}

pub enum Msg {
	Ignore,
	Change(Box<dyn FnOnce(&mut Connected)>),
	GotMessages(Vec<Message>),
	NewMessage,
	SetChatToChannel,
	Send,
	SendCommand,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
}

impl Component for Chat {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let server = ConnectionService::with_ready_unwrap(&props.connection, |con|
			con.key.clone()
		);
		// On channel change:
		// TODO Update chat id to new channel if current id points to current channel
		// TODO Add this as event listener

		let chat = ChatId {
			server,
			// TODO Default chat should be channel
			chat_type: ChatType::Server,
		};

		let mut res = Self {
			link,
			con: props.connection,
			chat,
			fetch_task: None,

			messages: Vec::new(),
		};
		res.add_listener();
		// Request messages
		res.request_messages(None);

		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Change(f) => {
				ConnectionService::with_mut_ready_unwrap(&self.con, f);
				true
			}
			Msg::GotMessages(msgs) => {
				// TODO Clever insert
				self.messages = msgs;
				true
			}
			Msg::NewMessage => {
				self.request_messages(None);
				false
			}
			Msg::SetChatToChannel => {
				// Set chat id to channel
				self.chat.chat_type = ChatType::Channel(ConnectionService::with_mut_ready_unwrap(&self.con, |c| {
					c.con.clients[&c.con.own_client].channel.0
				}));
				self.request_messages(None);
				false
			}
			Msg::Send => {
				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
					let cmd = c.con.send_message(MessageTarget::Channel, &c.composing);
					c.composing.clear();
					Some(cmd)
				}, "Failed to send message");
				true
			}
			Msg::SendCommand => {
				ConnectionService::with_mut_ready_unwrap(&self.con, |c| {
					let mut packet = OutPacket::new_with_dir(Direction::C2S,
						Flags::empty(), PacketType::Command);
					let static_args = std::iter::empty();
					let list_args = std::iter::empty();
					OutCommand::new_into::<&'static str, Cow<str>, &'static str, Cow<str>, _, _, std::iter::Empty<_>>(
						&c.composing_command, static_args, list_args, packet.data_mut());

					c.composing_command.clear();
				});
				true
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&props.connection, |con| {
				con.packet_listeners.remove("chat");
			}, || {});

			self.con = props.connection;
			self.add_listener();
			true
		} else {
			false
		}
	}

	fn view(&self) -> Html {
		let send_chat = self.link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::Send
		});
		let chat_change = self.link.callback(|e: InputData|
			Msg::Change(Box::new(move |c| c.composing = e.value))
		);
		let send_command = self.link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::SendCommand
		});
		let command_change = self.link.callback(|e: InputData|
			Msg::Change(Box::new(move |c| c.composing_command = e.value))
		);

		ConnectionService::with_ready_unwrap(&self.con, |c| {
			html! {
				<div class="chat">
					{ self.view_messages() }
					<form class="chat-form" onsubmit=send_chat>
						<input class="input" name="message" type="text"
							value=&c.composing
							oninput=chat_change />
						<button class="button" name="send" type="submit">
							{ "Send" }
						</button>
					</form>
					<form class="chat-form" onsubmit=send_command>
						<input class="input" name="message" type="text"
							value=&c.composing_command
							oninput=command_change />
						<button class="button" name="send" type="submit">
							{ "Send Command" }
						</button>
					</form>
				</div>
			}
		})
	}
}

impl Chat {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut(&self.con, |con| {
			let new_msg = self.link.callback(|_| Msg::NewMessage);
			let channel_msg = self.link.callback(|_| Msg::SetChatToChannel);
			con.packet_listeners.insert("chat".into(), Box::new(move |_, msg| {
				if msg.name() == "notifytextmessage" {
					new_msg.emit(());
				} else if msg.name() == "channellistfinished" {
					channel_msg.emit(());
				}
			}));
		}, || panic!("Should be in connected state"));
	}

	fn request_messages(&mut self, start: Option<(NaiveDateTime, i64)>) {
		let mut fetch = FetchService::new();
		let msg_request = MessagesRequest {
			chat: self.chat.clone(),
			start,
		};

		let request = Request::post(&format!("{}/messages", crate::Model::get_http_domain()))
			.body(MsgPack(&msg_request))
			.unwrap();
		self.fetch_task = Some(fetch.fetch_binary(request, self.link
			.callback(|resp: Response<MsgPack<Result<Vec<Message>, Error>>>| {
				match resp.into_body().0 {
					Ok(r) => Msg::GotMessages(r),
					Err(e) => {
						// TODO Display error message
						log::error!("Failed to fetch messages: {:?}", e);
						Msg::Ignore
					}
				}
			})));
	}

	fn view_message(&self, msg: &Message) -> Html {
		html! {
			<li>
				<article class="media">
					<figure class="media-left">
						<p class="image is-32x32">
							<img class="round" src="128x128.png" />
						</p>
					</figure>
					<div class="media-content">
						<div class="content">
							<p>
								<strong>{ msg.client_name.as_ref().or(msg.invoker_name.as_ref()).unwrap() }</strong>
								<br />
								{ &msg.content }
							</p>
						</div>
					</div>
					// <div class="media-right">
					// 	<button class="delete"></button>
					// </div>
				</article>

				// <div class="author",>{ &msg.invoker.name }</div>
				// <div class="chat-message",>{ &msg.message }</div>
			</li>
		}
	}

	fn view_messages(&self) -> Html {
		html! {
			<ul class="chat-messages",>
				{ for self.messages.iter().rev()
					.map(|m| self.view_message(m)) }
				<span class="chat-end"></span>
			</ul>
		}
		// TODO Use document.querySelectorAll('.chat-end')[0].scrollIntoView({behavior: "smooth"})
	}
}
