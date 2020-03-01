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

	const RANGE_H: i32 = 360i32;
	const RANGE_S: i32 = 20i32;
	const RANGE_L: i32 = 20i32;

	let var_h = (((data[0] as i32) << 8u32) | ((data[1] as i32) << 0u32)) / RANGE_H;
	let var_s = 80i32 + (data[2] as i32) / RANGE_S; // = 90 ± 10 => [80-100]
	let var_l = 30i32 + (data[3] as i32) / RANGE_L; // = 40 ± 10 => [30- 50]
	format!("color:hsl({}, {}%, {}%);", var_h, var_s, var_l)
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
