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
      in rec {
        packages.pong-bin = naersk-lib.buildPackage {
          name = "pong-bin";
          root = ./.;
          buildInputs = with pkgs; [
            libxkbcommon
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
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
        };
        packages.pong = pkgs.stdenv.mkDerivation {
          name = "pong";
          src = ./assets;
          phases = ["unpackPhase" "installPhase"];
          installPhase = ''
            mkdir -p $out
            cp ${packages.pong-bin}/bin/pong $out/pong
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
        in
          (naersk.lib.${system}.override {
            cargo = toolchain;
            rustc = toolchain;
          })
          .buildPackage
          {
            src = ./.;
            CARGO_BUILD_TARGET = target;
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
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
              wasm-bindgen --out-dir $out --out-name pong --target web ${packages.pong-wasm}/bin/pong.wasm
              cp index.html $out/index.html
              cp -r assets $out/assets
            '';
          };

        packages.pong-server = pkgs.writeShellScriptBin "run-pong-server" ''
          ${pkgs.python3}/bin/python -m http.server --directory ${packages.pong-web}
        '';

        defaultPackage = packages.pong;

        apps.pong = flake-utils.lib.mkApp {
          drv = packages.pong;
        };
        defaultApp = apps.pong;

        devShell = pkgs.mkShell {
          shellHook = ''export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath (with pkgs; [
              alsaLib
              udev
              vulkan-loader
              libxkbcommon
              wayland
              freetype
              fontconfig
              libglvnd
              xorg.libXcursor
              xorg.libXext
              xorg.libXrandr
              xorg.libXi
            ])}"'';
          inputsFrom = [packages.pong-bin];
          nativeBuildInputs = [
            (toolchain.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
            pkgs.rust-analyzer
          ];
        };
      }
    );
}
