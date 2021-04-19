# Qint
Qint allows you to speak with other people over the internet.

## Screenshots
![main ui](https://share.splamy.de/20/02/firefox_2020-02-02_19-06-04.png)

## Dependencies
- [Rust](https://rust-lang.org), preferred installation method is [rustup](https://rustup.rs)
- [yarn](https://yarnpkg.com)
- [SDL2](https://www.libsdl.org), Windows installation guide is [below](#download-sdl2-on-windows)
- [OpenSSL](https://www.openssl.org) 1.1, on linux only
- [libopus](https://opus-codec.org), on linux only

### Windows
Run `./install_sdl.ps1`  
 \--- OR ---  
Download the `SDL2-devel-2.x.x-VC.zip` from [libsdl.org](https://www.libsdl.org).  
From this file, copy `SDL2-2.x.x/lib/x64/*.lib` to `Qint/proxy/msvc/lib/64/`.  
And copy `SDL2-2.x.x/lib/x64/*.dll` to `Qint/proxy/msvc/dll/64/` and `Qint/proxy/`.

### macOS
```bash
brew install sdl2 opus automake
```

### Ubuntu
```bash
apt install libopus-dev libsdl2-dev libwebkit2gtk-4.0-dev
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
env RUST_LOG=debug cargo run
# For release builds
cargo build --release
```

To activate logging for audio, use e.g. `RUST_LOG=debug,qint_proxy::audio::audio_to_ts=trace`.

By default, the proxy searches for the frontend in `../frontend/build`, where the frontend gets built by default. For packaging, it is useful to load the frontend for another directory, which can be set during compilation: `env FRONTEND_PATH=./frontend/ cargo build`

### Build the frontend
The backend needs to be built first because it autogenerates part of the frontend code.

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

On windows in `%appdata%\ReSpeak\config.toml`:
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

On Linux/X11, shortcuts are currently not implemented.
For Linux/wayland, configure your compositor to make http requests, e.g. by using curl:
```bash
curl -H "Content-Type: application/json" -X POST -d '{"InputMute":"Toggle"}' http://localhost:4422/shortcut
curl -H "Content-Type: application/json" -X POST -d '{"Away":"True"}' http://localhost:4422/shortcut
```

## License
Licensed under the [Open Software License](LICENSE-OSL) and [GNU Affero General Public License v3](LICENSE-AGPL).
