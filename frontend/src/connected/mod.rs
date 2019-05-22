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

use crate::{Model, Msg};
use crate::connection::ConnectionMsg;
use channel_tree::ChannelTree;
use chat::Chat;

mod channel_tree;
mod chat;

pub struct Connected {
	logger: Logger,
	connection: Connection,
	message_handler: MessageHandler,

	channel_tree: ChannelTree,
	chat: Chat,
}

pub enum ConnectedMsg {
	Packet(InCommandMsg),
	ChangeChannel(ChannelId),
	Chat(chat::ChatMsg),
}

pub struct MessageHandler {
	return_codes: HashMap<usize, oneshot::Sender<TsError>>,
	cur_return_code: AtomicUsize,
	send_packets: Vec<OutPacket>,
}

impl MessageHandler {
	/// Get a return code and a receiver which gets notified when an answer is
	/// received.
	pub(crate) fn get_return_code(
		&mut self,
	) -> (usize, oneshot::Receiver<TsError>) {
		let code = self.cur_return_code.fetch_add(1, Ordering::Relaxed);
		let (send, recv) = oneshot::channel();
		// The receiver should fail when the sender is dropped, but usize should
		// be enough for every platform.
		self.return_codes.insert(code, send);
		(code, recv)
	}

	/// Adds a `return_code` to the command and returns if the corresponding
	/// answer is received. If an error occurs, the future will return an error.
	#[must_use = "futures do nothing unless polled"]
	pub fn send_message(
		&mut self,
		mut packet: OutPacket,
	) -> impl Future<Output = Result<(), TsError>>
	{
		// Store waiting in HashMap<usize (return code), oneshot::Sender>
		// The packet handler then sends a result to the sender if the answer is
		// received.

		let (code, recv) = self.get_return_code();
		// Add return code
		packet
			.data_mut()
			.extend_from_slice(" return_code=".as_bytes());
		packet
			.data_mut()
			.extend_from_slice(code.to_string().as_bytes());

		// Send a message and wait until we get an answer for the return code
		self.send_packets.push(packet);
		recv.map_err(|_| TsError::Undefined)
			.and_then(|r| {
				if r == TsError::Ok {
					future::ok(())
				} else {
					future::err(r.into())
				}
			})
	}
}

impl Connected {
	pub fn new(connection: Connection, logger: Logger) -> Self {
		let logger2 = logger.clone();
		let mut con = Connected {
			logger,
			connection,
			message_handler: MessageHandler {
				return_codes: Default::default(),
				cur_return_code: AtomicUsize::new(0),
				send_packets: Default::default(),
			},

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
