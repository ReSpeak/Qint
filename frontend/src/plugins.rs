use std::collections::HashMap;

use failure::Error;
use ts_bookkeeping::events::Event;
use serde::{Deserialize, Serialize};
use stdweb::{js, js_serializable, Value};
use uuid::Uuid;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use crate::connection_service::{ConnectionId, ConnectionService, FrontendConnectionState};

#[derive(Deserialize, Serialize)]
struct Connection(ts_bookkeeping::data::Connection);

js_serializable!(Connection);

pub enum Msg {
	Ignore,
	GotPlugins(Vec<String>),
	AddEventListener(String, CallbackInfo),
	Events(Vec<Event>),
}

pub struct CallbackInfo {
	plugin: String,
	callback: Value,
}

pub struct Plugins {
	link: ComponentLink<Self>,
	plugins: Vec<String>,
	event_listeners: HashMap<String, Vec<CallbackInfo>>,
	task: Option<FetchTask>,
	con: Option<ConnectionId>,
	talkers: Vec<u16>,
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	pub connection: Option<ConnectionId>,
	pub talkers: Vec<u16>,
}

impl Plugins {
	fn load_list(&mut self) {
		let req = Request::get("/plugins").body(Nothing).unwrap();

		self.task = Some(FetchService::new().fetch(req,
			self.link.callback(|res: Response<Json<Result<Vec<String>, Error>>>| {
				if let Json(Ok(plugins)) = res.into_body() {
					Msg::GotPlugins(plugins)
				} else {
					Msg::Ignore
				}
			})));
	}

	fn view_script(&self, name: &str) -> Html {
		html! {
			<script type="module">
				{ format!("import('/plugins/{}').then(module => {{
					module.init(window.qintPluginApi.getApi('{0}'));
				}});", name.replace('\\', "\\\\").replace('\'', "\\'")) }
			</script>
		}
	}

	fn add_listener(&self) {
		if let Some(con) = &self.con {
			ConnectionService::with_mut(con, |con| {
				let callback = self.link.callback(|e| Msg::Events(e));
				con.event_listeners.insert("plugins".into(), Box::new(move |_, events| {
					callback.emit(events.to_vec());
				}));
			}, || panic!("Should be in connected state"));
		}
	}

	fn talkers_changed(&self) {
		if let Some(listeners) = self.event_listeners.get("TalkersChanged") {
			let con = self.con.as_ref().unwrap().0.to_string();
			for l in listeners {
				js!{ @(no_return)
					try {
						@{&l.callback}(@{&con}, @{&self.talkers});
					} catch {
						// TODO Print
						console.error("Callback throws exception");
					}
				};
			}
		}
	}
}

impl Component for Plugins {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let mut res = Self {
			link,
			plugins: Default::default(),
			event_listeners: Default::default(),
			task: None,
			con: props.connection,
			talkers: props.talkers,
		};

		if res.con.is_some() {
			res.add_listener();
		}

		let event_listener_cb = res.link.callback(|(plugin, event, callback)| {
			Msg::AddEventListener(event, CallbackInfo {
				plugin,
				callback,
			})
		});
		let add_event_listener = move |plugin: String, event: String, listener: Value| {
			event_listener_cb.emit((plugin, event, listener));
		};

		// Returns the bookkeeping or null if the connection does not exist
		let get_state = move |_plugin: String, con: String| {
			let con = ConnectionId(match Uuid::parse_str(&con) {
				Ok(r) => r,
				Err(_) => {
					stdweb::console!(error, format!("Failed to parse connection id {:?}", con));
					return Value::Undefined;
				}
			});

			ConnectionService::with(&con, |c| {
				if let FrontendConnectionState::Connected(c) = &c.state {
					match stdweb::private::to_value(&c.con) {
						Ok(r) => r,
						Err(e) => {
							stdweb::console!(error, format!("Failed to serialize: {:?}", e));
							Value::Undefined
						}
					}
				} else {
					Value::Null
				}
			}, || {
				stdweb::console!(error, format!("Connection {:?} not found", con));
				Value::Undefined
			})
		};

		// All these methods need to be droped again in `destroy`. Otherwise,
		// they leak memory.
		js!{ @(no_return)
			window.qintPluginApi = {
				addEventListener: @{add_event_listener},
				getState: @{get_state},
				getApi(plugin) {
					return {
						addEventListener(event, listener) {
							window.qintPluginApi.addEventListener(plugin, event, listener);
						},
						getState(con) {
							return window.qintPluginApi.getState(plugin, con);
						}
					};
				}
			};
		};

		res.load_list();
		res
	}

	fn destroy(&mut self) {
		js!{ @(no_return)
			window.qintPluginApi.addEventListener.drop();
			window.qintPluginApi.getState.drop();
			window.qintPluginApi = undefined;
		}

		if let Some(con) = &self.con {
			ConnectionService::with_mut(con, |con| {
				con.packet_listeners.remove("plugins");
			}, || {});
		}
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		if self.con != props.connection {
			// Remove and add listener
			if let Some(con) = &self.con {
				ConnectionService::with_mut(con, |con| {
					con.packet_listeners.remove("plugins");
				}, || {});
			}

			self.con = props.connection;
			self.add_listener();
		}

		if self.talkers != props.talkers {
			self.talkers = props.talkers;
			self.talkers_changed();
		}

		false
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => {}
			Msg::GotPlugins(plugins) => {
				self.task = None;
				self.event_listeners.clear();
				self.plugins = plugins;
				return true;
			}
			Msg::AddEventListener(event, info) => {
				let listeners = self.event_listeners.entry(event)
					.or_insert_with(Default::default);
				listeners.push(info);
			}
			Msg::Events(_e) => {
				// TODO
			}
		}
		false
	}

	fn view(&self) -> Html {
		html! {
			<>
				{ for self.plugins.iter().map(|p| self.view_script(&p)) }
			</>
		}
	}
}
