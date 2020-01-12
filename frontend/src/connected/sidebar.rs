use stdweb::js;
use ts_bookkeeping::DisconnectOptions;
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;

use super::channel_tree::ChannelTree;

pub struct SideBar {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat: SelectedChat,
	set_chat: Callback<SelectedChat>,
}

pub enum Msg {
	Ignore,
	Disconnect,
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

impl Component for SideBar {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		Self {
			link,
			con: props.connection,
			chat: props.chat,
			set_chat: props.set_chat,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Disconnect => {
				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
						Some(c.con.disconnect(DisconnectOptions::new().message("Bye noobs")))
				}, "Failed to disconnect");
				true
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		let mut changed = false;

		if self.con != props.connection {
			self.con = props.connection;
			changed = true;
		}

		if self.chat != props.chat {
			self.chat = props.chat;
			changed = true;
		}

		if self.set_chat != props.set_chat {
			self.set_chat = props.set_chat;
			changed = true;
		}

		changed
	}

	fn view(&self) -> Html {
		let dropdown_click = self.link.callback(|e: ClickEvent| {
			js!(dropdown_click(@{e}));
			Msg::Ignore
		});
		let disconnect_click = self.link.callback(|_| Msg::Disconnect);

		html! {
			<aside class="sidebar">
				<div class="level" style="padding: 0.5em;">
					<div class="dropdown" onclick=dropdown_click>
						<div class="dropdown-trigger">
							<figure class="media-left" style="cursor: pointer;">
								<p class="image is-32x32">
									<img class="round" src="https://bulma.io/images/placeholders/128x128.png" />
								</p>
							</figure>
						</div>
						<div class="dropdown-menu" id="dropdown-menu3" role="menu">
							<div class="dropdown-content">
								<a href="#" class="dropdown-item">{ "Options" }</a>
								<hr class="dropdown-divider" />
								<a href="#" class="dropdown-item" onclick=disconnect_click>{ "Disconnect" }</a>
							</div>
						</div>
					</div>
					<div class="media-content">
						<p class="control has-icons-right">
							<input class="input" type="text" placeholder="Search" />
							<span class="icon is-small is-right">
								<i class="mdi mdi-magnify mdi-dark"></i>
							</span>
						</p>
					</div>
				</div>

				<ChannelTree connection=&self.con chat=&self.chat set_chat=&self.set_chat />

				<div class="menu">
					<ul class="menu-list">
						<li>
							<div class="channel-line">
								<a class="entry-expand">
									<span class="entry-expand" style="display:flex;">
										{ "Splamy (maybe)" }
									</span>
								</a>
							</div>
							<ul class="menu-list"></ul>
						</li>
					</ul>
				</div>
			</aside>
		}
	}
}
