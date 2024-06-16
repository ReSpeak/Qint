// #[cfg(windows)]
// extern crate winres;

#[cfg(windows)]
fn main() {
	const ICO: &'static str = "../assets/icon.ico";

	if std::path::Path::new(ICO).exists() {
		let mut res = winres::WindowsResource::new();
		res.set_icon_with_id(ICO, "32512");
		res.set_manifest(r#"
		<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
			<dependency>
				<dependentAssembly>
					<assemblyIdentity
						type="win32"
						name="Microsoft.Windows.Common-Controls"
						version="6.0.0.0"
						processorArchitecture="*"
						publicKeyToken="6595b64144ccf1df"
						language="*"
					/>
				</dependentAssembly>
			</dependency>
		</assembly>"#);
		res.compile().expect("Unable to find visual studio tools");
	} else {
		panic!("No Icon.ico found. Please add one or check the path");
	}
}

#[cfg(not(windows))]
fn main() {}
