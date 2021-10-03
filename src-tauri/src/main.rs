// Don't show terminal in release mode
#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

#[macro_use]
extern crate qint_proxy;

mod audio;
mod cmd;
mod core;
mod filetransfer;

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use actix::prelude::*;
use anyhow::format_err;
use qint_proxy::QintState;
use structopt::StructOpt;
use tauri::PhysicalPosition;
use tauri::SystemTray;
use tauri::SystemTrayEvent;
use tauri::SystemTrayMenu;
use tauri::WindowEvent;
use tauri::WindowUrl;
use tauri::{CustomMenuItem, Manager};
use tokio::runtime::Runtime;

use crate::audio::LoudnessShare;
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
	tracing_subscriber::fmt::init();
	// TODO tracing stdlog

	// Parse command line options
	let args = Args::from_args();

	let (app_addr, app_arc) = {
		let (sender, receiver) = std::sync::mpsc::channel();

		thread::spawn(move || {
			let mut runtime = Runtime::new().unwrap();
			let local = tokio::task::LocalSet::new();
			let handle = runtime.handle().clone();
			local.block_on(&mut runtime, async move {
				let state = QintState::new(args.into()).unwrap();
				let app = QintCore::new(handle, state);
				let app_arc = Arc::new(app.clone());
				let app_addr = app.start();

				sender.send((app_addr.clone(), app_arc.clone())).unwrap();

				QintCore::run(app_addr, app_arc).await;
			});
		});

		receiver.recv().unwrap()
	};

	tauri::Builder::default()
		.manage(app_addr)
		.manage(app_arc.state.clone())
		.manage(app_arc)
		.manage(LoudnessShare::new())
		.create_window(
			"main",
			WindowUrl::App("index.html".into()),
			|window_builder, webview_attributes| (window_builder, webview_attributes),
		)
		.on_page_load(|window, _| {
			if let Err(e) = window.set_title("Qint") {
				println!("Failed to set title: {}", e);
			}

			window.listen("js-event", move |event| {
				println!("got js-event with message '{:?}'", event.payload());
			});
		})
		.system_tray(
			SystemTray::new().with_menu(SystemTrayMenu::new()
			.add_item(CustomMenuItem::new("show", "Show"))
			.add_item(CustomMenuItem::new("exit", "Exit")))
		)
		.on_system_tray_event(|app, event| match event {
			SystemTrayEvent::LeftClick { position: _, size: _, .. } => {
				let window = app.get_window("main").unwrap();
				window.set_skip_taskbar(false).unwrap();
				window.unminimize().unwrap();
				window.show().unwrap();
				window.set_focus().unwrap();
			}
			SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
				"show" => {
					let window = app.get_window("main").unwrap();
					window.show().unwrap();
				}
				"exit" => {
					app.exit(0);
				}
				_ => {}
			},
			_ => {}
		})
		.on_window_event(move |ev| {
			let window = ev.window();
			match ev.event() {
				WindowEvent::Resized(size) => { println!("Resize: {:?}", size); }
				WindowEvent::CloseRequested => { println!("Close requested"); }
				WindowEvent::Destroyed => { println!("Destroyed"); }
				WindowEvent::Moved(pos) => {
					println!("Moved {:?}", pos);
					if pos == &(PhysicalPosition { x: -32000, y: -32000 }) {
						println!("Minimized.");
						window.hide().unwrap();
					}
				}
				_ => {}
			}
		})
		.invoke_handler(tauri::generate_handler![
			cmd::create_ws,
			cmd::close_ws,
			cmd::pass_ws_msg,
			cmd::db,
			cmd::get_settings,
			cmd::set_settings,
			cmd::filetransfer_list,
			cmd::download_bytes,
			cmd::download_bytes_from_cache,
			cmd::upload_bytes,
			cmd::read_file,
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
			cmd::markdown,
			cmd::set_loudness_callback,
		])
		.run(tauri::generate_context!())
		.map_err(|e| format_err!("tauri error: {}", e))
		.unwrap();
}
