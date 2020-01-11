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
	set_chat: Callback<SelectedChat>,
}

pub enum Msg {
	SetChat(SelectedChat),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
}

impl Component for Connected {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let set_chat = link.callback(move |c| {
			Msg::SetChat(c)
		});
		Self {
			con: props.connection,
			set_chat,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::SetChat(c) => ConnectionService::with_mut_ready_unwrap(&self.con, |con| {
				if con.chat != c {
					con.chat = c;
					true
				} else {
					false
				}
			}),
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
		ConnectionService::with_ready_unwrap(&self.con, |con| {
			html! {
				<div class="connected-container">
					<SideBar connection=&self.con chat=&con.chat set_chat=&self.set_chat />
					<Chat connection=&self.con chat=&con.chat />
				</div>
			}
		})
	}
}
