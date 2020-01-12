use std::borrow::Cow;

use chrono::NaiveDateTime;
use failure::{format_err, Error};
use qint_shared::{ChatId, ChatType, MESSAGES_LIMIT, MessagesRequest};
use qint_shared::models::Message;
use slog::error;
use stdweb::{js, Value};
use stdweb::web::event::IEvent;
use ts_bookkeeping::{ChannelId, IconHash, MessageTarget, UidRef};
use ts_bookkeeping::events::{Event, PropertyId, PropertyValue};
use tsproto_packets::packets::{Direction, Flags, OutCommand, OutPacket, PacketType};
use yew::format::MsgPack;
use yew::html;
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use crate::CLIENT_ICON;
use crate::connection_service::*;
use crate::controls::icon::Icon;

pub struct Chat {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: SelectedChat,
	/// If `true`, all messages are loaded and there are no older messages available.
	/// If `false`, we can still load older messages.
	all_loaded: bool,
	fetch_task: Option<FetchTask>,
	set_chat: Callback<SelectedChat>,

	send_chat: Callback<SubmitEvent>,
	chat_change: Callback<InputData>,
	send_command: Callback<SubmitEvent>,
	command_change: Callback<InputData>,
	scroll_down: Callback<()>,

	messages: Vec<Message>,
}

pub enum Msg {
	Ignore,
	/// The user changed the content of the text input
	ChatChange(String),
	/// The user changed the content of the command text input
	CommandChange(String),
	/// Requested messages arrived arrived from the proxy
	GotMessages(Vec<Message>),
	/// A new message arrived
	NewMessage(MessageTarget),
	/// Set the chat to our channel when connecting to a server.
	///
	/// We can do this after we known in which channel our client is.
	/// If the user already changed the chat to a channel or client chat, we do
	/// nothing.
	SetChatToChannel(ChannelId),
	/// Our client changed channel.
	///
	/// If the chat displayed the old channel, switch to the new channel.
	ChannelChanged { old: ChannelId, new: ChannelId },
	/// When the user clicked on 'Send'
	Send,
	/// When the user clicked on 'Send Command'
	SendCommand,
	/// Scroll to the end of the chat
	ScrollDown,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
	#[props(required)]
	pub chat: SelectedChat,
	#[props(required)]
	pub set_chat: Callback<SelectedChat>,
}

