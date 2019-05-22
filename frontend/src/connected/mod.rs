use std::collections::HashMap;
use std::mem;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures::channel::oneshot;
use futures::prelude::*;
use qint_shared::*;
use slog::{error, info, Logger};
use ts_bookkeeping::TsError;
use ts_bookkeeping::data::Connection;
use ts_bookkeeping::messages::s2c::{InCommandError, InMessageTrait};
use tsproto_packets::packets::{InCommand, OutPacket};
use yew::html;
use yew::prelude::*;

use crate::{Model, Msg};
use crate::connection::ConnectionMsg;
use channel_tree::ChannelTree;

mod channel_tree;

pub struct Connected {
	logger: Logger,
	connection: Connection,
	return_code_handler: ReturnCodeHandler,
	// TODO suboptimal
	send_packets: Vec<OutPacket>,

	channel_tree: ChannelTree,
}

pub enum ConnectedMsg {
	Packet(InCommandMsg),
}

struct ReturnCodeHandler {
	return_codes: HashMap<usize, oneshot::Sender<TsError>>,
	cur_return_code: AtomicUsize,
}

impl ReturnCodeHandler {
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
}

impl Connected {
	pub fn new(connection: Connection, logger: Logger) -> Self {
		let mut con = Connected {
			logger,
			connection,
			return_code_handler: ReturnCodeHandler {
				return_codes: Default::default(),
				cur_return_code: AtomicUsize::new(0),
			},
			send_packets: Default::default(),

			channel_tree: Default::default(),
		};

		let cmd = con.connection.server.set_subscribed(true);
		let logger = con.logger.clone();
		stdweb::spawn_local(con.send_packet(cmd).map(move |r| {
			if let Err(e) = r {
				error!(logger, "Failed to subscribe"; "error" => ?e);
			}
		}));

		con
	}

	pub fn update(&mut self, msg: ConnectedMsg) -> (Vec<OutPacket>, ShouldRender) {
		match msg {
			ConnectedMsg::Packet(packet) => {
				let packet: InCommand = packet.into();
				// Handle return codes
				if packet.name() == "error" {
					let error = match InCommandError::new(&packet) {
						Ok(r) => r,
						Err(e) => {
							error!(self.logger, "Failed to parse error command"; "error" => ?e);
							return (mem::replace(&mut self.send_packets, Vec::new()), false);
						}
					};
					let error = error.iter().next().unwrap();

					if let Some(code) =
						error.return_code.as_ref().and_then(|c| c.parse().ok())
					{
						if let Some(return_sender) =
							self.return_code_handler.return_codes.remove(&code)
						{
							// Ignore if sending fails
							let _ = return_sender.send(error.id);
						}
					} else {
						error!(self.logger, "Got error without return code"; "error" => ?error.id)
					}
					// Packet contains only handled return codes
					return (mem::replace(&mut self.send_packets, Vec::new()), false);
				}


				// Bookkeeping
				match self.connection.handle_command(&packet) {
					Ok(events) => {
						// TODO
						info!(self.logger, "Got events"; "events" => ?events);
						(mem::replace(&mut self.send_packets, Vec::new()), true)
					}
					Err(e) => {
						error!(self.logger, "Failed to handle command"; "error" => ?e);
						(mem::replace(&mut self.send_packets, Vec::new()), false)
					}
				}
			}
		}
	}

	/// Adds a `return_code` to the command and returns if the corresponding
	/// answer is received. If an error occurs, the future will return an error.
	#[must_use = "futures do nothing unless polled"]
	pub fn send_packet(
		&mut self,
		mut packet: OutPacket,
	) -> impl Future<Output = Result<(), TsError>>
	{
		// Store waiting in HashMap<usize (return code), oneshot::Sender>
		// The packet handler then sends a result to the sender if the answer is
		// received.

		let (code, recv) = self.return_code_handler.get_return_code();
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

impl Renderable<Model> for Connected {
	fn view(&self) -> Html<Model> {
		html! {
			<div class="connected-container",>
				<div class="channel-tree",>
					{ self.channel_tree.view(&self.connection) }
				</div>
			</div>
		}
	}
}

impl Into<Msg> for ConnectedMsg {
	fn into(self) -> Msg { Msg::Connection(ConnectionMsg::Connected(self)) }
}
