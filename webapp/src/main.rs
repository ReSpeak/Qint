#[macro_use]
extern crate qint_proxy;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;

use anyhow::Result;
use qint_proxy::QintState;
use slog::{debug, error, o, Drain};
use structopt::StructOpt;
use web::WebApp;

mod web;
mod websocket;

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
	no_search: bool,
	/// Do not cache link previews.
	#[structopt(long)]
	no_link_cache: bool,
	/// Open the frontend in the browser on start.
	#[structopt(long)]
	no_open: bool,
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

#[allow(unused_braces)]
#[actix_rt::main]
async fn main() -> Result<()> { real_main().await }

async fn real_main() -> Result<()> {
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
	let no_open = args.no_open;

	let app = WebApp::new(QintState::new(logger.clone(), args.into())?);

	if !no_open {
		// Open browser
		let addr = app.get_listen_address();
		let port = addr.port();
		let token = app.get_token().to_string();
		actix::spawn(async move {
			// Connect to localhost if == 0.0.0.0 or ::
			let url = if addr.ip() == "0.0.0.0".parse::<IpAddr>().unwrap()
				|| addr.ip() == "::".parse::<IpAddr>().unwrap()
				|| addr.ip() == "127.0.0.1".parse::<IpAddr>().unwrap()
				|| addr.ip() == "::1".parse::<IpAddr>().unwrap()
			{
				format!("http://localhost:{}", port)
			} else {
				format!("http://{}", addr)
			};
			let url = format!("{}/?token={}", url, token);
			debug!(logger, "Opening url"; "url" => &url);
			if let Err(e) = open::that(url) {
				error!(logger, "Failed to open frontend in browser"; "error" => %e);
			}
		});
	}

	app.serve().await
}
