use futures::prelude::*;
use qint_shared::*;
use slog::error;
use ts_bookkeeping::{ChannelId, ClientId};
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

pub struct ChannelTree {
	con: ConnectionId,
	callback: Callback<()>,
}

pub enum Msg {
	Ignore,
	Redraw,
	ChangeChannel(ChannelId),
}

#[derive(Clone, Default, PartialEq)]
pub struct Props {
	pub connection: Option<ConnectionId>,
}

impl Component for ChannelTree {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
		let con = props.connection.expect("ChannelTree needs a connection id");

		let callback = link.send_back(|_| Msg::Redraw);

		let res = Self {
			con,
			callback,
		};
		res.add_listener();
		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Redraw => true,
			Msg::ChangeChannel(id) => {
				ConnectionService::with_mut_con(self.con, |con| if let
					FrontendConnectionState::Connected(c) = &mut con.state {
					let cmd = c.con.server.clients[&c.con.own_client]
						.set_channel(id);
					let logger = con.logger.clone();
					stdweb::spawn_local(con.send_message(cmd).map(move |r| {
						if let Err(e) = r {
							// TODO Display notification
							error!(logger, "Failed to change channel"; "error" => ?e);
						}
					}));
				} else {
					panic!("Should be in connected state");
				}, || panic!("Should be in connected state"));
				false
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let con = props.connection.expect("Connect needs a connection id");
		if self.con != con {
			// Remove and add listener
			ConnectionService::with_mut_con(con, |con| {
				con.packet_listeners.remove("channeltree");
			}, || {});

			self.con = con;
			self.add_listener();
			true
		} else {
			false
		}
	}
}

impl ChannelTree {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut_con(self.con, |con| {
			let callback = self.callback.clone();
			con.event_listeners.insert("channeltree".into(), Box::new(move |_, events| {
				for e in events {
					// TODO If channel or clients are modified
				}
				callback.emit(());
			}));
		}, || panic!("Should be in connected state"));

	}

	fn view_client(&self, client: &Client, own_client: ClientId) -> Html<Self> {
		if client.id == own_client {
			html! { <li class="client current",>{ &client.name }</li> }
		} else {
			html! { <li class="client",>{ &client.name }</li> }
		}
	}

	fn view_channel(
		&self,
		clients: &[&Client],
		channels: &[&Channel],
		parent: ChannelId,
		own_client: ClientId,
		own_channel: ChannelId,
	) -> Html<Self>
	{
		let this_channel = if let Some(channel) = channels.iter().find(|c| c.id == parent) {
			let id = channel.id;
			if id == own_channel {
				html! { <li class="channel current",>{ &channel.name }</li> }
			} else {
				html! { <li class="channel", onclick=|_| Msg::ChangeChannel(id).into(),>{ &channel.name }</li> }
			}
		} else {
			html! { <></> }
		};

		html! {
			<>
				{ this_channel }
				<ul class="subchannels",>
					// Clients
					{ for clients.iter().filter(|c| c.channel == parent)
						.map(|c| self.view_client(c, own_client)) }
					// Channels
					{ for channels.iter().filter(|c| c.parent == parent)
						.map(|c| self.view_channel(clients, channels, c.id, own_client, own_channel)) }
				</ul>
			</>
		}
	}

	pub fn view(&self, con: &Connection) -> Html<Self> {
		let mut channels: Vec<_> = con.server.channels.values().collect();
		let mut clients: Vec<_> = con.server.clients.values().collect();
		channels.sort_by_key(|ch| ch.order);
		clients.sort_by_key(|c| -c.talk_power);
		// TODO Make more efficient?
		// TODO Also sort clients by name?

		// Get own client and channel
		let own_client = con.own_client;
		let own_channel = con.server.clients.get(&own_client).map(|c| c.channel)
			.unwrap_or(ChannelId(0));

		html! {
			<div class="channel-tree",>
				{ self.view_channel(&clients, &channels, ChannelId(0), own_client, own_channel) }
			</div>
		}
	}
}

impl Renderable<Self> for ChannelTree {
	fn view(&self) -> Html<Self> {
		ConnectionService::with_con(self.con, |con| if let
			FrontendConnectionState::Connected(c) = &con.state {
			self.view(&c.con)
		} else {
			panic!("Should be in connected state");
		}, || panic!("Should be in connected state"))
	}
}
