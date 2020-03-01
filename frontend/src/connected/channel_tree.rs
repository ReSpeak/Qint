use std::collections::{HashMap, HashSet};
use std::iter;

use qint_shared::ChatType;
use ts_bookkeeping::{ChannelId, ClientId};
use ts_bookkeeping::data::{Channel, Client, Connection};
use ts_bookkeeping::events::{Event, PropertyId};
use yew::html;
use yew::prelude::*;
use stdweb::web::event::IEvent;

use crate::connection_service::*;
use crate::controls::context_menu::{ContextMenu, Pos2D};
use crate::controls::icon::Icon;
use crate::html_util::ToColor;

pub struct ChannelTree {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: SelectedChat,
	set_chat: Callback<SelectedChat>,
	context_menu: ContextMenuData,
	close_ctxm: Callback<()>,
	/// All collapsed channels.
	collapsed: HashSet<ChannelId>,
}

pub enum Msg {
	Ignore,
	ChannelChanged,
	ChangeChannel(ChannelId),
	ToggleCollapse(ChannelId, bool),
	ChannelRemoved(ChannelId),
	ContextOpened(ContextMenuData),
	ContextClosed,
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
		let close_ctxm = link.callback(|_| { Msg::ContextClosed });
		let res = Self {
			link,
			con: props.connection,
			chat: props.chat,
			set_chat: props.set_chat,
			collapsed: Default::default(),
			context_menu: ContextMenuData { source: ContextMenuType::None, pos: Pos2D{ x:0, y:0 } },
			close_ctxm,
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
			},
			Msg::ContextOpened(ctxm) => {
				self.context_menu = ctxm;
				true
			},
			Msg::ContextClosed => {
				self.context_menu.source = ContextMenuType::None;
				true
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

	fn view_client(&self, ctx: &ViewContext, client: &Client) -> Html {
		let icon = Icon::client_icon(&self.con, client);
		let set = self.set_chat.clone();
		let uid = client.uid.0.clone();
		let id = client.id;
		let set_chat = self.link.callback(move |_| {
			set.emit(SelectedChat {
				chat_type: ChatType::Client(uid.clone()),
				client: Some(id),
			});
			Msg::Ignore
		});

		let context_request = self.link.callback(move |e: ContextMenuEvent| {
			e.prevent_default();
			Msg::ContextOpened(ContextMenuData{
				source: ContextMenuType::Client(id),
				pos: Pos2D { x:e.client_x(), y:e.client_y() },
			})
		});

		let cm = if self.context_menu.source == ContextMenuType::Client(id) {
			html! {
				<ContextMenu pos=&self.context_menu.pos close_cb=&self.close_ctxm>
					<a>{"Kick client"}</a>
				</ContextMenu>
			}
		} else { html!{} };

		let user_color = client.to_color();
		html! {
			<li>
				{ cm }
				<div class={ cl![
						"channel-line",
						("own-client", ctx.own_client == id),
						("selected-client", ctx.selected_client == Some(id))] } >
					{ Icon::mdi_icon("") }
					<a class="entry-expand" onclick=set_chat oncontextmenu=context_request>
						{ icon }
						<span class="entry-expand" style=user_color>{ &client.name }</span>
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

		let icon = Icon::channel_icon(&self.con, channel);
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

		let context_request = self.link.callback(move |e: ContextMenuEvent| {
			e.prevent_default();
			Msg::ContextOpened(ContextMenuData{
				source: ContextMenuType::Channel(id),
				pos: Pos2D { x:e.client_x(), y:e.client_y() },
			})
		});

		let cm = if self.context_menu.source == ContextMenuType::Channel(id) {
			html! {
				<ContextMenu pos=&self.context_menu.pos close_cb=&self.close_ctxm>
					<a>{"Rename"}</a>
					<a>{"Subscribe or I reporte u"}</a>
				</ContextMenu>
			}
		} else { html!{} };

		let mut formatted_channel_name = channel.name.as_str();
		let mut channel_align_center = false;
		let mut channel_align_right = false;

		if channel.name.starts_with("[cspacer") {
			let end = channel.name.find("]");
			if let Some(idx) = end {
				channel_align_center = true;
				formatted_channel_name = &channel.name[(idx + 1)..];
			}
		} else if channel.name.starts_with("[rspacer") {
			let end = channel.name.find("]");
			if let Some(idx) = end {
				channel_align_right = true;
				formatted_channel_name = &channel.name[(idx + 1)..];
			}
		}

		html! {
			<li>
				{ cm }
				<div class={cl![
						"channel-line",
						("own-client", ctx.own_channel == id),
						("selected-channel", ctx.selected_channel == Some(id))]}>
					<span class="collapse-button" onclick=toggle_collapse>{ Icon::mdi_icon(collapse_icon) }</span>
					{ icon }
					<a class=cl!["entry-expand", ("text-align-center", channel_align_center), ("text-align-right", channel_align_right)] ondoubleclick=change_channel onclick=set_chat oncontextmenu=context_request>
						<span class="entry-expand">{ formatted_channel_name }</span>
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
		};

		let set = self.set_chat.clone();
		let set_chat = self.link.callback(move |_| {
			set.emit(SelectedChat {
				chat_type: ChatType::Server,
				client: None,
			});
			Msg::Ignore
		});
		let icon = Icon::server_icon(&self.con, &con.server);
		html! {
			<div class="menu channel-list">
				<p class="menu-label" onclick=set_chat>
					{ icon }
					<span class=cl!["entry-expand", ("selected-server", selected_server)]>{ &con.server.name }</span>
				</p>
				<ul class="menu-list">
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
}

pub struct ContextMenuData {
	source: ContextMenuType,
	pos: Pos2D,
}

#[derive(PartialEq)]
pub enum ContextMenuType {
	None,
	Client(ClientId),
	Channel(ChannelId),
}
