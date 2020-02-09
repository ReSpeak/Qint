use failure::Error;
use yew::format::{Json, Nothing};
use yew::prelude::*;
use yew::services::fetch::{FetchService, FetchTask, Request, Response};

pub enum Msg {
	Ignore,
	GotPlugins(Vec<String>),
	RegisterCallback(CallbackInfo),
}

pub struct CallbackInfo {
	plugin: String,
}

pub struct Plugins {
	link: ComponentLink<Self>,
	plugins: Vec<String>,
	callbacks: Vec<CallbackInfo>,
	task: Option<FetchTask>,
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
		let name2 = name.to_string();
		let cb = self.link.callback(move |()| {
			Msg::RegisterCallback(CallbackInfo {
				plugin: name2.clone(),
			})
		});

		html! {
			<script type="module">
				{ format!("import('/plugins/{}').then(module => {{
					module.init();
				}});", name) }
			</script>
		}
	}
}

impl Component for Plugins {
	type Message = Msg;
	type Properties = ();

	fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
		let mut res = Self {
			link,
			plugins: Default::default(),
			callbacks: Default::default(),
			task: None,
		};
		res.load_list();
		res
	}

	fn update(&mut self, msg: Self::Message) -> ShouldRender {
		match msg {
			Msg::Ignore => false,
			Msg::GotPlugins(plugins) => {
				self.task = None;
				self.callbacks.clear();
				self.plugins = plugins;
				true
			}
			Msg::RegisterCallback(info) => {
				self.callbacks.push(info);
				false
			}
		}
	}

	fn view(&self) -> Html {
		html! {
			<>
				{ for self.plugins.iter().map(|p| self.view_script(&p)) }
			</>
		}
	}
}
