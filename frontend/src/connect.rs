use qint_shared::*;
use stdweb::web::event::IEvent;
use yew::html;
use yew::prelude::*;

use crate::connection_service::{ConnectionId, ConnectionService, FrontendConnectionState};

/// Shows the login form
pub struct Connect {
	con: ConnectionId,
	onconnect: Option<Callback<ConnectOptions>>,
}

pub enum Msg {
	Connect,
	Change(Box<dyn FnOnce(&mut ConnectOptions)>),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub connection: ConnectionId,
	#[props(required)]
	pub onconnect: Callback<ConnectOptions>,
}

impl Component for Connect {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
		Self {
			con: props.connection,
			onconnect: Some(props.onconnect),
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Connect => {
				if let Some(c) = &mut self.onconnect {
					let opts = ConnectionService::with_mut_con(self.con, |con| if let
						FrontendConnectionState::Disconnected(options, _)
						= &mut con.state {
						options.clone()
					} else {
						panic!("Should be in disconnected state");
					}, || panic!("Should be in disconnected state"));
					c.emit(opts)
				}
			}
			Msg::Change(f) => {
				ConnectionService::with_mut_con(self.con, |con| if let
					FrontendConnectionState::Disconnected(options, _)
					= &mut con.state {
					f(options);
				} else {
					panic!("Should be in disconnected state");
				}, || panic!("Should be in disconnected state"));
			}
		}
		false
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		self.onconnect = Some(props.onconnect);
		if self.con != props.connection {
			self.con = props.connection;
			true
		} else {
			false
		}
	}

	fn view(&self) -> Html<Self> {
		ConnectionService::with_mut_con(self.con, |con| if let
			FrontendConnectionState::Disconnected(options, _) = &mut con.state {
			html! {
				<div class="connect-container">
				<div class="inner-connect-container">
				<div class="connect-blur"></div>
				<form class="connect-form" onsubmit=|e| { e.prevent_default(); Msg::Connect }>
					<div>
						<input name="username" class="input" type="text" placeholder="Username"
							value=&options.name
							oninput=|e| Msg::Change({
								Box::new(move |o| { o.name(e.value); })
							}), />
					</div>
					<div>
						<input name="server" class="input" type="text" placeholder="Server"
							value=&options.address
							oninput=|e| Msg::Change({
								Box::new(move |o| { o.address(e.value); })
							}), />
					</div>
					<div>
						<button class="button is-primary" name="connect" type="submit">
							{ "Connect" }
						</button>
					</div>
				</form>
				</div>
				</div>
			}
		} else {
			panic!("Should be in disconnected state");
		}, || panic!("Should be in disconnected state"))
	}
}

fn checkbox_value(e: &ChangeData) -> bool {
	if let ChangeData::Value(v) = e {
		v == "true"
	} else {
		false
	}
}