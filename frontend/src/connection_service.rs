use std::cell::RefCell;
use std::collections::HashMap;

use failure::{format_err, Error};
use futures::channel::oneshot;
use futures::prelude::*;
use qint_shared::*;
use ts_bookkeeping::{Invoker, TsError, Uid};
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::events::Event;
use ts_bookkeeping::messages::s2c::{InCommandError, InMessage, InMessages, InMessageTrait, InTextMessage};
use tsproto_packets::packets::{InCommand, OutPacket};
use slog::{error, o, Logger};
use yew::ShouldRender;
use yew::format::MsgPack;
use yew::services::websocket::WebSocketTask;

thread_local! {
	/// Each tab is a connection
	pub static CONNECTIONS: RefCell<HashMap<ConnectionId, FrontendConnection>> = RefCell::new(HashMap::new());
}

pub struct FrontendConnection {
	pub logger: Logger,
	pub state: FrontendConnectionState,
	pub packet_listeners: HashMap<String, Box<dyn for<'a> Fn(&'a FrontendConnection, &'a InCommand)>>,
	pub event_listeners: HashMap<String, Box<dyn for<'a> Fn(&'a FrontendConnection, &'a [Event])>>,
}

pub enum FrontendConnectionState {
	Disconnected(ConnectOptions, Option<WebSocketTask>),
	Connected(Connected),
}

pub struct Connected {
	ws: WebSocketTask,
	message_handler: MessageHandler,
	pub con: Connection,

	pub messages: Vec<Message>,
	pub composing: String,
	pub composing_command: String,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ConnectionId(pub u32);

pub struct Message {
	pub(super) invoker: Invoker,
	pub(super) message: String,
}

#[derive(Default)]
pub struct ConnectionService;

#[derive(Debug, Default)]
struct MessageHandler {
	return_codes: HashMap<usize, oneshot::Sender<TsError>>,
	cur_return_code: usize,
}

impl ConnectionService {
	pub fn add_connection(logger: &Logger) -> ConnectionId {
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			for i in 0..=(cons.len() as u32) {
				if !cons.contains_key(&ConnectionId(i)) {
					let id = ConnectionId(i);
					let con = FrontendConnection {
						logger: logger.new(o!("id" => id.0)),
						state: Default::default(),
						packet_listeners: Default::default(),
						event_listeners: Default::default(),
					};
					cons.insert(id, con);
					return id;
				}
			}
			panic!("Too many connections");
		})
	}

	pub fn with_con<R, F: FnOnce(&FrontendConnection) -> R, F2: FnOnce() -> R>(
		id: ConnectionId,
		f: F,
		else_f: F2,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let cons = cons.borrow();
			cons.get(&id).map(f)
		}).unwrap_or_else(else_f)
	}

	pub fn with_mut_con<R, F: FnOnce(&mut FrontendConnection) -> R, F2: FnOnce() -> R>(
		id: ConnectionId,
		f: F,
		else_f: F2,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			cons.get_mut(&id).map(f)
		}).unwrap_or_else(else_f)
	}

	pub fn with_mut_ready_unwrap<R, F: FnOnce(&mut Connected) -> R>(
		id: ConnectionId,
		f: F,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			cons.get_mut(&id)
			.and_then(|con| {
				if let FrontendConnectionState::Connected(c) = &mut con.state {
					Some(f(c))
				} else { None }
			})
		}).expect("Should be in connected state")
	}

	pub fn with_mut_send_unwrap<F: FnOnce(&mut Connected) -> Option<OutPacket>>(
		id: ConnectionId,
		f: F,
	)
	{
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			cons.get_mut(&id)
			.and_then(|con| {
				if let FrontendConnectionState::Connected(c) = &mut con.state {
					f(c)
				} else { None }
				.map(|opt_pack| {
					let logger = con.logger.clone();
					stdweb::spawn_local(con.send_message(opt_pack).map(move |r| {
						if let Err(e) = r {
							// TODO Display notification
							error!(logger, "Failed to send message"; "error" => ?e);
						}
					}))
				})
			})
		}).expect("Should be in connected state")
	}
}

impl Connected {
	pub fn new(ws: WebSocketTask, con: Connection) -> Self {
		Self {
			ws,
			message_handler: Default::default(),
			con,

			messages: Default::default(),
			composing: Default::default(),
			composing_command: Default::default(),
		}
	}
}

impl MessageHandler {
	/// Get a return code and a receiver which gets notified when an answer is
	/// received.
	pub(crate) fn get_return_code(
		&mut self,
	) -> (usize, oneshot::Receiver<TsError>) {
		let code = self.cur_return_code;
		self.cur_return_code += 1;
		let (send, recv) = oneshot::channel();
		// The receiver should fail when the sender is dropped, but usize should
		// be enough for every platform.
		self.return_codes.insert(code, send);
		(code, recv)
	}
}

