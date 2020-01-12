use std::collections::{HashMap, HashSet};
use std::iter;

use qint_shared::ChatType;
use ts_bookkeeping::{ChannelId, ClientId, IconHash};
use ts_bookkeeping::data::{Channel, Client, Connection};
use ts_bookkeeping::events::{Event, PropertyId};
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

macro_rules! cl {
	( $( $x:tt ),* ) => {
		{
			let mut temp_vec = String::new();
			$(
				if !temp_vec.is_empty() { temp_vec.push_str(" "); }
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

const SERVER_ICON: &str = "server";
const CHANNEL_ICON: &str = "chat-outline";
const CLIENT_ICON: &str = "account-outline";

pub struct ChannelTree {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: SelectedChat,
	set_chat: Callback<SelectedChat>,
	/// All collapsed channels.
	collapsed: HashSet<ChannelId>,
}

pub enum Msg {
	Ignore,
	ChannelChanged,
	ChangeChannel(ChannelId),
	ToggleCollapse(ChannelId, bool),
	ChannelRemoved(ChannelId),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
	#[props(required)]
	pub chat: SelectedChat,
	#[props(required)]
	pub set_chat: Callback<SelectedChat>,
}

impl Component for ChannelTree {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let res = Self {
			link,
			con: props.connection,
			chat: props.chat,
			set_chat: props.set_chat,
			collapsed: Default::default(),
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
			Msg::ToggleCollapse(chan, col) => {
				if col {
					self.collapsed.insert(chan);
				} else {
					self.collapsed.remove(&chan);
				}
				true
			}
			Msg::ChannelRemoved(chan) => {
				self.collapsed.remove(&chan);
				false
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let mut changed = false;

		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&props.connection, |con| {
				con.event_listeners.remove("channeltree");
			}, || {});

			self.con = props.connection;
			self.add_listener();
			changed = true;
		}

		if self.chat != props.chat {
			self.chat = props.chat;
			changed = true;
		}

		if self.set_chat != props.set_chat {
			self.set_chat = props.set_chat;
		}

		changed
	}

	fn view(&self) -> Html {
		ConnectionService::with_ready_unwrap(&self.con, |c| {
			self.view(&c)
		})
	}

	fn destroy(&mut self) {
		ConnectionService::with_mut(&self.con, |con| {
			con.event_listeners.remove("channeltree");
		}, || {});
	}
}

impl ChannelTree {
	fn add_listener(&self) {
		// Listen for new messages
		ConnectionService::with_mut(&self.con, |con| {
			let callback = self.link.callback(|_| Msg::ChannelChanged);
			let chan_rem = self.link.callback(|id| Msg::ChannelRemoved(id));
			con.event_listeners.insert("channeltree".into(), Box::new(move |_, events| {
				let mut should_emit = false;
				for e in events {
					if let Event::PropertyRemoved { id: PropertyId::Channel(id), .. } = e {
						chan_rem.emit(*id);
					}

					// Check if something changed in the tree
					match e {
						// Ignore messages
						Event::Message { .. } => {}
						_ => {
							should_emit = true;
							break;
						}
					}
				}
				if should_emit {
					callback.emit(());
				}
			}));
		}, || panic!("Should be in connected state"));

	}

	fn icon(&self, icon: IconHash) -> Option<Html> {
		if icon.0 != 0 {
			Some(html! {
				<span class="icon is-small line-main-icon" style=format!("background: url(/file/{}/0/icon_{})", self.con.0, icon.0)>
					<i class="mdi mdi-dummy"></i>
				</span>
			})
		} else {
			None
		}
	}

	fn mdi_icon(name: &str) -> Html {
		let name = if name.is_empty() { "dummy" } else { name };
		html! {
			<span class="icon is-small line-main-icon">
				<i class={format!("mdi mdi-{}", name)}></i>
			</span>
		}
	}

	fn view_client(&self, ctx: &ViewContext, client: &Client) -> Html {
		let icon = self.icon(client.icon_id).unwrap_or_else(|| Self::mdi_icon(CLIENT_ICON));
		let set = self.set_chat.clone();
		let uid = base64::decode(&client.uid.0).unwrap();
		let id = client.id;
		let set_chat = self.link.callback(move |_| {
			set.emit(SelectedChat {
				chat_type: ChatType::Client(uid.clone()),
				client: Some(id),
			});
			Msg::Ignore
		});
		html! {
			<li>
				<div class={ cl![
						"channel-line",
						("own-client", ctx.own_client == id),
						("selected-client", ctx.selected_client == Some(id))] } >
					{ Self::mdi_icon("") }
					<a class="entry-expand description" onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &client.name }</span>
					</a>
				</div>
			</li>
		}
	}

	fn view_channel(&self, con: &Connection, ctx: &ViewContext, id: ChannelId) -> Html {
		let cbn = ctx.channels.get(&id);
		if cbn.is_none() { return html!{} }
		let cbn = cbn.unwrap();
		let channel = cbn.own.unwrap();

		// Sort clients by descending talk power and name
		let mut clients = cbn.clients.iter().filter_map(|client_id|
			con.clients.get(client_id)).collect::<Vec<_>>();
		clients.sort_by(|a, b| a.talk_power.cmp(&b.talk_power).reverse()
			.then_with(|| a.name.cmp(&b.name)));

		let collapsed = self.collapsed.contains(&id);
		let collapse_icon = if cbn.first_child.is_some() || !clients.is_empty() {
			if collapsed {
				"chevron-right"
			} else {
				"chevron-right mdi-rotate-90"
			}
		} else {
			""
		};

		let icon = channel.icon_id.and_then(|i| self.icon(i)).unwrap_or_else(|| Self::mdi_icon(CHANNEL_ICON));
		let change_channel = self.link.callback(move |_| Msg::ChangeChannel(id));
		let toggle_collapse = self.link.callback(move |_| {
			Msg::ToggleCollapse(id, !collapsed)
		});
		let set = self.set_chat.clone();
		let set_chat = self.link.callback(move |_| {
			set.emit(SelectedChat {
				chat_type: ChatType::Channel(id.0),
				client: None,
			});
			Msg::Ignore
		});
		html! {
			<li>
				<div class={cl![
						"channel-line",
						("own-client", ctx.own_channel == id),
						("selected-channel", ctx.selected_channel == Some(id))]}>
					<a onclick=toggle_collapse>{ Self::mdi_icon(collapse_icon) }</a>
					<a class="entry-expand description" ondoubleclick=change_channel onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &channel.name }</span>
					</a>
				</div>
				<ul class=cl!["menu-list", ("collapsed", collapsed)]>
					// Clients
					{ for clients.iter().map(|client| self.view_client(ctx, client)) }
					// Channels
					{ for iter::successors(cbn.first_child, |c| ctx.channels.get(c).and_then(|c| c.after))
						.map(|channel_id| self.view_channel(con, ctx, channel_id)) }
				</ul>
			</li>
		}
	}

	pub fn view(&self, c: &Connected) -> Html {
		let con = &c.con;
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

		// Get all special ids for highlighting convenience
		let mut selected_channel = None;
		let mut selected_client = None;
		let mut selected_server = false;
		match &self.chat.chat_type {
			ChatType::Client(_) => { selected_client = c.chat.client; },
			ChatType::Channel(c) => { selected_channel = Some(ChannelId(*c)) },
			ChatType::Server => { selected_server = true; },
			_ => {},
		}

		let ctx = ViewContext {
			channels,
			own_client: con.own_client,
			own_channel: con.clients.get(&con.own_client).map(|c| c.channel)
				.unwrap_or(ChannelId(0)),
			selected_channel,
			selected_client,
			selected_server,
		};

		let set = self.set_chat.clone();
		let set_chat = self.link.callback(move |_| {
			set.emit(SelectedChat {
				chat_type: ChatType::Server,
				client: None,
			});
			Msg::Ignore
		});
		let icon = self.icon(con.server.icon_id).unwrap_or_else(|| Self::mdi_icon(SERVER_ICON));
		html! {
			<div class="menu channel-list">
				<ul class="menu-list">
					<p class="menu-label" onclick=set_chat>
						{ icon }
						<span class="entry-expand">{ &con.server.name }</span>
					</p>
					{ for iter::successors(ctx.channels.get(&ChannelId(0)).unwrap().first_child, |c| ctx.channels.get(c).and_then(|c| c.after))
						.map(|c| self.view_channel(con, &ctx, c)) }
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

struct ViewContext<'a> {
	channels: HashMap<ChannelId,ChannelBuildNode<'a>>,
	own_client: ClientId,
	own_channel: ChannelId,
	selected_channel: Option<ChannelId>,
	selected_client: Option<ClientId>,
	selected_server: bool,
}
