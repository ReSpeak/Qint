use yew::html;
use yew::prelude::*;

pub struct Notification {
	pub content: Html,
}

pub struct Notifications {
	link: ComponentLink<Self>,
}

pub enum Msg {
	Ignore,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
}

impl Component for Notifications {
	type Message = Msg;
	type Properties = Props;

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		Self {
			link,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
		}
	}

	fn view(&self) -> Html {
		html! {
			<>
			</>
		}
	}
}
