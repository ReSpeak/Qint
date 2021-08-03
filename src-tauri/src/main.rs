// Don't show terminal in release mode
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

#[macro_use]
extern crate qint_proxy;

mod cmd;
mod core;

use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use actix::prelude::*;
use anyhow::format_err;
use qint_proxy::QintState;
use slog::{o, Drain};
use structopt::StructOpt;
use tauri::SystemTray;
use tauri::SystemTrayEvent;
use tauri::SystemTrayMenu;
use tauri::{CustomMenuItem, Manager};
use tokio::runtime::Runtime;

use crate::core::QintCore;

#[derive(Clone, Debug, StructOpt)]
#[structopt(author, about)]
struct Args {
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
			listen_address: None,
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

	let mut runtime = Runtime::new().unwrap();

	let (addr, app) = {
		let (sender, receiver) = std::sync::mpsc::channel();

		let logger2 = logger.clone();
		thread::spawn(move || {
			let local = tokio::task::LocalSet::new();
			local.block_on(&mut runtime, async move {
				let app = QintState::new(logger2, args.into()).unwrap();
				let core = QintCore { state: app };

				sender.send((core.clone().start(), core)).unwrap();

				loop {
					tokio::time::sleep(Duration::from_secs(1)).await;
				}
			});
		});

		receiver.recv().unwrap()
	};

	tauri::Builder::default()
		.manage(addr)
		.manage(app.state)
		.manage(logger)
		.on_page_load(|window, _| {
			if let Err(e) = window.set_title("Qint") {
				println!("Failed to set title: {}", e);
			}

			window.listen("js-event", move |event| {
				println!("got js-event with message '{:?}'", event.payload());
			});
		})
		.system_tray(
			SystemTray::new().with_menu(
				SystemTrayMenu::new()
					.add_item(CustomMenuItem::new("toggle", "Toggle"))
					.add_item(CustomMenuItem::new("new", "New window")),
			),
		)
		.on_system_tray_event(|app, event| match event {
			SystemTrayEvent::LeftClick { position: _, size: _, .. } => {}
			SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
				"toggle" => {
					let window = app.get_window("main").unwrap();
					window.hide().unwrap();
				}
				"show" => {
					let window = app.get_window("main").unwrap();
					window.show().unwrap();
				}
				_ => {}
			},
			_ => {}
		})
		.invoke_handler(tauri::generate_handler![
			cmd::create_ws,
			cmd::pass_ws_msg,
			cmd::db,
			cmd::get_settings,
			cmd::set_settings,
			cmd::get_file,
			cmd::get_cache_file,
			cmd::download_file,
			cmd::upload_file,
			cmd::peek_link,
			cmd::get_audio_device_list,
			cmd::identity_create,
			cmd::identity_import,
			cmd::identity_list,
			cmd::identity_update,
			cmd::identity_delete,
			cmd::get_mutestate,
			cmd::run_hotkey,
			cmd::plugin_list,
			cmd::plugin_get,
			cmd::plugin_save,
			cmd::plugin_delete,
		])
		.run(tauri::generate_context!())
		.map_err(|e| format_err!("tauri error: {}", e))
		.unwrap();
}
