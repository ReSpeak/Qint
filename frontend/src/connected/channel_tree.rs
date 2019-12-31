use ts_bookkeeping::{ChannelId, ClientId};
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;
use std::collections::HashMap;
use std::iter;
use stdweb::js;

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
			Msg::Redraw => true,
			Msg::ChangeChannel(id) => {
				ConnectionService::with_mut_send_unwrap(self.con, |c| {
					let cmd = c.con.clients[&c.con.own_client]
						.set_channel(id);
					Some(cmd)
				}, "Failed to change channel");
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

	fn view(&self) -> Html<Self> {
		ConnectionService::with_ready_unwrap(self.con, |c| {
			self.view(&c.con)
		})
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
		con: &Connection,
		channels: &HashMap<ChannelId,ChannelBuildNode>,
		id: ChannelId,
		own_client: ClientId,
		own_channel: ChannelId,
	) -> Html<Self>
	{
		let cbn = channels.get(&id);
		if let None = cbn { return html!{} } 
		let cbn = cbn.unwrap();
		let channel = cbn.own.unwrap();
		html! {
			<li onclick=|_| Msg::ChangeChannel(id).into() >
				<div class="channel-line">
					<a class="entry-expand">
						<span class="entry-expand" style="display:flex;">{ &channel.name }</span>
					</a>
				</div>
				<ul class="menu-list">
					// Clients
					{ for cbn.clients.iter().filter_map(|client_id|  con.clients.get(client_id).map(|client| self.view_client(client, own_client))) }
					// Channels
					{ for iter::successors(cbn.first_child, |c| channels.get(c).and_then(|c| c.after))
						.map(|channel_id| self.view_channel(con, channels, channel_id, own_client, own_channel)) }
				</ul>
			</li>
		}
	}

	pub fn view(&self, con: &Connection) -> Html<Self> {
		let mut channels: HashMap<_,_> = con.channels.values()
			.map(|c| (c.id, ChannelBuildNode { own: Some(c), after: None, first_child: None, clients: vec![] })).collect();
		channels.insert(ChannelId(0), ChannelBuildNode { own: None, after: None, first_child: None, clients: vec![] }); // Server root

		// TODO clients.sort_by_key(|c| -c.talk_power);
		// TODO Also sort clients by name?

		// Build Tree
		for channel in con.channels.values() {
			if channel.order.0 == 0 {
				if let Some(cbn) = channels.get_mut(&channel.parent) {
					cbn.first_child = Some(channel.id);
				}
			} else {
				if let Some(cbn) = channels.get_mut(&channel.order) {
					cbn.after = Some(channel.id);
				}
			}
		}
		// Add all clients
		for client in con.clients.values() {
			if let Some(cbn) = channels.get_mut(&client.channel) {
				cbn.clients.push(client.id);
			}
		}

		// Get own client and channel
		let own_client = con.own_client;
		let own_channel = con.clients.get(&own_client).map(|c| c.channel)
			.unwrap_or(ChannelId(0));

		html! {
			<div class="menu">
				<ul class="menu-list">
					<p class="menu-label">{ &con.server.name }</p>
					{ for iter::successors(channels.get(&ChannelId(0)).unwrap().first_child, |c| channels.get(c).and_then(|c| c.after))
						.map(|c| self.view_channel(con, &channels, c, own_client, own_channel)) }
				</ul>
			</div>
		}
	}
}

#[derive(Debug)]
struct ChannelBuildNode<'a> {
	own: Option<&'a Channel>,
	after: Option<ChannelId>,
	first_child: Option<ChannelId>,
	clients: Vec<ClientId>,
}