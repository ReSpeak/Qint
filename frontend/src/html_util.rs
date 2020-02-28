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
	let node = Node::try_from(div).map_err(|_| ())?; // TOD maybe a better error map
	Ok(VNode::VRef(node))
}

//const HEX_CONV: &'static [char] = &['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F'];
pub fn str_hash_to_color(text: &str) -> String {
	data_hash_to_color(text.as_bytes())
}

pub fn data_hash_to_color(data: &[u8]) -> String {
	let mut hash = 42u32;
	for &b in data {
		hash = hash.wrapping_mul(b as u32 + 1u32);
		hash = hash.rotate_left(b as u32);
	}
	// for i in 0..6 {
	// 	let index = ((0b1111 << (i * 4)) | hash) >> (i * 4);
	// 	s.push(HEX_CONV[u32::max(u32::min(index, 12), 3) as usize]);
	// }
	const RANGE_H: u32 = 360u32;
	const RANGE_S: u32 = 20u32;
	const RANGE_L: u32 = 20u32;

	let var_h = hash % RANGE_H;
	hash /= RANGE_H;
	let var_s = (90i32 + ((hash % RANGE_S) as i32 - 10i32)) as u32; // = 90 ± 10 => [80-100]
	hash /= RANGE_S;
	let var_l = (40i32 + ((hash % RANGE_L) as i32 - 10i32)) as u32; // = 40 ± 10 => [30-50]
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
