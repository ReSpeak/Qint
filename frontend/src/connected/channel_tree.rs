use ts_bookkeeping::ChannelId;
use ts_bookkeeping::data::{Channel, Client, Connection};
use yew::html;
use yew::prelude::*;

use crate::Model;
use super::ConnectedMsg;

#[derive(Default)]
pub struct ChannelTree {
}

impl ChannelTree {
	fn view_client(&self, client: &Client) -> Html<Model> {
		html! {
			<li class="client",>{ &client.name }</li>
		}
	}

	fn view_channel(&self, clients: &[&Client], channels: &[&Channel], parent: ChannelId) -> Html<Model> {
		let this_channel = if let Some(channel) = channels.iter().find(|c| c.id == parent) {
			let id = channel.id;
			html! { <li class="channel", onclick=|_| ConnectedMsg::ChangeChannel(id).into(),>{ &channel.name }</li> }
		} else {
			html! { <></> }
		};

		html! {
			<>
				{ this_channel }
				<ul class="subchannels",>
					// Clients
					{ for clients.iter().filter(|c| c.channel == parent)
						.map(|c| self.view_client(c)) }
					// Channels
					{ for channels.iter().filter(|c| c.parent == parent)
						.map(|c| self.view_channel(clients, channels, c.id)) }
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

		html! {
			<div class="channel-tree",>
				{ self.view_channel(&clients, &channels, ChannelId(0)) }
			</div>
		}
	}
}
