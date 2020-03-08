use yew::html;
use yew::prelude::*;

use crate::connection_service::*;
use sounds::{play_sound};
use sidebar::SideBar;
use chat::Chat;

mod channel_tree;
mod chat;
mod sidebar;
mod sounds;
mod yew_markdown;

pub struct Connected {
	link: ComponentLink<Self>,
	con: ConnectionId,
	set_chat: Callback<SelectedChat>,
}

pub enum Msg {
	SetChat(SelectedChat),
	SubscribeAll,
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
		let res = Self {
			link,
			con: props.connection,
			set_chat,
		};
		res.add_listener();
		res
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
			Msg::SubscribeAll => {
				ConnectionService::with_mut_send_unwrap(&self.con, |c| {
					Some(c.con.server.set_subscribed(true))
				}, "Failed to subscribe channels");
				false
			}
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.con != props.connection {
			// Remove and add listener
			ConnectionService::with_mut(&self.con, |con| {
				con.packet_listeners.remove("connected");
			}, || {});

			self.con = props.connection;
			self.add_listener();
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
					<Chat connection=&self.con chat=&con.chat set_chat=&self.set_chat />
				</div>
			}
		})
	}

	fn destroy(&mut self) {
		ConnectionService::with_mut(&self.con, |con| {
			con.packet_listeners.remove("connected");
		}, || {});
	}
}

impl Connected {
	fn add_listener(&self) {
		ConnectionService::with_mut(&self.con, |con| {
			let subscribe = self.link.callback(|()| Msg::SubscribeAll);
			con.packet_listeners.insert("connected".into(), Box::new(move |_, packet| {
				if packet.name() == "channellistfinished" {
					subscribe.emit(());
				}
			}));
			
			con.event_listeners.insert("sounds".into(), Box::new(move |c, events| {
				if let FrontendConnection { state: FrontendConnectionState::Connected(
						crate::connection_service::Connected { con: book, .. } ), .. } = c {
					for e in events {
						play_sound(&e, &book);
					}
				}
			}));
		}, || panic!("Should be in connected state"));
	}
}
