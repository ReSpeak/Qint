use core::cmp::Ordering;
use std::borrow::Cow;

use chrono::NaiveDateTime;
use failure::{format_err, Error};
use qint_shared::{ChatId, ChatType, MESSAGES_LIMIT, MessagesRequest};
use qint_shared::models::{Message, MessageStatus};
use slog::{error, info};
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
use crate::html_util::{data_hash_to_color};
use crate::bulma_icon;
use crate::connected::yew_markdown::markdown;

pub struct Chat {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: SelectedChat,
	/// If `true`, all messages are loaded and there are no older messages available.
	/// If `false`, we can still load older messages.
	all_loaded: bool,
	/// True when new messages are in view which need to be prostprocessed
	new_messages: bool,
	fetch_task: Option<FetchTask>,
	set_chat: Callback<SelectedChat>,

	send_chat: Callback<SubmitEvent>,
	chat_change: Callback<InputData>,
	chat_key_down: Callback<KeyDownEvent>,
	send_command: Callback<SubmitEvent>,
	command_change: Callback<InputData>,
	/// Called whenever new messages get added into the view list of the active chat.
	/// Manages html postprocessing like highlight, katex or scrolling into view.
	chat_postprocess: Callback<()>,
	toggle_raw: Callback<ClickEvent>,
	toggle_view_orig: Callback<ClickEvent>,

	/// Displayed messages, sorted ascending by time.
	messages: Vec<UiChatMessage>,
}

#[derive(Clone, Debug)]
pub struct UiChatMessage {
	data: Message,
	rendered_markdown: Html,
	is_edit: bool,
	show_original: bool,
}

