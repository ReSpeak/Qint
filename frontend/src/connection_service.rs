use std::cell::RefCell;
use std::collections::HashMap;

use failure::{format_err, Error};
use futures::channel::oneshot;
use futures::prelude::*;
use qint_shared::*;
use ts_bookkeeping::{TsError, Uid};
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::events::Event;
use ts_bookkeeping::messages::s2c::{InCommandError, InMessage, InMessages, InMessageTrait};
use tsproto_packets::packets::{InCommand, OutPacket};
use slog::{debug, error, o, Logger};
use uuid::Uuid;
use yew::ShouldRender;
use yew::prelude::Callback;
use yew::format::MsgPack;
use yew::services::websocket::{WebSocketService, WebSocketStatus, WebSocketTask};

thread_local! {
	/// Each tab is a connection
	pub static CONNECTIONS: RefCell<HashMap<ConnectionId, FrontendConnection>> = RefCell::new(HashMap::new());
}

pub type Listener<T> = Box<dyn for<'a> Fn(&'a FrontendConnection, &'a T)>;

pub struct FrontendConnection {
	pub logger: Logger,
	pub state: FrontendConnectionState,
	pub packet_listeners: HashMap<String, Listener<InCommand>>,
	pub event_listeners: HashMap<String, Listener<[Event]>>,
}

pub enum FrontendConnectionState {
	Uninitialized,
	/// The used options, the websocket and the public key of the server, which
	/// we should get before the initserver.
	Connecting(ConnectOptions, WebSocketTask, Option<Vec<u8>>),
	Connected(Connected),
}

pub struct Connected {
	ws: WebSocketTask,
	message_handler: MessageHandler,
	pub con: Connection,
	/// The public key of the server
	pub key: Vec<u8>,

	pub composing: String,
	pub composing_command: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[repr(transparent)]
pub struct ConnectionId(pub Uuid);

#[derive(Default)]
pub struct ConnectionService;

#[derive(Debug, Default)]
struct MessageHandler {
	return_codes: HashMap<usize, oneshot::Sender<TsError>>,
	cur_return_code: usize,
}

impl ConnectionService {
	pub fn add(logger: &Logger, options: ConnectOptions, callback: Callback<crate::WsMsg>, notification: Callback<WebSocketStatus>) -> Result<ConnectionId, Error> {
		let mut ws_service = WebSocketService::new();

		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			for _ in 0..5 {
				let id = ConnectionId(Uuid::new_v4());
				if !cons.contains_key(&id) {
					// Create connection
					let url = format!("{}/ws/{}", crate::Model::get_ws_domain(), id.0);
					let task = ws_service.connect(&url, callback, notification).map_err(|e| format_err!("{}", e))?;

					let con = FrontendConnection {
						logger: logger.new(o!("id" => id.0.to_string())),
						state: FrontendConnectionState::Connecting(options, task, None),
						packet_listeners: Default::default(),
						event_listeners: Default::default(),
					};
					debug!(con.logger, "Creating connection");
					cons.insert(id.clone(), con);
					return Ok(id);
				}
			}
			panic!("Too many connections");
		})
	}

	pub fn remove(id: &ConnectionId) -> Option<FrontendConnection> {
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			let res = cons.remove(id);
			if let Some(con) = &res {
				debug!(con.logger, "Removing connection");
			}
			res
		})
	}

	pub fn with<R, F: FnOnce(&FrontendConnection) -> R, F2: FnOnce() -> R>(
		id: &ConnectionId,
		f: F,
		else_f: F2,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let cons = cons.borrow();
			cons.get(id).map(f)
		}).unwrap_or_else(else_f)
	}

	pub fn with_mut<R, F: FnOnce(&mut FrontendConnection) -> R, F2: FnOnce() -> R>(
		id: &ConnectionId,
		f: F,
		else_f: F2,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			cons.get_mut(id).map(f)
		}).unwrap_or_else(else_f)
	}

	pub fn with_unwrap<R, F: FnOnce(&FrontendConnection) -> Option<R>>(
		id: &ConnectionId,
		f: F,
		error_message: &str,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let cons = cons.borrow();
			cons.get(id).and_then(f)
		}).expect(error_message)
	}

	pub fn with_mut_unwrap<R, F: FnOnce(&mut FrontendConnection) -> Option<R>>(
		id: &ConnectionId,
		f: F,
		error_message: &str,
	) -> R
	{
		CONNECTIONS.with(|cons| {
			let mut cons = cons.borrow_mut();
			cons.get_mut(id).and_then(f)
		}).expect(error_message)
	}

	pub fn with_ready_unwrap<R, F: FnOnce(&Connected) -> R>(
		id: &ConnectionId,
		f: F,
	) -> R
	{
		Self::with_unwrap(id, |con| {
			if let FrontendConnectionState::Connected(c) = &con.state {
				Some(f(c))
			} else { None }
		}, "Should be in connected state")
	}

	pub fn with_mut_ready_unwrap<R, F: FnOnce(&mut Connected) -> R>(
		id: &ConnectionId,
		f: F,
	) -> R
	{
		Self::with_mut_unwrap(id, |con| {
			if let FrontendConnectionState::Connected(c) = &mut con.state {
				Some(f(c))
			} else { None }
		}, "Should be in connected state")
	}

	pub fn with_mut_send_unwrap<F: FnOnce(&mut Connected) -> Option<OutPacket>>(
		id: &ConnectionId,
		f: F,
		error_msg: &'static str,
	)
	{
		Self::with_mut_unwrap(id, |con| {
			if let FrontendConnectionState::Connected(c) = &mut con.state {
				f(c)
			} else { None }
			.map(|opt_pack| {
				let logger = con.logger.clone();
				stdweb::spawn_local(con.send_message(opt_pack).map(move |r| {
					if let Err(e) = r {
						// TODO Display notification
						error!(logger, "{}", error_msg; "error" => ?e);
					}
				}))
			})
		}, "Should be in connected state")
	}
}

