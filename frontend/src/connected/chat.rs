use chrono::NaiveDateTime;
use failure::Error;
use qint_shared::{ChatId, ChatType, MESSAGES_LIMIT, MessagesRequest};
use qint_shared::models::Message;
use slog::error;
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
	chat: SelectedChat,
	/// If `true`, all messages are loaded and there are no older messages available.
	/// If `false`, we can still load older messages.
	all_loaded: bool,
	// TODO More than one
	// TODO Display loading spinner when there is an active fetch task
	fetch_task: Option<FetchTask>,

	messages: Vec<Message>,
}

pub enum Msg {
	Ignore,
	ChatChange(String),
	CommandChange(String),
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
	#[props(required)]
	pub chat: SelectedChat,
}

impl Component for Chat {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		// On channel change:
		// TODO Update chat id to new channel if current id points to current channel
		// TODO Add this as event listener

		let mut res = Self {
			link,
			con: props.connection,
			chat: props.chat.clone(),
			all_loaded: false,
			fetch_task: None,

			messages: Vec::new(),
		};
		res.add_listener();
		res.set_chat(props.chat);

		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::ChatChange(s) => {
				ConnectionService::with_mut_ready_unwrap(&self.con, |con| {
					con.composing.insert(self.chat.clone(), s);
				});
				true
			}
			Msg::CommandChange(s) => {
				ConnectionService::with_mut_ready_unwrap(&self.con, |con| {
					con.composing_command = s;
				});
				true
			}
			Msg::GotMessages(mut msgs) => {
				self.all_loaded = msgs.len() < MESSAGES_LIMIT;
				if msgs.is_empty() {
					return false;
				}
				if self.messages.is_empty() {
					self.messages = msgs;
					return true;
				}

				if msgs[0] <= self.messages[0] {
					// Prepend msgs
					if let Ok(i) = msgs.binary_search(&self.messages[0]) {
						ConnectionService::with_unwrap(&self.con, |con| {
							slog::debug!(con.logger, "Prepend msgs"; "i" => i, "messages[0]" => ?&self.messages[0], "msgs[0]" => ?&msgs[0]);
							Some(())
						}, "Connection not found for logging");
						msgs.truncate(i);
						msgs.append(&mut self.messages);
						self.messages = msgs;
					} else {
						// There may be a gap between msgs and self.messages,
						// so we just replace them.
						self.messages = msgs;
					}
				} else {
					// Append msgs
					self.messages.append(&mut msgs);
				}
				true
			}
			Msg::NewMessage => {
				self.request_messages(None);
				false
			}
			Msg::SetChatToChannel => {
				// TODO Set chat id to channel
				/*self.set_chat(ChatType::Channel(ConnectionService::with_mut_ready_unwrap(&self.con, |c| {
					c.con.clients[&c.con.own_client].channel.0
				})));*/
				false
			}
			Msg::Send => {
				let message_target = match &self.chat.chat_type {
					ChatType::Server => MessageTarget::Server,
					ChatType::Channel(_) => MessageTarget::Channel,
					ChatType::Client(_) => {
						if let Some(id) = self.chat.client {
							MessageTarget::Client(id)
						} else {
							// TODO Show notification
							ConnectionService::with_unwrap(&self.con, |con| {
								error!(con.logger, "Cannot send a message without a client id");
								Some(())
							}, "Connection not found for sending message");
							return false;
						}
					}
					ChatType::Poke(_) => {
						// TODO Show notification
						ConnectionService::with_unwrap(&self.con, |con| {
							error!(con.logger, "Poke is not valid for the chat");
							Some(())
						}, "Connection not found for sending message");
						return false;
					}
				};

				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
					let cmd = c.con.send_message(message_target,
						&c.composing.get(&self.chat).map(String::as_str)
						.unwrap_or_default());
					c.composing.remove(&self.chat);
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
		let res = if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&props.connection, |con| {
				con.packet_listeners.remove("chat");
			}, || {});

			self.con = props.connection.clone();
			self.add_listener();

			self.set_chat(props.chat.clone());

			true
		} else {
			false
		};

		if self.chat != props.chat && self.con == props.connection {
			self.set_chat(props.chat);
		}
		res
	}

	fn view(&self) -> Html {
		let send_chat = self.link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::Send
		});
		let chat_change = self.link.callback(|e: InputData|
			Msg::ChatChange(e.value)
		);
		let send_command = self.link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::SendCommand
		});
		let command_change = self.link.callback(|e: InputData|
			Msg::CommandChange(e.value)
		);

		ConnectionService::with_ready_unwrap(&self.con, |c| {
			let msg = c.composing.get(&self.chat).map(String::as_str)
				.unwrap_or_default();
			html! {
				<div class="chat">
					{ self.view_messages() }
					<form class="chat-form" onsubmit=send_chat>
						<input class="input" name="message" type="text"
							value=msg
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

	pub fn set_chat(&mut self, chat: SelectedChat) {
		self.chat = chat;
		self.messages.clear();
		self.request_messages(None);
	}

	fn request_messages(&mut self, start: Option<(NaiveDateTime, i64)>) {
		let server = ConnectionService::with_ready_unwrap(&self.con, |con|
			con.key.clone()
		);
		let msg_request = MessagesRequest {
			chat: ChatId {
				server,
				chat_type: self.chat.chat_type.clone(),
			},
			start,
		};

		let mut fetch = FetchService::new();
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
