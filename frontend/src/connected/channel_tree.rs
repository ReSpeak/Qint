use futures::prelude::*;
use slog::error;
use ts_bookkeeping::{ChannelId, ClientId};
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

macro_rules! cl {
	( $( $x:tt ),* ) => {
		{
			let mut temp_vec = String::new();
			$(
				cl_intern!(temp_vec, $x);
			)*
			temp_vec
		}
	};
}

macro_rules! cl_intern {
	($st:expr, ($x:expr, $y:expr)) => {
		if $y {
			$st.push_str($x);
		}
	};
	($st:expr, $x:expr) => { $st.push_str($x) };
}

pub struct ChannelTree {
	con: ConnectionId,
	callback: Callback<()>,
}

pub enum Msg {
	Ignore,
	Redraw,
	ChangeChannel(ChannelId),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
}

impl Component for ChannelTree {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
		let callback = link.send_back(|_| Msg::Redraw);

		let res = Self {
			con: props.connection,
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
					let cmd = c.con.clients[&c.con.own_client]
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
		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut_con(props.connection, |con| {
				con.packet_listeners.remove("channeltree");
			}, || {});

			self.con = props.connection;
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
					// If the id is a ChannelId or ClientId
				}
				callback.emit(());
			}));
		}, || panic!("Should be in connected state"));

	}

	fn view_client(&self, client: &Client, own_client: ClientId) -> Html<Self> {
		html! {
		<li>
			<div class="channel-line">
				<a class="entry-expand">
					<span class="entry-expand" style="display:flex;">{ &client.name }</span>
				</a>
			</div>
		</li>
		}
	}

	fn view_channel(
		&self,
		clients: &[&Client],
		channels: &[&Channel],
		channel: &Channel,
		own_client: ClientId,
		own_channel: ChannelId,
	) -> Html<Self>
	{
		let id = channel.id;
		html! {
			<li onclick=|_| Msg::ChangeChannel(id).into() >
				<div class="channel-line">
					<a class="entry-expand">
						<span class="entry-expand" style="display:flex;">{ &channel.name }</span>
					</a>
				</div>
				<ul class="menu-list">
					// Clients
					{ for clients.iter().filter(|c| c.channel == id)
						.map(|c| self.view_client(c, own_client)) }
					// Channels
					{ for channels.iter().filter(|c| c.parent == id)
						.map(|c| self.view_channel(clients, channels, c, own_client, own_channel)) }
				</ul>
			</li>
		}
	}

	pub fn view(&self, con: &Connection) -> Html<Self> {
		let mut channels: Vec<_> = con.channels.values().collect();
		let mut clients: Vec<_> = con.clients.values().collect();
		// TODO This is not the right order
		channels.sort_by_key(|ch| ch.order.0);
		clients.sort_by_key(|c| -c.talk_power);
		// TODO Make more efficient?
		// TODO Also sort clients by name?

		// Get own client and channel
		let own_client = con.own_client;
		let own_channel = con.clients.get(&own_client).map(|c| c.channel)
			.unwrap_or(ChannelId(0));

		html! {
			<div class="menu">
				<ul class="menu-list">
					<p class="menu-label">{ &con.server.name }</p>
					{ for channels.iter().filter(|c| c.parent == ChannelId(0))
						.map(|c| self.view_channel(&clients, &channels, c, own_client, own_channel)) }
				</ul>
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
