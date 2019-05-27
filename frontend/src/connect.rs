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
	Change(Box<FnOnce(&mut ConnectOptions)>),
}

#[derive(Clone, Default, PartialEq)]
pub struct Props {
	pub connection: Option<ConnectionId>,
	pub onconnect: Option<Callback<ConnectOptions>>,
}

impl Component for Connect {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, _: ComponentLink<Self>) -> Self {
		let con = props.connection.expect("Connect needs a connection id");

		Self {
			con,
			onconnect: props.onconnect,
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
		self.onconnect = props.onconnect;
		let con = props.connection.expect("Connect needs a connection id");
		if self.con != con {
			self.con = con;
			true
		} else {
			false
		}
	}
}

fn checkbox_value(e: &ChangeData) -> bool {
	if let ChangeData::Value(v) = e {
		v == "true"
	} else {
		false
	}
}

impl Renderable<Self> for Connect {
	fn view(&self) -> Html<Self> {
		ConnectionService::with_mut_con(self.con, |con| if let
			FrontendConnectionState::Disconnected(options, _) = &mut con.state {
			html! {
				<div class="connect-container",>
				<form class="connect-form", onsubmit=|e| { e.prevent_default(); Msg::Connect },>
					<div class="connect-item",>
						<input name="username", type="text", placeholder="Username",
							value=&options.name,
							oninput=|e| Msg::Change({
								Box::new(move |o| { o.name(e.value); })
							}), />
					</div>
					<div class="connect-item",>
						<input name="server", type="text", placeholder="Server",
							value=&options.address,
							oninput=|e| Msg::Change({
								Box::new(move |o| { o.address(e.value); })
							}), />
					</div>
					<div class="connect-item",>
						<label>
							<input name="log-commands", type="checkbox", value="true",
								onchange=|e| Msg::Change({
									Box::new(move |o| { o.log_commands(checkbox_value(&e)); })
								}), />
							{ "Log commands" }
						</label>
					</div>
					<div class="connect-item",>
						<label>
							<input name="log-packets", type="checkbox", value="true",
								onchange=|e| Msg::Change({
									Box::new(move |o| { o.log_packets(checkbox_value(&e)); })
								}), />
							{ "Log packets" }
						</label>
					</div>
					<div class="connect-item",>
						<button name="connect", type="submit",>
							{ "Connect" }
						</button>
					</div>
				</form>
				</div>
			}
		} else {
			panic!("Should be in disconnected state");
		}, || panic!("Should be in disconnected state"))
	}
}