pub enum Msg {
	Ignore,
	/// The user changed the content of the text input
	ChatChange(String),
	/// The user changed the content of the command text input
	CommandChange(String),
	/// Requested messages arrived arrived from the proxy
	GotMessages(Vec<UiChatMessage>),
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
	/// After new Messages have been added into the view
	ChatPostprocess,
	ToggleShowOriginal,
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
		let chat_key_down = link.callback(|e: KeyDownEvent|
			if e.key() == "Enter" && !e.shift_key() && !e.ctrl_key() {
				// Send message on enter
				e.prevent_default();
				Msg::Send
			} else {
				Msg::Ignore
			}
		);
		let send_command = link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::SendCommand
		});
		let command_change = link.callback(|e: InputData|
			Msg::CommandChange(e.value)
		);
		let chat_postprocess = link.callback(|()| Msg::ChatPostprocess);
		let toggle_raw = Callback::from(|e: ClickEvent| {
			js! { @(no_return) parent(@{e.target()}, ".message-content").classList.toggle("view_raw"); }
		});
		let toggle_view_orig = link.callback(|_: ClickEvent| {
			Msg::ToggleShowOriginal
		});

		let mut res = Self {
			link,
			con: props.connection,
			chat: props.chat.clone(),
			all_loaded: false,
			new_messages: false,
			fetch_task: None,
			set_chat: props.set_chat,

			send_chat,
			chat_change,
			chat_key_down,
			send_command,
			command_change,
			chat_postprocess,
			toggle_raw,
			toggle_view_orig,

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
				self.update_chat_height();
				false
			}
			Msg::CommandChange(s) => {
				ConnectionService::with_mut_ready_unwrap(&self.con, |con| {
					con.composing_command = s;
				});
				true
			}
			Msg::GotMessages(mut msgs) => {
				js! { console.log("ON GotMessages"); };
				self.fetch_task = None;
				self.all_loaded = msgs.len() < MESSAGES_LIMIT;
				if msgs.is_empty() {
					return true;
				}
				if self.messages.is_empty() {
					self.messages = msgs;
					self.new_messages = true;
					self.check_load_messages();
					return true;
				}
				let logger = ConnectionService::with_unwrap(&self.con,
					|con| Some(con.logger.clone()), "Failed to find connection");

				info!(logger, "Received messages"; "new" => ?msgs, "current" => ?self.messages);
				if msgs.last().unwrap() >= self.messages.last().unwrap() {
					// Messages are more recent, append msgs
					if let Ok(i) = self.messages.binary_search(&msgs[0]) {
						info!(logger, "Appending, found"; "at" => i);
						self.messages.truncate(i);
						self.messages.append(&mut msgs);
					} else {
						info!(logger, "Gap, replacing");
						// There may be a gap between msgs and self.messages,
						// so we just replace them.
						self.messages = msgs;
					}
				} else {
					// Messages are older, prepend msgs
					if let Ok(i) = msgs.binary_search(&self.messages[0]) {
						info!(logger, "Prepending, found"; "at" => i);
						msgs.truncate(i);
						msgs.append(&mut self.messages);
						self.messages = msgs;
					} else {
						info!(logger, "Prepending");
						msgs.append(&mut self.messages);
						self.messages = msgs;
					}
				}
				self.new_messages = true;
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
				self.update_chat_content();
				self.update_chat_height();
				false
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
					// move last chat into view
					document.querySelector(".chat-end").scrollIntoView({behavior: "smooth"});
				};
				false
			}
			Msg::ChatPostprocess => {
				if !self.new_messages {
					js! { console.log("Already processed"); };
					return false;
				}
				self.new_messages = false;
				js! { @(no_return)
					console.log("ON post proc");
					// katex
					document.querySelectorAll(".chat-messages .latex_proc").forEach(elem => {
						elem.classList.remove("latex_proc");
						console.log("Processed");
						window.renderMathInElement(elem, {
							errorCallback: (err) => { console.log("Failed to LaTeX", err); }
						});
					});
					// highlight
					document.querySelectorAll(".chat-messages pre code:not(.hljs)").forEach(elem => {
						elem.classList.remove("highlight_proc");
						window.highlightBlock(elem);
					});
					// move last chat into view
					document.querySelector(".chat-end").scrollIntoView(/*{behavior: "smooth"}*/);
				};
				false
			}
			Msg::ToggleShowOriginal => {

				true
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

		js! { console.log("ON change"); };
		self.chat_postprocess.emit(());

		changed
	}

	fn view(&self) -> Html {
		ConnectionService::with_ready_unwrap(&self.con, |c| {
			html! {
				<div class="chat">
					{ self.view_messages() }
					<form class="chat-form" onsubmit=&self.send_chat>
						<textarea class="input auto_height" name="message"
							oninput=&self.chat_change
							onkeydown=&self.chat_key_down>
						</textarea>
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
						Event::Message { target, .. } => {
							new_msg.emit(target.clone());
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
		self.update_chat_content();
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
					Ok(r) => Msg::GotMessages(r.into_iter().map(|m| UiChatMessage::new(m)).collect()),
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
			let start = if let Some(UiChatMessage{ data: msg, .. }) = self.messages.first() {
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

		let user_name = msg.client_name.as_ref().or(msg.invoker_name.as_ref()).unwrap();
		let user_color = if let Some(ref uid) = msg.invoker {
			data_hash_to_color(uid)
		} else {
			data_hash_to_color(user_name.as_bytes())
		};
		html! {
			<>
				<div class="invoker-icon">
					{ icon }
				</div>
				<div class="invoker-name has-text-weight-bold" style={ user_color }>
					{ user_name }
				</div>
			</>
		}
	}

	fn view_message_group(&self, group: &[&UiChatMessage]) -> Html {
		html! {
			<li class="message-group">
				{ self.view_message_header(&group[0].data) }
				{ for group.iter().map(|m| self.view_message(m)) }
			</li>
		}
	}

	fn view_messages(&self) -> Html {
		// Check if we are at bottom of chat window, if so, scroll to the bottom
		// after adding new messages.
		// https://developer.mozilla.org/en-US/docs/Web/API/Element/scrollHeight
		// if let Value::Bool(true) = js! {
		// 	const elements = document.querySelectorAll(".chat-messages");
		// 	if (elements.length === 0) {
		// 		return false;
		// 	}

		// 	const element = elements[0];
		// 	return element.scrollHeight - element.scrollTop === element.clientHeight;
		// } {
		// 	self.chat_postprocess.emit(());
		// }

		self.chat_postprocess.emit(());
		//self.send_message(Msg::ChatPostprocess);
		js! { console.log("ON render"); };

		// Display loading spinner when there is an active fetch task
		let spinner = if self.fetch_task.is_some() {
			html! {
				<div class="is-loading" style="color: gray; font-style: italic; text-align: center;">{ "Loading…" }</div>
			}
		} else {
			html!{}
		};

		// Group by same author messages following each other
		let mut groups: Vec<Vec<&UiChatMessage>> = Vec::new();
		for m in self.messages.iter() {
			if groups.last().map(|l| {
				let l = &l[0].data;
				l.invoker == m.data.invoker && l.invoker_name == m.data.invoker_name
			}).unwrap_or_default() {
				let last = groups.last_mut().unwrap();
				if m.is_edit && !m.show_original { last.pop(); }
				last.push(m);
			} else {
				groups.push(vec![m]);
			}
		}

		html! {
			<ul class="chat-messages">
				{ spinner }
				{ for groups.iter().map(|g| self.view_message_group(g)) }
				<div class="chat-end"></div>
			</ul>
		}
	}

	fn view_message(&self, ui_msg: &UiChatMessage) -> Html {
		let msg = &ui_msg.data;
		html! {
			<div class="message-row">
				<div class="message-time">
					<span title={ format!("{}", msg.get_date_time().format("%Y-%m-%d %H:%M, UTC%:z")) }>
						{ msg.get_date_time().format("%H:%M") }
					</span>
				</div>
				<div class=cl![
						"message-content",
						("message-sending", msg.status == MessageStatus::Sending),
						("message-error", msg.status == MessageStatus::Error)]>
					<div class="content message-rendered latex_proc">
						{ ui_msg.rendered_markdown.clone() }
					</div>
					<div class="message-raw">
						<pre>
							{ &msg.content }
						</pre>
					</div>
					<div class="tool-buttons" >
						<div class="tool-buttons-wrap buttons has-addons" >
							<button class="button is-small is-rounded">
								{ bulma_icon!("pencil") }
							</button>
							<button class="button is-small is-rounded">
								{ bulma_icon!("format-quote-close") }
							</button>
							<button class="button is-small is-rounded" onclick=&self.toggle_raw>
								{ bulma_icon!(="🥩") }
							</button>
						</div>
					</div>
				</div>
			</div>
		}
	}

	fn update_chat_content(&self) {
		ConnectionService::with_ready_unwrap(&self.con, |c| {
			let msg = c.composing.get(&self.chat).map(String::as_str)
				.unwrap_or_default();
			js! {
				document.querySelectorAll(".auto_height").forEach(e => {
					e.value = @{msg};
				});
			}
		});
	}

	fn update_chat_height(&self) {
		js! {
			document.querySelectorAll(".auto_height").forEach(e => {
				e.style.height = "5px";
				if (e.scrollHeight < 300) {
					e.style.height = e.scrollHeight + "px";
					e.style.overflowY = "hidden";
				} else {
					e.style.height = "300px";
					e.style.overflowY = "auto";
				}
			});
		}
	}
}

impl UiChatMessage {
	const EDIT_PREFIX: &'static str = "*EDIT*";

	pub fn new(msg: Message) -> UiChatMessage {
		let is_edit = false && msg.content.starts_with(Self::EDIT_PREFIX);
		let content_text = if is_edit { &msg.content[Self::EDIT_PREFIX.len()..] } else { &msg.content };
		let rendered_markdown = markdown(content_text);

		UiChatMessage {
			data: msg,
			rendered_markdown,
			is_edit,
			show_original: false
		}
	}
}

impl Ord for UiChatMessage {
	fn cmp(&self, other: &Self) -> Ordering {
		self.data.cmp(&other.data)
	}
}

impl PartialOrd for UiChatMessage {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(other))
	}
}

impl PartialEq for UiChatMessage {
	fn eq(&self, other: &Self) -> bool {
		self.data == other.data
	}
}
impl Eq for UiChatMessage {}
