# Qint
Qint allows you to speak with other people over the internet.

## Screenshots
TODO

## Dependencies
- [Rust](https://rust-lang.org), preferred installation method is [rustup](https://rustup.rs)
- [cargo-web](https://github.com/koute/cargo-web), install with `cargo install cargo-web`
- [SDL2](https://www.libsdl.org), Windows installation guide is [below](#download-sdl2-on-windows)
- [OpenSSL](https://www.openssl.org) 1.1, on linux only

## Clone
At the moment, tsclientlib is needed beside the Qint folder.
```bash
git clone https://github.com/ReSpeak/tsclientlib.git --recurse-submodules
git clone https://github.com/ReSpeak/Qint.git
```

## Download SDL2 on Windows
Download the `SDL2-devel-2.x.x-VC.zip` from [libsdl.org](https://www.libsdl.org).  
From this file, copy `SDL2-2.x.x/lib/x64/*.lib` to `Qint/proxy/msvc/lib/x64/`.  
And copy `SDL2-2.x.x/lib/x64/*.dll` to `Qint/proxy/msvc/dll/x64/`.

## Build and run Qint
### Build the frontend
```bash
cd Qint/frontend
cargo web build
```

### Build and start the backend
```bash
cd Qint/proxy
cargo run
```

Now, you can use the client at http://localhost:4422.

## License
Licensed under the [Open Software License](LICENSE-OSL) and [GNU Affero General Public License v3](LICENSE-AGPL).
