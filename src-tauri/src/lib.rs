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
use tauri::AppHandle;
use tauri::Listener;
use tauri::Manager;
use tauri::WindowEvent;
#[cfg(desktop)]
use tauri::menu::{MenuBuilder, MenuItemBuilder};
#[cfg(desktop)]
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
#[cfg(desktop)]
use tauri::{PhysicalPosition, PhysicalSize};
#[cfg(mobile)]
use tauri_plugin_log::log::LevelFilter;
use tokio::runtime::Runtime;
use tracing::error;

use crate::audio::LoudnessShare;
use crate::core::QintCore;

#[derive(Clone, Debug, Default, StructOpt)]
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
	// Enable logging and panic messages on mobile
	#[cfg(mobile)]
	std::panic::set_hook(Box::new(move |panic| {
		let backtrace = std::backtrace::Backtrace::capture();

		let message = match (
			panic.payload().downcast_ref::<&str>(),
			panic.payload().downcast_ref::<String>(),
		) {
			(Some(s), _) => s.to_string(),
			(_, Some(s)) => s.clone(),
			(None, None) => "Unknown".to_string(),
		};

		let Some(l) = panic.location() else {
			error!("Panic: {:?}, message {}, backtrace:\n{:#?}", panic, message, backtrace);
			return;
		};

		error!(
			"Panic: {:?}, message {}, file: {}, line: {}, col: {}, backtrace:\n{:#?}",
			panic,
			message,
			l.file(),
			l.line(),
			l.column(),
			backtrace,
		);
	}));
	#[cfg(not(mobile))]
	tracing_subscriber::fmt::init();

	// Parse command line options
	#[cfg(not(mobile))]
	let args = Args::from_args();

	#[cfg(mobile)]
	let args = {
		let mut args = Args::default();
		// TODO Do not hardcode, but use Android API
		args.config_path = Some("/data/data/org.respeak.qint/files".into());
		args.cache_path = Some("/data/data/org.respeak.qint/cache".into());
		args
	};

	let (app_addr, app_arc, handle) = {
		let (sender, receiver) = std::sync::mpsc::channel();

		thread::spawn(move || {
			let mut runtime = Runtime::new().expect("Failed to create runtime");
			let local = tokio::task::LocalSet::new();
			let handle = runtime.handle().clone();
			local.block_on(&mut runtime, async move {
				let state = QintState::new(args.into()).expect("Failed to create QintState");
				let app = QintCore::new(handle.clone(), state);
				let app_arc = Arc::new(app.clone());
				let app_addr = app.start();

				sender
					.send((app_addr.clone(), app_arc.clone(), handle))
					.expect("Failed to send app addr");

				QintCore::run(app_arc).await;
			});
		});

		receiver.recv().expect("Failed to receive app addr")
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

	let builder = tauri::Builder::default();
	#[cfg(mobile)]
	let builder = builder.plugin(
		tauri_plugin_log::Builder::new()
			.level(LevelFilter::Warn)
			.level_for("tsproto", LevelFilter::Debug)
			.level_for("ts_bookkeeping", LevelFilter::Debug)
			.level_for("tsclientlib", LevelFilter::Debug)
			.level_for("qint_proxy", LevelFilter::Debug)
			.build(),
	);
	builder
		.plugin(tauri_plugin_dialog::init())
		.plugin(tauri_plugin_http::init())
		.plugin(tauri_plugin_notification::init())
		.plugin(tauri_plugin_opener::init())
		.manage(app_addr)
		.manage(app_arc.state.clone())
		.manage(app_arc.clone())
		.manage(LoudnessShare::new())
		.setup(|app| {
			#[cfg(desktop)]
			{
				let show = MenuItemBuilder::with_id("show", "Show").build(app)?;
				let exit = MenuItemBuilder::with_id("exit", "Exit").build(app)?;
				TrayIconBuilder::new()
					.icon_as_template(false)
					.icon(tauri::image::Image::from_bytes(include_bytes!("../../assets/32x32.png")).expect("Failed to load tray image"))
					.menu(&MenuBuilder::new(app).items(&[&show, &exit]).build()?)
					.on_menu_event(move |app, event| match event.id().as_ref() {
						"show" => {
							let window = app.get_webview_window("main").expect("Failed to get main webview");
							window.show().expect("Failed to show window");
							window.set_focus().expect("Failed to focus window");
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
					if let Some(true) = app_arc.state.settings.read().expect("Failed to get settings").get_close_to_tray() {
						if WINDOW_EVENT_DEBUG_PRINTS {
							println!("Closing to tray instead");
						}
						#[cfg(desktop)]
						window.hide().expect("Failed to hide window");
					} else {
						do_exit2(window.app_handle().clone());
					}
				}
				#[cfg(desktop)]
				WindowEvent::Focused(focus) => {
					if *focus && window.is_visible().expect("Failed to get is_visible") {
						let pos = window.inner_position().expect("Failed to get position");
						// Recover from Windows-D or Windows-M broken minimize
						if pos.x == -32000 && pos.y == -32000 {
							if WINDOW_EVENT_DEBUG_PRINTS {
								println!("Restore window to default position and size");
							}
							//window.set_skip_taskbar(false).unwrap();
							window.set_position(PhysicalPosition::new(300, 300)).expect("Failed to set position");
							window.set_size(PhysicalSize::new(800, 600)).expect("Failed to set size");
							// When we don't call show the minimize button can get bricked...
							window.show().expect("Failed to show window");
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
							app_arc.state.settings.read().expect("Failed to read settings").get_minimize_to_tray()
						{
							if WINDOW_EVENT_DEBUG_PRINTS {
								println!("Hide window");
							}
							window.hide().expect("Failed to hide window");
							// Skip taskbar is required with Windows-D / M on the "normal" window
							// design
							window.set_skip_taskbar(true).expect("Failed to set skip_taskbar");
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
