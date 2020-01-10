use qint_shared::ChatType;
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;
use sidebar::SideBar;
use chat::Chat;

mod channel_tree;
mod chat;
mod sidebar;

pub struct Connected {
	link: ComponentLink<Self>,
	con: ConnectionId,
	chat_type: ChatType,
}

pub enum Msg {
	SetChat(ChatType),
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
		Self {
			link,
			con: props.connection,
			chat_type: ChatType::Server,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::SetChat(c) => {
				// TODO This is not optimal and should call Chat::set_chat
				self.chat_type = c;
				true
			}
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
		let set_chat = self.link.callback(move |c| {
			Msg::SetChat(c)
		});
		html! {
			<div class="connected-container">
				<SideBar connection=&self.con set_chat=set_chat />
				<Chat connection=&self.con chat_type=&self.chat_type />
			</div>
		}
	}
}
