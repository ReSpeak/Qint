use std::cell::RefCell;
use std::collections::HashMap;

use failure::{format_err, Error};
use futures::channel::oneshot;
use futures::prelude::*;
use qint_shared::*;
use ts_bookkeeping::{ChannelId, Invoker, TsError};
use ts_bookkeeping::data::Connection;
use tsproto_packets::packets::OutPacket;
use slog::{o, Logger};
use yew::format::MsgPack;
use yew::services::websocket::WebSocketTask;

thread_local! {
	/// Each tab is a connection
	pub static CONNECTIONS: RefCell<HashMap<ConnectionId, FrontendConnection>> = RefCell::new(HashMap::new());
}

pub struct FrontendConnection {
	pub logger: Logger,
	pub state: FrontendConnectionState,
}

pub enum FrontendConnectionState {
	Disconnected(ConnectOptions, Option<WebSocketTask>),
	Connected(Connected),
}

pub struct Connected {
	ws: WebSocketTask,
	message_handler: MessageHandler,
	con: Connection,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ConnectionId(pub u32);

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
}

impl Connected {
	pub fn new(ws: WebSocketTask, con: Connection) -> Self {
		Self {
			ws,
			message_handler: Default::default(),
			con,
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
	) -> Box<Future<Output = Result<(), TsError>>>
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
