#![feature(async_await)]
#![recursion_limit="128"]

use failure::Error;
use serde::Deserialize;
use slog::{o, Drain};
use yew::{html, Component, ComponentLink, Html, Renderable, ShouldRender};
use yew::format::{Binary, Nothing, Json, Text, Toml};
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

use crate::connection::{WsConnection, ConnectionMsg};

mod connected;
mod connection;

type AsBinary = bool;

pub enum Format {
	Json,
	Toml,
}

pub struct Model {
	fetch_service: FetchService,
	link: ComponentLink<Model>,
	fetching: bool,
	data: Option<u32>,
	ft: Option<FetchTask>,

	connections: Vec<WsConnection>,
	/// The currently selected connection.
	current_con: usize,
}

pub enum Msg {
	FetchData(Format, AsBinary),
	FetchReady(Result<DataFromFile, Error>),
	Ignore,

	Connection(ConnectionMsg),
}

/// This type is used to parse data from `./static/data.json` file and
/// have to correspond the data layout from that file.
#[derive(Deserialize, Debug)]
pub struct DataFromFile {
	value: u32,
}

impl Component for Model {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let logger = slog::Logger::root(slog_stdlog::StdLog.fuse(), o!());

		Model {
			fetch_service: FetchService::new(),
			link,
			fetching: false,
			data: None,
			ft: None,

			connections: vec![WsConnection::new(logger)],
			current_con: 0,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::FetchData(format, binary) => {
				self.fetching = true;
				let task = match format {
					Format::Json => {
						let callback = self.link.send_back(move |response: Response<Json<Result<DataFromFile, Error>>>| {
							let (meta, Json(data)) = response.into_parts();
							println!("META: {:?}, {:?}", meta, data);
							if meta.status.is_success() {
								Msg::FetchReady(data)
							} else {
								Msg::Ignore  // FIXME: Handle this error accordingly.
							}
						});
						let request = Request::get("/data.json").body(Nothing).unwrap();
						if binary {
							self.fetch_service.fetch_binary(request, callback)
						} else {
							self.fetch_service.fetch(request, callback)
						}
					},
					Format::Toml => {
						let callback = self.link.send_back(move |response: Response<Toml<Result<DataFromFile, Error>>>| {
							let (meta, Toml(data)) = response.into_parts();
							println!("META: {:?}, {:?}", meta, data);
							if meta.status.is_success() {
								Msg::FetchReady(data)
							} else {
								Msg::Ignore  // FIXME: Handle this error accordingly.
							}
						});
						let request = Request::get("/data.toml").body(Nothing).unwrap();
						if binary {
							self.fetch_service.fetch_binary(request, callback)
						} else {
							self.fetch_service.fetch(request, callback)
						}
					},
				};
				self.ft = Some(task);
			}
			Msg::FetchReady(response) => {
				self.fetching = false;
				self.data = response.map(|data| data.value).ok();
			}
			Msg::Ignore => {
				return false;
			}

			Msg::Connection(cm) => {
				return self.connections[self.current_con].update(cm, &mut self.link);
			}
		}
		true
	}
}

impl Renderable<Model> for Model {
	fn view(&self) -> Html<Self> {
		let con = &self.connections[self.current_con];
		html! {
			<div>
				{ con.view() }
			</div>
		}
	}
}

#[derive(Debug)]
pub enum WsMsg {
	Text(Text),
	Binary(Binary),
}

impl From<Text> for WsMsg {
	fn from(t: Text) -> WsMsg { WsMsg::Text(t) }
}

impl From<Binary> for WsMsg {
	fn from(b: Binary) -> WsMsg { WsMsg::Binary(b) }
}
