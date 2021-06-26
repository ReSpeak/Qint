// #[cfg(windows)]
// extern crate winres;

#[cfg(windows)]
fn main() {
	const ICO: &'static str = "../assets/icon.ico";

	if std::path::Path::new(ICO).exists() {
		let mut res = winres::WindowsResource::new();
		res.set_icon_with_id(ICO, "32512");
		res.compile().expect("Unable to find visual studio tools");
	} else {
		panic!("No Icon.ico found. Please add one or check the path");
	}
}

#[cfg(not(windows))]
fn main() {}