impl Component for Chat {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let send_chat = link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::Send
		});
		let chat_change = link.callback(|e: InputData|
			Msg::ChatChange(e.value)
		);
		let send_command = link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::SendCommand
		});
		let command_change = link.callback(|e: InputData|
			Msg::CommandChange(e.value)
		);
		let scroll_down = link.callback(|()| Msg::ScrollDown);

		let mut res = Self {
			link,
			con: props.connection,
			chat: props.chat.clone(),
			all_loaded: false,
			fetch_task: None,
			set_chat: props.set_chat,

			send_chat,
			chat_change,
			send_command,
			command_change,
			scroll_down,

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
				self.fetch_task = None;
				self.all_loaded = msgs.len() < MESSAGES_LIMIT;
				if msgs.is_empty() {
					return true;
				}
				if self.messages.is_empty() {
					self.messages = msgs;
					self.check_load_messages();
					return true;
				}

				if msgs[0] >= self.messages[0] {
					// Prepend msgs
					if let Ok(i) = msgs.binary_search(&self.messages[0]) {
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
				self.check_load_messages();
				true
			}
			Msg::NewMessage(from) => {
				let target = match self.get_message_target() {
					Ok(r) => r,
					Err(e) => {
						// TODO Show notification
						ConnectionService::with_unwrap(&self.con, |con| {
							error!(con.logger, "Failed to get message target";
								"error" => ?e);
							Some(())
						}, "Connection not found for logger");
						return false;
					}
				};
				if from == target {
					self.request_messages(None);
					true
				} else {
					false
				}
			}
			Msg::SetChatToChannel(channel) => {
				if self.chat.chat_type == ChatType::Server {
					self.set_chat.emit(SelectedChat {
						chat_type: ChatType::Channel(channel.0),
						client: None,
					});
				}
				false
			}
			Msg::ChannelChanged { old, new } => {
				if self.chat.chat_type == ChatType::Channel(old.0) {
					self.set_chat.emit(SelectedChat {
						chat_type: ChatType::Channel(new.0),
						client: None,
					});
				}
				false
			}
			Msg::Send => {
				let message_target = match self.get_message_target() {
					Ok(r) => r,
					Err(e) => {
						// TODO Show notification
						ConnectionService::with_unwrap(&self.con, |con| {
							error!(con.logger, "Failed to get message target";
								"error" => ?e);
							Some(())
						}, "Connection not found for logger");
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
				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
					let mut packet = OutPacket::new_with_dir(Direction::C2S,
						Flags::empty(), PacketType::Command);
					let static_args = std::iter::empty();
					let list_args = std::iter::empty();
					OutCommand::new_into::<&'static str, Cow<str>, &'static str, Cow<str>, _, _, std::iter::Empty<_>>(
						&c.composing_command, static_args, list_args, packet.data_mut());

					c.composing_command.clear();
					Some(packet)
				}, "Failed to send command");
				true
			}
			Msg::ScrollDown => {
				js! { @(no_return)
					document.querySelectorAll(".chat-end")[0].scrollIntoView({behavior: "smooth"});
				};
				false
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let mut changed = false;
		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&props.connection, |con| {
				con.event_listeners.remove("chat");
			}, || {});

			self.con = props.connection.clone();
			self.add_listener();

			self.set_chat(props.chat.clone());

			changed = true;
		}

		if self.chat != props.chat && self.con == props.connection {
			self.set_chat(props.chat);
			changed = true;
		}

		if self.set_chat != props.set_chat {
			self.set_chat = props.set_chat;
		}

		changed
	}

	fn view(&self) -> Html {
		ConnectionService::with_ready_unwrap(&self.con, |c| {
			let msg = c.composing.get(&self.chat).map(String::as_str)
				.unwrap_or_default();
			html! {
				<div class="chat">
					{ self.view_messages() }
					<form class="chat-form" onsubmit=&self.send_chat>
						<input class="input" name="message" type="text"
							value=msg
							oninput=&self.chat_change />
						<button class="button" name="send" type="submit">
							{ "Send" }
						</button>
					</form>
					<form class="chat-form" onsubmit=&self.send_command>
						<input class="input" name="message" type="text"
							value=&c.composing_command
							oninput=&self.command_change />
						<button class="button" name="send" type="submit">
							{ "Send Command" }
						</button>
					</form>
				</div>
			}
		})
	}

	fn destroy(&mut self) {
		ConnectionService::with_mut(&self.con, |con| {
			con.event_listeners.remove("chat");
		}, || {});
	}
}

