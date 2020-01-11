use yew::html;
use yew::prelude::*;

pub struct Notification {
	pub content: Html,
}

pub struct Notifications {
	link: ComponentLink<Self>,
	notifications: Vec<Notification>,
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
			notifications: Vec::new(),
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
		}
	}

	fn view(&self) -> Html {
		html! {
			<div class="notifications">
				{ for self.notifications.iter().map(|n| self.view_notification(n)) }
			</div>
		}
	}
}

impl Notifications {
	fn view_notification(&self, notification: &Notification) -> Html {
		html! {}
	}
}