impl Connected {
	pub fn new(ws: WebSocketTask, key: Vec<u8>, con: Connection) -> Self {
		Self {
			ws,
			message_handler: Default::default(),
			con,
			key,

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
			FrontendConnectionState::Connecting(_, _, key) => {
				let msg = InMessage::new(packet).map_err(|(_, e)| e)?;
				if let InMessages::InitServer(_) = msg.msg() {
				} else if let InMessages::InitIvExpand2(_) = msg.msg() {
					return Ok(false);
				} else {
					error!(self.logger, "Got no initserver as first packet";
						"packet" => ?msg);
					return Ok(false);
				}
				let key = key.take().expect("Public key of server has to be \
					sent before initserver");

				// TODO Save key instead of uid
				if let FrontendConnectionState::Connecting(_, ws, _) =
					std::mem::replace(&mut self.state, FrontendConnectionState::Uninitialized) {
					self.state = FrontendConnectionState::Connected(
						Connected::new(ws, key, Connection::new(
							Uid("".into()),
							&msg,
						)));
				} else {
					unreachable!()
				}
				true
			}
			FrontendConnectionState::Connected(c) => {
				// Handle return codes
				if packet.name() == "error" {
					let error = InCommandError::new(&packet)?;
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
				}

				// Bookkeeping
				let events = c.con.handle_command(&packet)?;
				// Call listeners
				for l in self.event_listeners.values() {
					l(self, &events);
				}
				false
			}
			_ => panic!("Frontend connection is in uninitialized state"),
		};
		Ok(res)
	}

	pub fn send_ws_message(&mut self, msg: &MessageF2P) -> Result<(), Error> {
		match &mut self.state {
			FrontendConnectionState::Connecting(_, ws, _) => {
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
			.extend_from_slice(b" return_code=");
		packet
			.data_mut()
			.extend_from_slice(code.to_string().as_bytes());

		// Send a message and wait until we get an answer for the return code
		if self.send_ws_message(&MessageF2P::Packet(packet)).is_err() {
			return Box::new(future::err(TsError::Undefined));
		}
		Box::new(recv.map_err(|_| TsError::Undefined)
			.and_then(|r| {
				if r == TsError::Ok {
					future::ok(())
				} else {
					future::err(r)
				}
			}))
	}
}
