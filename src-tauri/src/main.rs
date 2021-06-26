// Don't show terminal in release mode
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod cmd;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::{format_err, Result};
use cmd::TauriMsg;
use qint_proxy::messages::MessageF2P;
use qint_proxy::App;
use qint_proxy::ConnectionId;
use slog::{o, Drain};
use structopt::StructOpt;
use tauri::async_runtime::channel;
use tauri::command;
use tauri::State;
use tauri::{CustomMenuItem, Manager, SystemTrayMenuItem};
use tokio::runtime::Runtime;
use uuid::Uuid;

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
struct Args {
	/// The address where the server listens
	#[structopt(short = "l", long)]
	listen_address: Option<SocketAddr>,
	/// The id of the identity that is used by default
	#[structopt(short = "i", long)]
	default_identity: Option<u64>,
	/// The path for all the settings files. This makes only senses as a command line argument, it
	/// is ignored in the settings file.
	///
	/// If no value is given, the configuration path depends on the operating system.
	#[structopt(short = "c", long)]
	config_path: Option<PathBuf>,
	/// The path for cached files. This is used for the `FileCache`.
	///
	/// If no value is given, the configuration path depends on the operating system.
	#[structopt(long)]
	cache_path: Option<PathBuf>,
	/// The path for plugins.
	///
	/// If no value is given, this is the path of the config file plus `plugins/`.
	#[structopt(long)]
	plugin_path: Option<String>,
	/// Do not capture and play audio.
	// This is used for testing, which cannot initialize SDL.
	// SDL must only be initialized once per process, at the same time, it can only be used from a
	// single thread, which does not work well with parallel tests.
	#[structopt(long)]
	no_audio: bool,
	/// Do not open database to search messages.
	#[structopt(long)]
	pub no_search: bool,
	/// Do not cache link previews.
	#[structopt(long)]
	pub no_link_cache: bool,
	/// How much log output do you want?
	///
	/// 0. Print nothing
	/// 1. Print command string
	/// 2. Print packets
	/// 3. Print udp packets
	#[structopt(short = "v", long, parse(from_occurrences))]
	verbosity: u8,
}

impl Into<qint_proxy::Args> for Args {
	fn into(self) -> qint_proxy::Args {
		qint_proxy::Args {
			listen_address: self.listen_address,
			default_identity: self.default_identity,
			config_path: self.config_path,
			cache_path: self.cache_path,
			plugin_path: self.plugin_path,
			no_audio: self.no_audio,
			no_search: self.no_search,
			no_link_cache: self.no_link_cache,
			verbosity: self.verbosity,
		}
	}
}

fn main() {
	let logger = {
		let decorator = slog_term::TermDecorator::new().build();
		let drain = slog_term::CompactFormat::new(decorator).build();
		let drain = slog_envlogger::new(drain).fuse();
		let drain = slog_async::Async::new(drain).build().fuse();

		slog::Logger::root(drain, o!())
	};

	let _scope_guard = slog_scope::set_global_logger(logger.clone());
	// Ignore errors if a logger has already been set
	let _ = slog_stdlog::init();

	// Parse command line options
	let args = Args::from_args();

	let app = {
		let (sender, receiver) = std::sync::mpsc::channel();

		let mut runtime = Runtime::new().unwrap();
		thread::spawn(move || {
			let local = tokio::task::LocalSet::new();
			local.block_on(&mut runtime, async move {
				let app = App::new(logger.clone(), args.into()).unwrap();

				sender.send(app).unwrap();

				app.0.accept_ws().await;
			});
		});

		receiver.recv().unwrap()
	};

	//let window_thread_handle = std::thread::spawn(|| );
	//app.serve().await;
	//window_thread_handle.join().unwrap();

	tauri::Builder::default()
		.manage(app)
		.on_page_load(|window, _| {
			if let Err(e) = window.set_title("Qint") {
				println!("Failed to set title: {}", e);
			}
			/*if let Err(e) = window.set_icon(Icon::File("../frontend/public/128x128.png".into())) {
				println!("Failed to set icon: {}", e);
			}*/
			//let window_ = window.clone();
			window.on_window_event(move |e| {
				//println!("Scale factor: {:?}", window_.scale_factor());
				//println!("Window event: {:?}", e);
			});
			window.listen("js-event", move |event| {
				println!("got js-event with message '{:?}'", event.payload());
				//let reply = Reply { data: "something else".to_string() };

				//window_.emit("rust-event", Some(reply)).expect("failed to emit");
			});
		})
		.system_tray(vec![
			SystemTrayMenuItem::Custom(CustomMenuItem::new("toggle".into(), "Show/Hide")),
			SystemTrayMenuItem::Custom(CustomMenuItem::new("show".into(), "New window")),
		])
		.on_system_tray_event(|app, event| {
			match event.menu_item_id().as_str() {
				"toggle" => {
					let window = app.get_window("main").unwrap();
					// TODO: window.is_visible API
					window.hide().unwrap();
					// if let Err(e) = Notification::new("qint")
					// 	.title("Window hidden")
					// 	.body("The window has gone 😯")
					// 	.show()
					// {
					// 	println!("Failed to show notification");
					// }
				}
				"show" => {
					let window = app.get_window("main").unwrap();
					/*println!("Get monitor");
					let window2 = window.clone();
					std::thread::spawn(move || {
						match window2.current_monitor() {
							Err(e) => println!("Failed to get monitor: {}", e),
							Ok(None) => println!("No current monitor"),
							Ok(Some(monitor)) => {
								println!("Got monitor");
								println!("Scale factor: {}", monitor.scale_factor());
							}
						}
					});*/
					// TODO: window.is_visible API
					window.show().unwrap();
				}
				_ => {}
			}
		})
		.invoke_handler(tauri::generate_handler![cmd::create_ws, cmd::pass_ws_msg,])
		.run(tauri::generate_context!())
		.map_err(|e| format_err!("tauri error: {}", e))
		.unwrap();
}
