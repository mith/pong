{
  description = "pong";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixpkgs-local.url = "github:mith/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    naersk,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        toolchain = fenix.packages.${system}.stable;
        naersk-lib = naersk.lib."${system}";
        buildInputs = with pkgs; [
          libxkbcommon
          alsaLib
          udev
          xorg.libX11
          xorg.libXcursor
          xorg.libXi
          xorg.libXrandr
          libxkbcommon
          python3
          vulkan-loader
          wayland
        ];
        nativeBuildInputs = with pkgs; [
          mold
          clang
          pkg-config
        ];
      in {
        packages.pong-bin = naersk-lib.buildPackage {
          name = "pong-bin";
          root = ./.;
          buildInputs = buildInputs;
          nativeBuildInputs = nativeBuildInputs;
        };
        packages.pong = pkgs.stdenv.mkDerivation {
          name = "pong";
          src = ./assets;
          phases = ["unpackPhase" "installPhase"];
          installPhase = ''
            mkdir -p $out
            cp ${self.packages.${system}.pong-bin}/bin/pong $out/pong
            cp -r $src $out/assets
          '';
        };

        packages.pong-wasm = let
          target = "wasm32-unknown-unknown";
          toolchain = with fenix.packages.${system};
            combine [
              minimal.rustc
              minimal.cargo
              targets.${target}.latest.rust-std
            ];
          naerskWasm = naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          };
        in
          naerskWasm.buildPackage {
            src = ./.;
            CARGO_BUILD_TARGET = target;
            nativeBuildInputs = nativeBuildInputs;
          };

        packages.pong-web = let
          local = import inputs.nixpkgs-local {system = "${system}";};
        in
          pkgs.stdenv.mkDerivation {
            name = "pong-web";
            src = ./.;
            nativeBuildInputs = [
              local.wasm-bindgen-cli
            ];
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name pong --target web ${self.packages.${system}.pong-wasm}/bin/pong.wasm
              cp index.html $out/index.html
              cp -r assets $out/assets
            '';
          };

        packages.pong-server = pkgs.writeShellScriptBin "run-pong-server" ''
          ${pkgs.python3}/bin/python -m http.server --directory ${self.packages.${system}.pong-web}
        '';

        defaultPackage = self.packages.pong;

        apps.pong = flake-utils.lib.mkApp {
          drv = self.packages.${system}.pong;
          exePath = "/pong";
        };
        defaultApp = self.apps.${system}.pong;

        devShell = pkgs.mkShell {
          shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"'';
          inputsFrom = [self.packages.${system}.pong-bin];
          nativeBuildInputs =
            [
              (toolchain.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
              pkgs.rust-analyzer
              pkgs.lldb
            ]
            ++ nativeBuildInputs;
        };
      }
    );
}
