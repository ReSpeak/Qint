use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};

mod book_events;

use crate::book_events::BookEvents;

fn main() {
	let target = env::var("TARGET").unwrap();
	if target.contains("pc-windows") {
		let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
		let mut lib_dir = manifest_dir.clone();
		let mut dll_dir = manifest_dir.clone();
		if target.contains("msvc") {
			lib_dir.push("msvc");
			dll_dir.push("msvc");
		} else {
			lib_dir.push("gnu-mingw");
			dll_dir.push("gnu-mingw");
		}
		lib_dir.push("lib");
		dll_dir.push("dll");
		if target.contains("x86_64") {
			lib_dir.push("64");
			dll_dir.push("64");
		} else {
			lib_dir.push("32");
			dll_dir.push("32");
		}
		println!("cargo:rustc-link-search=all={}", lib_dir.display());
		println!("cargo:rerun-if-changed={}/SDL2.dll", dll_dir.display());
		for entry in std::fs::read_dir(dll_dir).expect("Can't read DLL dir, please add SDL") {
			let entry_path = entry.expect("Invalid fs entry").path();
			let file_name_result = entry_path.file_name();
			let mut new_file_path = manifest_dir.clone();
			if let Some(file_name) = file_name_result {
				let file_name = file_name.to_str().unwrap();
				if file_name.ends_with(".dll") {
					new_file_path.push(file_name);
					std::fs::copy(&entry_path, new_file_path.as_path())
						.expect("Can't copy SDL from DLL dir");
					println!("cargo:rerun-if-changed={}", new_file_path.display());
				}
			}
		}
	}

	let out_dir = env::var("OUT_DIR").unwrap();
	let path = Path::new(&out_dir);

	// Bookkeeping events
	let mut structs = File::create(&path.join("book_events.rs")).unwrap();
	write!(&mut structs, "{}", BookEvents::default()).unwrap();
}
