# From https://github.com/nix-community/naersk/blob/master/examples/cross-windows/flake.nix under MIT
{
  description = "Qint TeamSpeak client";

  inputs = {
    naersk = {
      url = "github:yusdacra/naersk/feat/cargolock-git-deps";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils = {
      url = "github:numtide/flake-utils";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, naersk, fenix, flake-utils }: flake-utils.lib.eachDefaultSystem (system: let
    pkgs = nixpkgs.legacyPackages.${system};
    toolchain = with fenix.packages.${system}; combine [
      minimal.rustc
      minimal.cargo
      targets.x86_64-pc-windows-gnu.latest.rust-std
    ];
    naersk-lib = naersk.lib.${system};
    naersk-lib-win = naersk.lib.${system}.override {
      cargo = toolchain;
      rustc = toolchain;
    };

    defaultBuildArgs = {
      pname = "qint";
      root = ./.;
      gitSubmodules = true;

      nativeBuildInputs = with pkgs; [
        cmake
        perl
        pkg-config
      ];
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
    };

    # Pre-generate book_events.ts
    book_events = naersk-lib.buildPackage (defaultLinuxBuildArgs // {
      overrideMain = oldAttrs: {
        cargo_build_options = [ "$cargo_release" ''-j "$NIX_BUILD_CORES"'' "--out-dir" "out" "--message-format=$cargo_message_format" "-p" "proxy-codegen" ];

        installPhase = ''
          cp frontend/src/book_events.ts $out
        '';
      };
    });
  in rec {
    defaultPackage = packages.x86_64-pc-windows-gnu;

    packages.book_events = book_events;
    packages.frontend = pkgs.mkYarnPackage {
      pname = "qint-frontend";
      src = ./frontend;
    
      postPatch = ''
        cp ${book_events} src/book_events.ts
        substituteInPlace svelte.config.js \
          --replace node_modules "$node_modules\", \"$node_modules/.."
      '';

      buildPhase = ''
        yarn build
      '';
    
      installPhase = ''
        mv deps/qint_frontend/dist $out
      '';
    
      distPhase = "\n";
    };

    packages.qint = naersk-lib.buildPackage (defaultLinuxBuildArgs // {
      # Only set for main derivation
      overrideMain = oldAttrs: {
        postPatch = ''
          substituteInPlace src-tauri/tauri.conf.json \
            --replace ../frontend/dist ${packages.frontend}
        '';

        FRONTEND_PATH = packages.frontend;
      };
    });

    # The rust compiler is internally a cross compiler, so a single
    # toolchain can be used to compile multiple targets. In a hermetic
    # build system like nix flakes, there's effectively one package for
    # every permutation of the supported hosts and targets.
    # i.e.: nix build .#packages.x86_64-linux.x86_64-pc-windows-gnu
    # where x86_64-linux is the host and x86_64-pc-windows-gnu is the
    # target
    packages.x86_64-pc-windows-gnu = naersk-lib-win.buildPackage (defaultBuildArgs // {
      nativeBuildInputs = defaultBuildArgs.nativeBuildInputs ++ (with pkgs; [
        pkgsCross.mingwW64.stdenv.cc
        # Used for running tests.
        wineWowPackages.stable
        # wineWowPackages is overkill, but it's built in CI for nixpkgs,
        # so it doesn't need to be built from source. It needs to provide
        # wine64 not just wine. An alternative would be this:
        # (wineMinimal.override { wineBuild = "wine64"; })
      ]);

      buildInputs = with pkgs.pkgsCross.mingwW64.windows; [ mingw_w64_headers mingw_w64_pthreads pthreads ];

      # Configures the target which will be built.
      # ref: https://doc.rust-lang.org/cargo/reference/config.html#buildtarget
      CARGO_BUILD_TARGET = "x86_64-pc-windows-gnu";

      # Configures the linker which will be used. cc.targetPrefix is
      # sometimes different than the targets used by rust. i.e.: the
      # mingw-w64 linker is "x86_64-w64-mingw32-gcc" whereas the rust
      # target is "x86_64-pc-windows-gnu".
      #
      # This is only necessary if rustc doesn't already know the correct linker to use.
      #
      # ref: https://doc.rust-lang.org/cargo/reference/config.html#targettriplelinker
      # CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = with pkgs.pkgsCross.mingwW64.stdenv;
      #   "${cc}/bin/${cc.targetPrefix}gcc";

      # Configures the script which should be used to run tests. Since
      # this is compiled for 64-bit Windows, use wine64 to run the tests.
      # ref: https://doc.rust-lang.org/cargo/reference/config.html#targettriplerunner
      CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUNNER = pkgs.writeScript "wine-wrapper" ''
        # Without this, wine will error out when attempting to create the
        # prefix in the build's homeless shelter.
        export WINEPREFIX="$(mktemp -d)"
        exec wine64 $@
      '';

      preBuild = ''
        export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_RUSTFLAGS="-C link-args=''$(echo $NIX_LDFLAGS | tr ' ' '\n' | grep -- '^-L' | tr '\n' ' ')"
        export NIX_LDFLAGS=
      '';

      #doCheck = true;

      # Multi-stage builds currently fail for mingwW64.
      singleStep = true;
    });
  });
}
