# From https://github.com/nix-community/naersk/blob/master/examples/cross-windows/flake.nix under MIT
{
  description = "Qint TeamSpeak client";

  inputs = {
    fenix.url = "github:nix-community/fenix";
    flake-utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nix-community/naersk";
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    naersk.inputs.nixpkgs.follows = "nixpkgs";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { self, nixpkgs, naersk, fenix, flake-utils }: flake-utils.lib.eachSystem [ "x86_64-linux" ] (system: let
    pkgs = (import nixpkgs) {
      inherit system;
    };
    lib = pkgs.lib;

    native-toolchain = with fenix.packages.${system};
      combine [
        minimal.rustc
        minimal.cargo
        complete.clippy
        latest.rustfmt
      ];

    mingw-toolchain = with fenix.packages.${system};
      combine [
        minimal.rustc
        minimal.cargo
        targets.x86_64-pc-windows-gnu.latest.rust-std
      ];

    naersk-lib = naersk.lib.${system}.override {
      cargo = native-toolchain;
      rustc = native-toolchain;
    };
    naersk-lib-win = naersk.lib.${system}.override {
      cargo = mingw-toolchain;
      rustc = mingw-toolchain;
    };

    sdlVersion = "2.26.5";

    sdl-mingw = pkgs.fetchurl {
      url = "https://www.libsdl.org/release/SDL2-devel-${sdlVersion}-mingw.tar.gz";
      hash = "sha256-sO/fq5+qrrS0nVFFEriV0W/a/XBJYDKhMr2fS5jw9ME=";
    };

    defaultBuildArgs = {
      pname = "qint";
      root = ./.;
      gitSubmodules = true;
      strictDeps = true;

      nativeBuildInputs = with pkgs; [
        cmake
        perl
        pkg-config
      ];

      # Deny warnings and clippy warnings
      RUSTFLAGS = [ "-Dwarnings" ];
    };

    defaultLinuxBuildArgs = defaultBuildArgs // {
      nativeBuildInputs = defaultBuildArgs.nativeBuildInputs ++ (with pkgs; [
        llvmPackages_latest.clang
        llvmPackages_latest.lld
      ]);

      buildInputs = with pkgs; [
        dbus
        glib-networking
        gtk3
        gtksourceview
        libappindicator
        libopus
        openssl
        SDL2
        webkitgtk
        zlib
      ];

      # Needed so bindgen can find libclang.so
      LIBCLANG_PATH="${pkgs.llvmPackages_latest.libclang.lib}/lib";

      doCheck = true;

      cargoTestOptions = opts: opts ++ [ "--" "--skip" "web::tests" ];

      cargoTestCommands = tests: tests ++ [
        # TODO Run clippy lints
        #"cargo $cargo_options clippy --all-targets"
        # Check formatting
        "cargo $cargo_options fmt --all --check"
      ];
    };

    # Create a fixed-output derivation from yarn install
    fetchYarnModulesPackage = { lib, stdenvNoCC, writeScript, yarn }: {
      pname,
      version,
      hash,
      packageJSON,
      yarnLock,
      yarnRc,
      yarnFolder,
      yarnFlags ? []
    }: stdenvNoCC.mkDerivation {
      inherit pname version packageJSON yarnLock yarnRc yarnFolder;

      yarnFlags = lib.escapeShellArgs yarnFlags;

      builder = writeScript "fetch-yarn-modules" ''
        source $stdenv/setup

        ln -s "$packageJSON" package.json
        cp "$yarnLock" yarn.lock
        cp "$yarnRc" .yarnrc.yml
        cp -r "$yarnFolder" .yarn
        chmod -R +w .yarn

        # yarn needs a home directory
        export HOME="$(mktemp -d)"

        yarn install --immutable $yarnFlags

        if [ -z "$skipPostFetch" ]; then
          runHook postFetch
        fi

        mv node_modules $out
      '';

      outputHashAlgo = null;
      outputHash = if hash == "" then lib.fakeHash else hash;
      outputHashMode = "recursive";

      nativeBuildInputs = [ yarn ];
    };

    fetchYarnModules = pkgs.callPackage fetchYarnModulesPackage {};

    yarnModules = fetchYarnModules {
      pname = "qint-frontend-modules";
      version = "1.0";
      hash = pkgs.lib.fakeHash;

      packageJSON = ./frontend/package.json;
      yarnLock = ./frontend/yarn.lock;
      yarnRc = ./frontend/.yarnrc.yml;
      yarnFolder = ./frontend/.yarn;
    };

    build-frontend = book_events: pkgs.runCommand "build-qint-frontend" {
      nativeBuildInputs = with pkgs; [ yarn ];
      src = ./frontend;
    } ''
      cp -r "$src/." .
      cp -r ${yarnModules} node_modules

      chmod -R +w .
      cp ${book_events} src/book_events.ts

      yarn run build

      mv dist $out
    '';

    win-pkg = naersk-lib-win.buildPackage (defaultBuildArgs // {
      depsBuildBuild = with pkgs; [
        pkgsCross.mingwW64.stdenv.cc
        pkgsCross.mingwW64.windows.pthreads
      ];

      # Only build webapp
      #cargoBuildOptions = opts: opts ++ [ "--package" "webapp" ];

      /*nativeBuildInputs = defaultBuildArgs.nativeBuildInputs ++ (with pkgs; [
        # We need Wine to run tests:
        wineWowPackages.stable
      ]);*/

      # Tells Cargo that we're building for Windows.
      # (https://doc.rust-lang.org/cargo/reference/config.html#buildtarget)
      CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

      # Fix some `extern` functions couldn't be found; some native libraries may need to be installed or have their path specified
      # when using C dependencies
      TARGET_CC = "x86_64-w64-mingw32-gcc";
      TARGET_CXX = "x86_64-w64-mingw32-g++";

      # Tells Cargo that it should use Wine to run tests.
      # (https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner)
      CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeShellScript "wine-wrapper" ''
        export WINEPREFIX="$(mktemp -d)"
        exec wine64 $@
      '';

      # Fix undefined reference to `__stack_chk_fail' and mingw ld segmentation fault.
      RUSTFLAGS = "-C link-args=-lssp -C link-args=-s";

      overrideMain = oldAttrs: oldAttrs // {
        # Production path
        FRONTEND_PATH = "./ui/";

        postPatch = ''
          substituteInPlace src-tauri/tauri.conf.json \
            --replace ../frontend/dist ${frontend}
        '';

        # Extract sdl
        preBuild = ''
          ${pkgs.gnutar}/bin/tar xf ${sdl-mingw}
          mkdir -p proxy-codegen/gnu-mingw/{lib,dll}/64
          mv SDL2-${sdlVersion}/x86_64-w64-mingw32/lib/*.a proxy-codegen/gnu-mingw/lib/64/
          mv SDL2-${sdlVersion}/x86_64-w64-mingw32/bin/SDL2.dll proxy-codegen/gnu-mingw/dll/64/
        '';

        postInstall = ''
          install -t $out/bin target/x86_64-pc-windows-gnu/release/build/webview2-com-sys-*/out/x64/WebView2Loader.dll
        '';
      };
    });

    qint = naersk-lib.buildPackage (defaultLinuxBuildArgs // {
      # Only set for main derivation
      overrideMain = oldAttrs: oldAttrs // {
        postPatch = ''
          substituteInPlace src-tauri/tauri.conf.json \
            --replace ../frontend/dist ${frontend}
        '';

        FRONTEND_PATH = frontend;

        # For tests
        RUST_BACKTRACE = "short";
      };
    });

    # Generate just book_events.ts
    book_events = naersk-lib.buildPackage (defaultLinuxBuildArgs // {
      cargoBuildOptions = opts: opts ++ [ "--package" "proxy-codegen" ];

      doCheck = false;

      overrideMain = oldAttrs: oldAttrs // {
        installPhase = ''
          mkdir -p $out
          cp frontend/src/book_events.ts $out/
        '';
      };
    });

    frontend = build-frontend "${book_events}/book_events.ts";
  in rec {
    defaultPackage = packages.win;

    packages.yarnModules = yarnModules;
    packages.frontend = frontend;

    packages.builtDeps = pkgs.runCommand "qint-dependencies" {} ''
      cat > $out <<EOF
        ${lib.concatStringsSep "," qint.builtDependencies}
        ${lib.concatStringsSep "," book_events.builtDependencies}
        ${lib.concatStringsSep "," win-pkg.builtDependencies}
      EOF
    '';

    packages.qint = qint;

    # The rust compiler is internally a cross compiler, so a single
    # toolchain can be used to compile multiple targets. In a hermetic
    # build system like nix flakes, there's effectively one package for
    # every permutation of the supported hosts and targets.
    # i.e.: nix build .#packages.x86_64-linux.x86_64-pc-windows-gnu
    # where x86_64-linux is the host and x86_64-pc-windows-gnu is the
    # target
    packages.win = win-pkg;

    packages.win-webapp-package = pkgs.runCommand "qint-win" {} ''
      mkdir -p $out
      mkdir -p Qint
      cp ${win-pkg}/bin/webapp.exe Qint/
      cp -ar ${frontend}/ Qint/ui

      # Add SDL
      ${pkgs.gnutar}/bin/tar xf ${sdl-mingw}
      mv SDL2-${sdlVersion}/x86_64-w64-mingw32/bin/SDL2.dll Qint/

      # Add libssp
      mingw_path="$(cat ${pkgs.pkgsCross.mingwW64.stdenv.cc}/nix-support/orig-cc)"
      cp "$mingw_path/x86_64-w64-mingw32/lib/libssp-0.dll" Qint/

      ${pkgs.zip}/bin/zip -r $out/Qint-webapp.zip Qint
    '';

    packages.win-tauri-package = pkgs.runCommand "qint-win" {} ''
      mkdir -p $out
      mkdir -p Qint
      cp ${win-pkg}/bin/qint.exe Qint/

      # Add SDL
      ${pkgs.gnutar}/bin/tar xf ${sdl-mingw}
      mv SDL2-${sdlVersion}/x86_64-w64-mingw32/bin/SDL2.dll Qint/

      # Add webview
      cp ${win-pkg}/bin/WebView2Loader.dll Qint/

      # Add libssp
      mingw_path="$(cat ${pkgs.pkgsCross.mingwW64.stdenv.cc}/nix-support/orig-cc)"
      cp "$mingw_path/x86_64-w64-mingw32/lib/libssp-0.dll" Qint/

      ${pkgs.zip}/bin/zip -r $out/Qint.zip Qint
    '';

    apps.default = apps.qint;

    apps.qint = flake-utils.lib.mkApp {
      name = "qint";
      drv = packages.qint;
    };

    apps.webapp = flake-utils.lib.mkApp {
      name = "webapp";
      drv = packages.qint;
    };

    # TODO Fix typos
    /*checks.typos = pkgs.runCommand "check-typos" {} ''
      ${pkgs.typos}/bin/typos ${self}
      mkdir -p $out
    '';*/

    checks.build = packages.qint;
  });
}
