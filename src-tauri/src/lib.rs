mod audio;
mod cmd;
mod core;
mod filetransfer;

use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use actix::prelude::*;
use qint_proxy::QintState;
use structopt::StructOpt;
#[cfg(desktop)]
use tauri::menu::{MenuBuilder, MenuItemBuilder};
#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::AppHandle;
use tauri::Listener;
use tauri::Manager;
use tauri::WindowEvent;
#[cfg(desktop)]
use tauri::{PhysicalPosition, PhysicalSize};
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

const WINDOW_EVENT_DEBUG_PRINTS: bool = false;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
	tracing_subscriber::fmt::init();
	// TODO tracing stdlog

	// Parse command line options
	let args = Args::from_args();

	let (app_addr, app_arc, handle) = {
		let (sender, receiver) = std::sync::mpsc::channel();

		thread::spawn(move || {
			let mut runtime = Runtime::new().unwrap();
			let local = tokio::task::LocalSet::new();
			let handle = runtime.handle().clone();
			local.block_on(&mut runtime, async move {
				let state = QintState::new(args.into()).unwrap();
				let app = QintCore::new(handle.clone(), state);
				let app_arc = Arc::new(app.clone());
				let app_addr = app.start();

				sender.send((app_addr.clone(), app_arc.clone(), handle)).unwrap();

				QintCore::run(app_arc).await;
			});
		});

		receiver.recv().unwrap()
	};

	let state = app_arc.state.clone();
	let do_exit = move |app: AppHandle| {
		let state = state.clone();
		handle.spawn(async move {
			state.close_all().await;
			app.exit(0);
		});
	};
	let do_exit2 = do_exit.clone();

	tauri::Builder::default()
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_http::init())
		.plugin(tauri_plugin_notification::init())
		.plugin(tauri_plugin_shell::init())
		.manage(app_addr)
		.manage(app_arc.state.clone())
		.manage(app_arc.clone())
		.manage(LoudnessShare::new())
		.setup(|app| {
			#[cfg(desktop)]
			{
				let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
				let exit = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
				TrayIconBuilder::with_id("qint")
					.menu(&MenuBuilder::new(app).items(&[&show, &exit]).build()?)
					.on_menu_event(move |app, event| match event.id().as_ref() {
						"show" => {
							let window = app.get_webview_window("main").unwrap();
							window.show().unwrap();
							window.set_focus().unwrap();
						}
						"exit" => {
							do_exit(app.clone());
						}
						_ => {}
					})
					.on_tray_icon_event(move |tray, event| match event {
						TrayIconEvent::Click {
							button: MouseButton::Left,
							button_state: MouseButtonState::Up,
							..
						} => {
							if let Some(window) = tray.app_handle().get_webview_window("main") {
								let _ = window.set_skip_taskbar(false);
								let _ = window.unminimize();
								let _ = window.show();
								let _ = window.set_focus();
							}
						}
						_ => {}
					})
					.build(app)?;
			}

			Ok(())
		})
		.on_page_load(|window, _| {
			window.listen("js-event", move |event| {
				println!("got js-event with message '{:?}'", event.payload());
			});
		})
		.on_window_event(move |window, ev| {
			match ev {
				WindowEvent::Resized(_size) => {}
				WindowEvent::CloseRequested { api, .. } => {
					if WINDOW_EVENT_DEBUG_PRINTS {
						println!("Close requested");
					}
					api.prevent_close();
					if let Some(true) = app_arc.state.settings.read().unwrap().get_close_to_tray() {
						if WINDOW_EVENT_DEBUG_PRINTS {
							println!("Closing to tray instead");
						}
						#[cfg(desktop)]
						window.hide().unwrap();
					} else {
						do_exit2(window.app_handle().clone());
					}
				}
				#[cfg(desktop)]
				WindowEvent::Focused(focus) => {
					if *focus && window.is_visible().unwrap() {
						let pos = window.inner_position().unwrap();
						// Recover from Windows-D or Windows-M broken minimize
						if pos.x == -32000 && pos.y == -32000 {
							if WINDOW_EVENT_DEBUG_PRINTS {
								println!("Restore window to default position and size");
							}
							//window.set_skip_taskbar(false).unwrap();
							window.set_position(PhysicalPosition::new(300, 300)).unwrap();
							window.set_size(PhysicalSize::new(800, 600)).unwrap();
							// When we don't call show the minimize button can get bricked...
							window.show().unwrap();
						}
					}
				}
				WindowEvent::Destroyed => {
					if WINDOW_EVENT_DEBUG_PRINTS {
						println!("Destroyed");
					}
				}
				#[cfg(desktop)]
				WindowEvent::Moved(pos) => {
					// Handling Windows minimize
					// Caused by
					// - pressing the UI minimize button on the windows "native" design
					// - pressing Windows-D or Windows-M
					if pos.x == -32000 && pos.y == -32000 {
						if let Some(true) =
							app_arc.state.settings.read().unwrap().get_minimize_to_tray()
						{
							if WINDOW_EVENT_DEBUG_PRINTS {
								println!("Hide window");
							}
							window.hide().unwrap();
							// Skip taskbar is required with Windows-D / M on the "normal" window
							// design
							window.set_skip_taskbar(true).unwrap();
						} else if WINDOW_EVENT_DEBUG_PRINTS {
							println!("Windows minimize");
						}
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
		// This allows triggering hotkeys while Window is focused
		.device_event_filter(tauri::DeviceEventFilter::Always)
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
