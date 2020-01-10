use std::collections::HashMap;
use std::iter;

use qint_shared::ChatType;
use ts_bookkeeping::{ChannelId, ClientId, IconHash};
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

/*macro_rules! cl {
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
}*/

pub struct ChannelTree {
	link: ComponentLink<Self>,
	con: ConnectionId,
	set_chat: Callback<ChatType>,
}

pub enum Msg {
	Ignore,
	ChannelChanged,
	ChangeChannel(ChannelId),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
	#[props(required)]
	pub set_chat: Callback<ChatType>,
}

impl Component for ChannelTree {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let res = Self {
			link,
			con: props.connection,
			set_chat: props.set_chat,
		};
		res.add_listener();
		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::ChannelChanged => true,
			Msg::ChangeChannel(id) => {
				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
					let cmd = c.con.clients[&c.con.own_client]
						.set_channel(id);
					Some(cmd)
				}, "Failed to change channel");
				false
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let mut changed = false;

		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&props.connection, |con| {
				con.packet_listeners.remove("channeltree");
			}, || {});

			self.con = props.connection;
			self.add_listener();
			changed = true;
		}

		if self.set_chat != props.set_chat {
			self.set_chat = props.set_chat;
			changed = true;
		}

		changed
	}

	fn view(&self) -> Html {
		ConnectionService::with_ready_unwrap(&self.con, |c| {
			self.view(&c.con)
		})
	}
}

impl ChannelTree {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut(&self.con, |con| {
			let callback = self.link.callback(|_| Msg::ChannelChanged);
			con.event_listeners.insert("channeltree".into(), Box::new(move |_, events| {
				for _e in events {
					// TODO If channel or clients are modified
					// If the id is a ChannelId or ClientId
				}
				callback.emit(());
			}));
		}, || panic!("Should be in connected state"));

	}

	fn icon(&self, icon: IconHash) -> Html {
		if icon.0 != 0 {
			html! {
				<span class="icon">
					<img src=format!("/file/{}/0/icon_{}", self.con.0, icon.0) />
				</span>
			}
		} else {
			html! {}
		}
	}

	fn view_client(&self, client: &Client, _own_client: ClientId) -> Html {
		let icon = self.icon(client.icon_id);
		let set = self.set_chat.clone();
		let uid = base64::decode(&client.uid.0).unwrap();
		let set_chat = self.link.callback(move |_| {
			set.emit(ChatType::Client(uid.clone()));
			Msg::Ignore
		});
		html! {
			<li>
				<div class="channel-line">
					<a class="entry-expand" style="display:flex;" onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &client.name }</span>
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
	) -> Html
	{
		let cbn = channels.get(&id);
		if cbn.is_none() { return html!{} }
		let cbn = cbn.unwrap();
		let channel = cbn.own.unwrap();

		// Sort clients by descending talk power and name
		let mut clients = cbn.clients.iter().filter_map(|client_id|
			con.clients.get(client_id)).collect::<Vec<_>>();
		clients.sort_by(|a, b| a.talk_power.cmp(&b.talk_power).reverse()
			.then_with(|| a.name.cmp(&b.name)));

		let icon = channel.icon_id.map(|i| self.icon(i)).unwrap_or_else(|| html! {});
		let change_channel = self.link.callback(move |_| Msg::ChangeChannel(id));
		let set = self.set_chat.clone();
		let set_chat = self.link.callback(move |_| {
			set.emit(ChatType::Channel(id.0));
			Msg::Ignore
		});
		html! {
			<li>
				<div class="channel-line">
					<a class="entry-expand" style="display:flex;" ondoubleclick=change_channel onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &channel.name }</span>
					</a>
				</div>
				<ul class="menu-list">
					// Clients
					{ for clients.iter().map(|client| self.view_client(client, own_client)) }
					// Channels
					{ for iter::successors(cbn.first_child, |c| channels.get(c).and_then(|c| c.after))
						.map(|channel_id| self.view_channel(con, channels, channel_id, own_client, own_channel)) }
				</ul>
			</li>
		}
	}

	pub fn view(&self, con: &Connection) -> Html {
		let mut channels: HashMap<_,_> = con.channels.values()
			.map(|c| (c.id, ChannelBuildNode { own: Some(c), after: None, first_child: None, clients: vec![] })).collect();
		channels.insert(ChannelId(0), ChannelBuildNode { own: None, after: None, first_child: None, clients: vec![] }); // Server root

		// Build Tree
		for channel in con.channels.values() {
			if channel.order.0 == 0 {
				if let Some(cbn) = channels.get_mut(&channel.parent) {
					cbn.first_child = Some(channel.id);
				}
			} else if let Some(cbn) = channels.get_mut(&channel.order) {
				cbn.after = Some(channel.id);
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

		let set = self.set_chat.clone();
		let set_chat = self.link.callback(move |_| {
			set.emit(ChatType::Server);
			Msg::Ignore
		});
		let icon = self.icon(con.server.icon_id);
		html! {
			<div class="menu">
				<ul class="menu-list">
					<p class="menu-label" onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &con.server.name }</span>
					</p>
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
