use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod book_events;

use crate::book_events::{BookEvents, BookEventsTs};

fn main() {
	let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

	let out_dir = env::var("OUT_DIR").unwrap();
	let path = Path::new(&out_dir);

	// Bookkeeping events
	let mut structs = File::create(&path.join("book_events.rs")).unwrap();
	let events = BookEvents::default();
	write!(&mut structs, "{}", events).unwrap();

	let path = manifest_dir.join("..").join("frontend").join("src");
	std::fs::create_dir_all(&path).unwrap();
	let mut structs = File::create(&path.join("book_events.ts")).unwrap();
	write!(&mut structs, "{}", BookEventsTs(events)).unwrap();
}
