use ts_bookkeeping::{ChannelId, ClientId};
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;

use crate::Model;
use super::ConnectedMsg;

pub struct ChannelTree {
	pub is_talking: bool,
}

impl Default for ChannelTree {
	fn default() -> Self { Self { is_talking: false } }
}

impl ChannelTree {
	fn view_client(&self, client: &Client, own_client: ClientId) -> Html<Model> {
		if client.id == own_client {
			html! { <li class="client current",>{ &client.name }</li> }
		} else {
			html! { <li class="client",>{ &client.name }</li> }
		}
	}

	fn view_channel(
		&self,
		clients: &[&Client],
		channels: &[&Channel],
		parent: ChannelId,
		own_client: ClientId,
		own_channel: ChannelId,
	) -> Html<Model>
	{
		let this_channel = if let Some(channel) = channels.iter().find(|c| c.id == parent) {
			let id = channel.id;
			if id == own_channel {
				html! { <li class="channel current",>{ &channel.name }</li> }
			} else {
				html! { <li class="channel", onclick=|_| ConnectedMsg::ChangeChannel(id).into(),>{ &channel.name }</li> }
			}
		} else {
			html! { <></> }
		};

		html! {
			<>
				{ this_channel }
				<ul class="subchannels",>
					// Clients
					{ for clients.iter().filter(|c| c.channel == parent)
						.map(|c| self.view_client(c, own_client)) }
					// Channels
					{ for channels.iter().filter(|c| c.parent == parent)
						.map(|c| self.view_channel(clients, channels, c.id, own_client, own_channel)) }
				</ul>
			</>
		}
	}

	pub fn view(&self, con: &Connection) -> Html<Model> {
		let mut channels: Vec<_> = con.server.channels.values().collect();
		let mut clients: Vec<_> = con.server.clients.values().collect();
		channels.sort_by_key(|ch| ch.order);
		clients.sort_by_key(|c| -c.talk_power);
		// TODO Make more efficient?
		// TODO Also sort clients by name?

		// Get own client and channel
		let own_client = con.own_client;
		let own_channel = con.server.clients.get(&own_client).map(|c| c.channel)
			.unwrap_or(ChannelId(0));

		let is_talking = self.is_talking;
		let talking = if self.is_talking {
			"Stop talking"
		} else {
			"Start talking"
		};

		html! {
			<div class="channel-tree",>
				{ self.view_channel(&clients, &channels, ChannelId(0), own_client, own_channel) }
				<button onclick=|_| ConnectedMsg::SetTalking(!is_talking).into(),>{ talking }</button>
			</div>
		}
	}
}

impl Component for ChannelTree {
	type Message = ();
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		panic!()
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		false
	}
}
