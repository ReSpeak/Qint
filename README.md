# Qint
Qint is a modern open-source alternative client for [TeamSpeak](https://teamspeak.com) servers that allows you to chat and speak with other people over the internet.

## Screenshots
### Login
![Login](https://share.splamy.de/21/12/firefox_2021-12-30_08-57-19.png)
### Main
![Main](https://share.splamy.de/21/12/firefox_2021-12-30_08-55-40.png)
### Search
![Search](https://share.splamy.de/21/12/firefox_2021-12-30_08-58-56.png)

### Filebrowser
![Filebrowser](https://share.splamy.de/21/12/firefox_2021-12-30_09-00-11.png)
### Channeleditor
![Channeledit](https://share.splamy.de/21/12/firefox_2021-12-30_09-03-21.png)

## Dependencies
- [Rust](https://rust-lang.org), preferred installation method is [rustup](https://rustup.rs)
- [yarn](https://yarnpkg.com)
- [SDL2](https://www.libsdl.org), Windows installation guide is [below](#windows)
- [OpenSSL](https://www.openssl.org) 1.1, on Linux only
- [libopus](https://opus-codec.org), on Linux only

### Windows
Run `./install_sdl.ps1`  
 \--- OR ---  
Download the `SDL2-devel-2.x.x-VC.zip` from [libsdl.org](https://www.libsdl.org).  
From this file, copy `SDL2-2.x.x/lib/x64/*.lib` to `proxy-codegen/msvc/lib/64/`.  
And copy `SDL2-2.x.x/lib/x64/*.dll` to `proxy-codegen/msvc/dll/64/`, `proxy-codegen/` and `src-tauri/`.

### macOS
```bash
brew install sdl2 opus automake
```

### Ubuntu
```bash
apt install libopus-dev libsdl2-dev libwebkit2gtk-4.0-dev libappindicator3-dev
```

## Clone
At the moment, tsclientlib is needed beside the Qint folder.
```bash
git clone https://github.com/ReSpeak/tsclientlib.git --recurse-submodules
git clone https://github.com/Flakebi/Qint.git
```

## Build and run Qint
### Build and start the backend
```bash
cd Qint/proxy
RUST_LOG=info cargo run
# For release builds
cargo build --release
```

To activate more logging for qint and see sent commands or packets, use
`RUST_LOG=tsproto=debug,ts_bookkeeping=debug,tsclientlib=debug,qint_proxy=debug,webapp=debug,warn cargo run -- -v`.

By default, the proxy searches for the frontend in `../frontend/build`, where the frontend gets
built by default. For packaging, it is useful to load the frontend for another directory, which can
be set during compilation: `FRONTEND_PATH=./frontend/ cargo build`

### Build the frontend
Make sure to build the backend once before building the frontend, because the backend build
autogenerates the `book_events.ts` file of the frontend.

```bash
cd Qint/frontend
# Install dependencies
yarn

# For the development server
yarn dev
# For builds
yarn build

# For checks
yarn typecheck
yarn lint
yarn format
```

Now, you can use the client at [http://localhost:4422](http://localhost:4422).

### Run with Tauri

Install `WebView2` like described on the tauri page: https://tauri.studio/en/docs/getting-started/setup-windows/#4-install-webview2

Build the frontend first, then run `cargo run` in the `src-tauri` folder.

### Enable logging in the frontend

By default, only errors are logged.
To change that, run one of the following in the browser console:
```js
// Log everything
debug.enable("*")
debug.enable("CHAT,BINPUT")
debug.enable("*,-LL") // Everything but the lazy list
debug.enable("error:*")

// debug.enable is saved in local storage
localStorage.debug = "*"
```

## Settings

### Configure Shortcuts

On Windows in `%appdata%\ReSpeak\config.toml`:
```toml
[[shortcuts.actions]]
keycode = "F13"
action = { InputMute = "Toggle" }

[[shortcuts.actions]]
keycode = "F12"
action = { OutputMute = "True" }

[[shortcuts.actions]]
keycode = "F11"
action = { Away = "False" }
```

On Linux/X11, global shortcuts are not implemented, using the same way as wayland below is possible.
For Linux/Wayland, configure your compositor to write to the socket or make http requests (http
requests do not work when running tauri), e.g. with the following commands:
```bash
# For the unix socket
echo '{"InputMute":null}' | nc -UN /tmp/qint-hotkeys

# For web requests
curl -H "Content-Type: application/json" -X POST -d '{"InputMute":null}' http://localhost:4422/shortcut
curl -H "Content-Type: application/json" -X POST -d '{"Away":null}' http://localhost:4422/shortcut
```

## License
Licensed under the [Open Software License](LICENSE-OSL) and [GNU Affero General Public License v3](LICENSE-AGPL).
