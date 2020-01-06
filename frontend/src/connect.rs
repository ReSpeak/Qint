use failure::Error;
use qint_shared::*;
use qint_shared::models::Bookmark;
use stdweb::web::event::IEvent;
use yew::format::{MsgPack, Nothing};
use yew::html;
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

/// Shows the login form
pub struct Connect {
	link: ComponentLink<Self>,
	options: ConnectOptions,
	/// If the options were changed since the start
	changed: bool,
	onconnect: Option<Callback<ConnectOptions>>,
	_bookmarks_fetch_task: FetchTask,
}

pub enum Msg {
	Ignore,
	GotBookmarks(Vec<Bookmark>),
	Connect,
	Change(Box<dyn FnOnce(&mut ConnectOptions)>),
}

#[derive(Clone, PartialEq, Properties)]
pub struct Props {
	#[props(required)]
	pub onconnect: Callback<ConnectOptions>,
}

impl Component for Connect {
	type Message = Msg;
	type Properties = Props;

	fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
		let mut fetch = FetchService::new();
		let request = Request::get(&format!("{}/bookmarks", crate::Model::get_http_domain()))
			.body(Nothing)
			.unwrap();
		let fetch_task = fetch.fetch_binary(request, link
			.callback(|resp: Response<MsgPack<Result<Vec<Bookmark>, Error>>>| {
				match resp.into_body().0 {
					Ok(r) => Msg::GotBookmarks(r),
					Err(e) => {
						// TODO Display error message
						log::error!("Failed to fetch bookmarks: {:?}", e);
						Msg::Ignore
					}
				}
			}));

		Self {
			link,
			options: ConnectOptions::new("localhost".into()),
			changed: false,
			onconnect: Some(props.onconnect),
			_bookmarks_fetch_task: fetch_task,
		}
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => {}
			Msg::GotBookmarks(bookmarks) => {
				if !self.changed {
					// TODO This does not work too well with paging
					if let Some(b) = bookmarks.iter()
						.filter(|b| b.last_used.is_some())
						.max_by_key(|b| b.last_used.unwrap()) {
						// Set options to last used connection
						self.options.name = b.username.clone();
						self.options.address = b.address.clone();
						return true;
					}
				}
			}
			Msg::Connect => {
				if let Some(c) = &mut self.onconnect {
					c.emit(self.options.clone())
				}
			}
			Msg::Change(f) => {
				self.changed = true;
				f(&mut self.options)
			}
		}
		false
	}

	fn change(&mut self, props: Self::Properties) -> ShouldRender {
		self.onconnect = Some(props.onconnect);
		false
	}

	fn view(&self) -> Html {
		let connect_submit = self.link.callback(|e: SubmitEvent| {
			e.prevent_default();
			Msg::Connect
		});
		let username_change = self.link.callback(|e: InputData| {
			Msg::Change(Box::new(move |o| { o.name(e.value); }))
		});
		let address_change = self.link.callback(|e: InputData| {
			Msg::Change(Box::new(move |o| { o.address(e.value); }))
		});

		html! {
			<div class="connect-container">
			<div class="inner-connect-container">
			<div class="connect-blur"></div>
			<form class="connect-form" onsubmit=connect_submit>
				<div>
					<input name="username" class="input" type="text" placeholder="Username"
						value=&self.options.name
						oninput=username_change />
				</div>
				<div>
					<input name="server" class="input" type="text" placeholder="Server"
						value=&self.options.address
						oninput=address_change />
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
	}
}

fn checkbox_value(e: &ChangeData) -> bool {
	if let ChangeData::Value(v) = e {
		v == "true"
	} else {
		false
	}
}
