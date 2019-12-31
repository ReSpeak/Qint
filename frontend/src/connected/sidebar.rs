use yew::html;
use yew::prelude::*;
use stdweb::js;
use ts_bookkeeping::DisconnectOptions;

use crate::connection_service::*;

use super::channel_tree::ChannelTree;

pub struct SideBar {
	con: ConnectionId,
}

pub enum Msg {
	Ignore,
	Disconnect
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
}

impl Component for SideBar {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, mut link: ComponentLink<Self>) -> Self {
		let res = Self {
			con: props.connection,
		};
		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::Disconnect => {
				ConnectionService::with_mut_send_unwrap(self.con, |c| {
						Some(c.con.disconnect(DisconnectOptions::new().message("Bye noobs")))
				}, "Failed to disconnect");
				true
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		false
	}

	fn view(&self) -> Html<Self> {
		html! {
			<aside class="sidebar">
				<div class="level" style="padding: 0.5em;">
					<div class="dropdown" onclick = |e| { js!(dropdown_click(@{e})); Msg::Ignore } >
						<div class="dropdown-trigger">
							<figure class="media-left" style="cursor: pointer;">
								<p class="image is-32x32">
									<img class="round" src="https://bulma.io/images/placeholders/128x128.png" />
								</p>
							</figure>
						</div>
						<div class="dropdown-menu" id="dropdown-menu3" role="menu">
							<div class="dropdown-content">
								<a href="#" class="dropdown-item">{"Options"}</a>
								<hr class="dropdown-divider" />
								<a href="#" class="dropdown-item" onclick=|_|{
									Msg::Disconnect
								}>{"Disconnect"}</a>
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

				<hr />

				<ChannelTree: connection=self.con />

				<hr />

				<div class="menu">
					<ul class="menu-list">
						<li>
							<div class="channel-line">
								<a class="entry-expand">
									<span class="entry-expand" style="display:flex;">
										{"Splamy (maybe)"}
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
