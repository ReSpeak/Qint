use ts_bookkeeping::{IconHash, UidRef};
use ts_bookkeeping::data::{Channel, Client, Server};
use yew::prelude::*;

use crate::{CHANNEL_ICON, CLIENT_ICON, SERVER_ICON};
use crate::connection_service::ConnectionId;

pub struct Icon;

impl Icon {
	pub fn server_icon(con: &ConnectionId, server: &Server) -> Html {
		Self::icon_hash(con, server.icon_id)
			.unwrap_or_else(|| Self::mdi_icon(SERVER_ICON))
	}

	pub fn channel_icon(con: &ConnectionId, channel: &Channel) -> Html {
		channel.icon_id
			.and_then(|i| Self::icon_hash(con, i))
			.unwrap_or_else(|| Self::mdi_icon(CHANNEL_ICON))
	}

	/// Choose avatar, client icon or generic icon.
	pub fn client_icon(con: &ConnectionId, client: &Client) -> Html {
		if !client.avatar_hash.is_empty() {
			Self::client_avatar(con, client.uid.as_ref())
		} else {
			Self::icon_hash(con, client.icon_id).unwrap_or_else(|| Self::mdi_icon(CLIENT_ICON))
		}
	}

	pub fn client_avatar(con: &ConnectionId, client_uid: UidRef) -> Html {
		Self::icon_intern("dummy",
			&format!("background-image: url(/file/{}/0/avatar_{})", con.0,
				client_uid.as_avatar()))
	}

	pub fn icon_hash(con: &ConnectionId, icon: IconHash) -> Option<Html> {
		if icon.0 != 0 {
			Some(Self::icon_intern("dummy",
				&format!("background-image: url(/file/{}/0/icon_{})", con.0, icon.0)))
		} else {
			None
		}
	}

	pub fn mdi_icon(name: &str) -> Html {
		Self::icon_intern(name, "")
	}

	fn icon_intern(name: &str, style: &str) -> Html {
		let name = if name.is_empty() { "dummy" } else { name };
		html! {
			<span class="icon is-small" style=style>
				<i class=format!("mdi mdi-{}", name)></i>
			</span>
		}
	}
}
