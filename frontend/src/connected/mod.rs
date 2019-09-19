use futures::prelude::*;
use slog::error;
use yew::html;
use yew::prelude::*;

use crate::connection_service::*;
use channel_tree::ChannelTree;
use chat::Chat;

mod channel_tree;
mod chat;

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
		ConnectionService::with_mut_con(props.connection, |con| if let
			FrontendConnectionState::Connected(c) = &mut con.state {
			let cmd = c.con.server.set_subscribed(true);
			let logger = con.logger.clone();
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
}

impl Renderable<Self> for Connected {
	fn view(&self) -> Html<Self> {
		html! {
			<div class="connected-container">
				<ChannelTree: connection=self.con />
				<Chat: connection=self.con />
			</div>
		}
	}
}
