use yew::html;
use yew::prelude::*;

use crate::connection_service::*;
use sidebar::SideBar;
use chat::Chat;

mod channel_tree;
mod chat;
mod sidebar;

pub struct Connected {
	con: ConnectionId,
}

pub enum Msg {
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
}

impl Component for Connected {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
		Self {
			con: props.connection,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.con != props.connection {
			self.con = props.connection;
			true
		} else {
			false
		}
	}

	fn view(&self) -> Html {
		html! {
			<div class="connected-container">
				<SideBar: connection=&self.con />
				<Chat: connection=&self.con />
			</div>
		}
	}
}