impl Chat {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut(&self.con, |con| {
			let new_msg = self.link.callback(|from| Msg::NewMessage(from));
			let chat_to_channel = self.link.callback(|c| Msg::SetChatToChannel(c));
			let channel_changed = self.link.callback(|(old, new)|
				Msg::ChannelChanged { old, new });
			con.event_listeners.insert("chat".into(), Box::new(move |con, events| {
				for e in events {
					match e {
						Event::PropertyAdded { id, .. } => {
							if let FrontendConnectionState::Connected(con) = &con.state {
								if let PropertyId::Client(id) = id {
									if *id == con.con.own_client {
										// We can switch to our own channel
										chat_to_channel.emit(con.con.clients[id].channel);
									}
								}
							}
						}
						Event::PropertyChanged { id, old, .. } => {
							// On channel change
							if let FrontendConnectionState::Connected(con) = &con.state {
								if let PropertyId::ClientChannel(id) = id {
									if *id == con.con.own_client {
										if let PropertyValue::ChannelId(chan) = old {
											channel_changed.emit((*chan,
												con.con.clients[id].channel));
										}
									}
								}
							}
						}
						Event::Message { from, .. } => {
							new_msg.emit(from.clone());
						}
						_ => {}
					}
				}
			}));
		}, || panic!("Should be in connected state"));
	}

	pub fn set_chat(&mut self, chat: SelectedChat) {
		self.all_loaded = false;
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

	/// Check if more messages should be loaded.
	fn check_load_messages(&mut self) {
		if self.all_loaded {
			return;
		}

		if let Value::Bool(true) = js! {
			const elements = document.querySelectorAll(".chat-messages");
			if (elements.length === 0) {
				return false;
			}

			const element = elements[0];
			// Less than 10% of the screen height is left as a buffer
			return element.scrollTop / element.clientHeight <= 0.1;
		} {
			// Need more messages
			let start = if let Some(msg) = self.messages.last() {
				Some((msg.time, msg.id))
			} else {
				None
			};
			self.request_messages(start);
		}
	}

	fn get_message_target(&self) -> Result<MessageTarget, Error> {
		match &self.chat.chat_type {
			ChatType::Server => Ok(MessageTarget::Server),
			ChatType::Channel(_) => Ok(MessageTarget::Channel),
			ChatType::Client(_) => {
				if let Some(id) = self.chat.client {
					Ok(MessageTarget::Client(id))
				} else {
					Err(format_err!("Cannot send a message without a client id"))
				}
			}
			ChatType::Poke(_) => {
				Err(format_err!("Poke is not valid for the chat"))
			}
		}
	}

	fn view_message_header(&self, msg: &Message) -> Html {
		let icon = if msg.client_avatar.as_ref().map(|a| !a.is_empty()).unwrap_or_default() {
			Icon::client_avatar(&self.con, UidRef(&base64::encode(msg.invoker.as_ref().unwrap())))
		} else if let Some(icon) = msg.client_icon {
			Icon::icon_hash(&self.con, IconHash(icon as u32))
				.unwrap_or_else(|| Icon::mdi_icon(CLIENT_ICON))
		} else {
			Icon::mdi_icon(CLIENT_ICON)
		};

		html! {
			<>
				<div class="invoker-icon">
					{ icon }
				</div>
				<div class="invoker-name">
					{ msg.client_name.as_ref().or(msg.invoker_name.as_ref()).unwrap() }
				</div>
			</>
		}
	}

	fn view_message(&self, msg: &Message) -> Html {
		html! {
			<>
				<div class="message-time">
					<span title={ format!("{}", msg.get_date_time().format("%Y-%m-%d %H:%M, UTC%:z")) }>
						{ msg.get_date_time().format("%H:%M") }
					</span>
				</div>
				<div class="message-content">
					{ &msg.content }
				</div>
			</>
		}
	}

	fn view_message_group(&self, group: &[&Message]) -> Html {
		html! {
			<li>
				{ self.view_message_header(group[0]) }
				{ for group.iter().map(|m| self.view_message(m)) }
			</li>
		}
	}

	fn view_messages(&self) -> Html {
		// Check if we are at bottom of chat window, if so, scroll to the bottom
		// after adding new messages.
		// https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollHeight
		if let Value::Bool(true) = js! {
			const elements = document.querySelectorAll(".chat-messages");
			if (elements.length === 0) {
				return false;
			}

			const element = elements[0];
			return element.scrollHeight - element.scrollTop === element.clientHeight;
		} {
			self.scroll_down.emit(());
		}

		// Display loading spinner when there is an active fetch task
		let spinner = if self.fetch_task.is_some() {
			html! {
				<div class="is-loading" style="color: gray; font-style: italic; text-align: center;">{ "Loading…" }</div>
			}
		} else {
			html!{}
		};

		// Group by same author messages following each other
		let mut groups: Vec<Vec<&Message>> = Vec::new();
		for m in self.messages.iter().rev() {
			if groups.last().map(|l| {
				let l = l[0];
				l.invoker == m.invoker && l.invoker_name == m.invoker_name
			}).unwrap_or_default() {
				groups.last_mut().unwrap().push(m);
			} else {
				groups.push(vec![m]);
			}
		}

		html! {
			<ul class="chat-messages">
				{ spinner }
				{ for groups.iter().map(|g| self.view_message_group(g)) }
				<span class="chat-end"></span>
			</ul>
		}
	}
}
