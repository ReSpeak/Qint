use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::channel::oneshot;
use futures::prelude::*;
use qint_shared::*;
use slog::{error, info, Logger};
use ts_bookkeeping::{ChannelId, Invoker, TsError};
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::messages::s2c::{InCommandError, InMessageTrait, InTextMessage};
use tsproto_packets::packets::{InCommand, OutPacket};
use yew::html;
use yew::prelude::*;
use yew::services::websocket::WebSocketTask;

use crate::{Model, Msg};
use crate::connection::ConnectionMsg;
use channel_tree::ChannelTree;
use chat::Chat;

mod channel_tree;
mod chat;

pub struct Connected {
	logger: Logger,
	connection: Connection,

	channel_tree: ChannelTree,
	chat: Chat,
}

pub enum ConnectedMsg {
	Packet(InCommandMsg),
	ChangeChannel(ChannelId),
	SetTalking(bool),
	Chat(chat::ChatMsg),
}

impl Connected {
	pub fn new(connection: Connection, logger: Logger) -> Self {
		let logger2 = logger.clone();
		let mut con = Connected {
			logger,
			connection,

			channel_tree: Default::default(),
			chat: Chat::new(logger2),
		};

		let cmd = con.connection.server.set_subscribed(true);
		let logger = con.logger.clone();
		stdweb::spawn_local(con.message_handler.send_message(cmd).map(move |r| {
			if let Err(e) = r {
				error!(logger, "Failed to subscribe"; "error" => ?e);
			}
		}));

		con
	}

	fn update_internal(&mut self, msg: ConnectedMsg) -> ShouldRender {
		match msg {
			ConnectedMsg::Packet(packet) => {
				let packet: InCommand = packet.into();
				let mut res = false;
				// Handle return codes
				if packet.name() == "error" {
					let error = match InCommandError::new(&packet) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Failed to parse error command"; "error" => ?e);
							return false;
						}
					};
					let error = error.iter().next().unwrap();

					if let Some(code) =
						error.return_code.as_ref().and_then(|c| c.parse().ok())
					{
						if let Some(return_sender) =
							self.message_handler.return_codes.remove(&code)
						{
							// Ignore if sending fails
							let _ = return_sender.send(error.id);
						}
					} else {
						error!(self.logger, "Got error without return code"; "error" => ?error.id)
					}
					// Packet contains only handled return codes
					return false;
				} else if packet.name() == "notifytextmessage" {
					let msg = match InTextMessage::new(&packet) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Failed to parse message command"; "error" => ?e);
							return false;
						}
					};
					let msg = msg.iter().next().unwrap();

					let msg = chat::ChatMsg::NewMessage(chat::Message {
						invoker: Invoker {
							id: msg.invoker_id,
							name: msg.invoker_name.to_string(),
							uid: msg.invoker_uid.as_ref().map(|u| u.clone().into()),
						},
						message: msg.message.to_string(),
					});
					res |= self.chat.update(&self.connection, &mut self.message_handler, msg);
				}

				// Bookkeeping
				match self.connection.handle_command(&packet) {
					Ok(events) => {
						// TODO
						info!(self.logger, "Got events"; "events" => ?events);
						true
					}
					Err(e) => {
						error!(self.logger, "Failed to handle command"; "error" => ?e);
						res
					}
				}
			}
			ConnectedMsg::ChangeChannel(id) => {
				let cmd = self.connection.server.clients[&self.connection.own_client]
					.set_channel(id);
				let logger = self.logger.clone();
				stdweb::spawn_local(self.message_handler.send_message(cmd).map(move |r| {
					if let Err(e) = r {
						// TODO Display popup
						error!(logger, "Failed to change channel"; "error" => ?e);
					}
				}));
				false
			}
			ConnectedMsg::SetTalking(talk) => {
				self.channel_tree.is_talking = talk;
				true
			}
			ConnectedMsg::Chat(msg) => {
				self.chat.update(&self.connection, &mut self.message_handler, msg)
			}
		}
	}

	pub fn update(&mut self, msg: ConnectedMsg) -> (Vec<OutPacket>, ShouldRender) {
		let res = self.update_internal(msg);
		(mem::replace(&mut self.message_handler.send_packets, Vec::new()), res)
	}
}

impl Renderable<Model> for Connected {
	fn view(&self) -> Html<Model> {
		html! {
			<div class="connected-container",>
				{ self.channel_tree.view(&self.connection) }
				{ self.chat.view(&self.connection) }
			</div>
		}
	}
}

impl Into<Msg> for ConnectedMsg {
	fn into(self) -> Msg { Msg::Connection(ConnectionMsg::Connected(self)) }
}
