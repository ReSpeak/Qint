use qint_shared::*;
use slog::{error, info, Logger};
use ts_bookkeeping::data::Connection;
use yew::html;
use yew::prelude::*;

use crate::{Model, Msg};
use crate::connection::ConnectionMsg;
use channel_tree::ChannelTree;

mod channel_tree;

pub struct Connected {
	connection: Connection,
	channel_tree: ChannelTree,
}

pub enum ConnectedMsg {
	Packet(InCommandMsg),
}

impl Connected {
	pub fn new(connection: Connection) -> Self {
		Connected {
			connection,
			channel_tree: Default::default(),
		}
	}

	pub fn update(&mut self, msg: ConnectedMsg, logger: &Logger) -> ShouldRender {
		match msg {
			ConnectedMsg::Packet(packet) => {
				match self.connection.handle_command(&packet.into()) {
					Ok(events) => {
						// TODO
						info!(logger, "Got events"; "events" => ?events);
						true
					}
					Err(e) => {
						error!(logger, "Failed to handle command"; "error" => ?e);
						false
					}
				}
			}
		}
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
