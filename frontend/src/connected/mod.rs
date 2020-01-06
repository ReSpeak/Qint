use futures::prelude::*;
use slog::error;
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
		ConnectionService::with_mut(&props.connection, |con| if let
			FrontendConnectionState::Connected(c) = &mut con.state {
			let cmd = c.con.server.set_subscribed(true);
			let logger = con.logger.clone();
			// TODO This does never resolve??
			stdweb::spawn_local(con.send_message(cmd).map(move |r| {
				if let Err(e) = r {
					error!(logger, "Failed to subscribe to all channels"; "error" => ?e);
				}
			}));
		}, || panic!("Should be in connected state"));

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

	fn view(&self) -> Html<Self> {
		html! {
			<div class="connected-container">
				<SideBar: connection=self.con.clone() />
				<Chat: connection=self.con.clone() />
			</div>
		}
	}
}