impl FrontendConnection {
	pub fn is_connected(&self) -> bool {
		if let FrontendConnectionState::Connected(_) = self.state {
			true
		} else {
			false
		}
	}

	pub fn handle_packet(&mut self, packet: InCommandMsg) -> Result<ShouldRender, Error> {
		let packet: InCommand = packet.into();
		// Call listeners
		for l in self.packet_listeners.values() {
			l(self, &packet);
		}

		let res = match &mut self.state {
			FrontendConnectionState::Disconnected(_, ws) => {
				let msg = match InMessage::new(packet) {
					Ok(r) => r,
					Err(e) => {
						error!(self.logger, "Failed to parse packet"; "error" => ?e);
						return Ok(false);
					}
				};
				if let InMessages::InitServer(_) = msg.msg() {
				} else if let InMessages::InitIvExpand2(_) = msg.msg() {
					return Ok(false);
				} else {
					error!(self.logger, "Got no initserver as first packet";
						"packet" => ?msg);
					return Ok(false);
				}

				// TODO Uid
				self.state = FrontendConnectionState::Connected(
					Connected::new(ws.take().unwrap(), Connection::new(
						Uid("".into()),
						&msg,
					)));
				true
			}
			FrontendConnectionState::Connected(c) => {
				// Handle return codes
				if packet.name() == "error" {
					let error = match InCommandError::new(&packet) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Failed to parse error command"; "error" => ?e);
							return Ok(false);
						}
					};
					let error = error.iter().next().unwrap();

					if let Some(code) =
						error.return_code.as_ref().and_then(|c| c.parse().ok())
					{
						if let Some(return_sender) =
							c.message_handler.return_codes.remove(&code)
						{
							// Ignore if sending fails
							let _ = return_sender.send(error.id);
						}
					} else {
						error!(self.logger, "Got error without return code"; "error" => ?error.id)
					}
					// Packet contains only handled return codes
					return Ok(false);
				} else if packet.name() == "notifytextmessage" {
					let msg = match InTextMessage::new(&packet) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Failed to parse message command"; "error" => ?e);
							return Ok(false);
						}
					};
					let msg = msg.iter().next().unwrap();

					let msg = Message {
						invoker: Invoker {
							id: msg.invoker_id,
							name: msg.invoker_name.to_string(),
							uid: msg.invoker_uid.as_ref().map(|u| u.clone().into()),
						},
						message: msg.message.to_string(),
					};
					c.messages.push(msg);
				}

				// Bookkeeping
				match c.con.handle_command(&packet) {
					Ok(events) => {
						// Call listeners
						for l in self.event_listeners.values() {
							l(self, &events);
						}
						false
					}
					Err(e) => {
						error!(self.logger, "Failed to handle command"; "error" => ?e);
						false
					}
				}
			}
		};
		Ok(res)
	}

	pub fn send_ws_message(&mut self, msg: &MessageF2P) -> Result<(), Error> {
		match &mut self.state {
			FrontendConnectionState::Disconnected(_, Some(ws)) => {
				ws.send_binary(MsgPack(msg));
				Ok(())
			}
			FrontendConnectionState::Connected(Connected { ws, .. }) => {
				ws.send_binary(MsgPack(msg));
				Ok(())
			}
			_ => {
				Err(format_err!("Tried to send a message without a connection"))
			}
		}
	}

	/// Adds a `return_code` to the command and returns if the corresponding
	/// answer is received. If an error occurs, the future will return an error.
	#[must_use = "futures do nothing unless polled"]
	pub fn send_message(
		&mut self,
		mut packet: OutPacket,
	) -> Box<dyn Future<Output = Result<(), TsError>> + Unpin>
	{
		let con = if let FrontendConnectionState::Connected(c) = &mut self.state {
			c
		} else {
			// TODO Return not a TsError
			return Box::new(future::err(TsError::Undefined));
		};

		// Store waiting in HashMap<usize (return code), oneshot::Sender>
		// The packet handler then sends a result to the sender if the answer is
		// received.

		let (code, recv) = con.message_handler.get_return_code();
		// Add return code
		packet
			.data_mut()
			.extend_from_slice(" return_code=".as_bytes());
		packet
			.data_mut()
			.extend_from_slice(code.to_string().as_bytes());

		// Send a message and wait until we get an answer for the return code
		if let Err(_) = self.send_ws_message(&MessageF2P::Packet(packet)) {
			return Box::new(future::err(TsError::Undefined));
		}
		Box::new(recv.map_err(|_| TsError::Undefined)
			.and_then(|r| {
				if r == TsError::Ok {
					future::ok(())
				} else {
					future::err(r.into())
				}
			}))
	}
}

impl Default for FrontendConnectionState {
	fn default() -> Self {
		FrontendConnectionState::Disconnected(
			ConnectOptions::new("localhost".into()),
			None,
		)
	}
}
