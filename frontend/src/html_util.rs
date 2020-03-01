use qint_shared::models::Message;
use ts_bookkeeping::data::Client;
use stdweb::unstable::TryFrom;
use stdweb::web::Node;
use yew::prelude::*;
use yew::virtual_dom::VNode;

pub fn html_from_string(html: &str) -> Result<Html, ()> {
	let div = js! {
		var div = document.createElement("div");
		div.innerHTML = @{html};
		return div;
	};
	let node = Node::try_from(div).map_err(|_| ())?; // TODO maybe a better error map
	Ok(VNode::VRef(node))
}

pub fn str_hash_to_color(text: &str) -> String {
	data_hash_to_color(text.as_bytes())
}

pub fn data_hash_to_color(data: &[u8]) -> String {
	if data.len() < 4 {
		return String::from("color:black;");
	}
	let var_h = (((data[0] as i32) << 8u32) | ((data[1] as i32) << 0u32)) % 360i32;
	let var_s = 60i32 + (data[2] as i32) % 40i32; // = 80 ± 20 => [60-100]
	let var_l = 30i32 + (data[3] as i32) % 30i32; // = 45 ± 15 => [30- 60]
	format!("color:hsl({}, {}%, {}%);", var_h, var_s, var_l)
}

pub trait MessageExtensions {
	fn get_user_name(&self) -> &str;
}

impl MessageExtensions for Message {
	fn get_user_name(&self) -> &str {
		if let Some(name) = &self.client_name {
			name
		} else if let Some(name) = &self.invoker_name {
			name
		} else {
			"Anonymous"
		}
	}
}

pub trait ToColor {
	fn to_color(&self) -> String;
}

impl ToColor for Message {
	fn to_color(&self) -> String {
		if let Some(ref uid) = self.invoker {
			data_hash_to_color(uid)
		} else {
			str_hash_to_color(self.get_user_name())
		}
	}
}

impl ToColor for Client {
	fn to_color(&self) -> String {
		data_hash_to_color(&self.uid.0)
	}
}

#[macro_export]
macro_rules! bulma_icon {
	(= $x:expr) => {
		html! {
			<span class="icon">
				{ $x }
			</span>
		}
	};
	($x:expr) => { $crate::bulma_icon!($x, 18) };
	($x:expr, $size: expr) => {
		html! {
			<span class="icon">
				<i class={concat!("mdi mdi-", $size, "px mdi-", $x)}>
				</i>
			</span>
		}
	};
}
