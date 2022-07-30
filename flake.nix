{
  description = "pong";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
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

  outputs = { self, nixpkgs, flake-utils, fenix, naersk, ... }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = nixpkgs.legacyPackages."${system}";
        toolchain = fenix.packages.${system}.stable;
        naersk-lib = naersk.lib."${system}";
      in
      rec {
        packages.pong = naersk-lib.buildPackage {
          pname = "pong";
          root = ./.;
          buildInputs = with pkgs; [
            libxkbcommon
          ];
          nativeBuildInputs = with pkgs; [
            pkg-config
            alsaLib
            udev
            xorg.libX11
            xorg.libX11
            xorg.libXcursor
            xorg.libXi
            xorg.libXrandr
            libxkbcommon
            python3
            vulkan-loader
            wayland
            mold
          ];
        };
        packages.pong-web =
          let
            target = "wasm32-unknown-unknown";
            toolchain = with fenix.packages.${system};
              combine [
                minimal.rustc
                minimal.cargo
                targets.${target}.latest.rust-std
              ];
            pong-wasm = (naersk.lib.${system}.override {
              cargo = toolchain;
              rustc = toolchain;
            }).buildPackage
              {
                src = ./.;
                cargoBuildOptions = old: old ++ [ "--target wasm32-unknown-unknown"];
                nativeBuildInputs = with pkgs; [
                  pkg-config
                ];
                buildInputs = with pkgs; [
                ];
              };
          in
          pkgs.stdenv.mkDerivation {
            name = "pong-web";
            src = ./.;
            nativeBuildInputs = with pkgs; [
              wasm-bindgen-cli
            ];
            phases = [ "unpackPhase" "installPhase" ];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name wasm --target no-modules --no-typescript ${pong-wasm}/bin/pong.wasm
              cp index.html $out/index.html
              cp -r assets $out/assets
            '';
          };

        defaultPackage = packages.pong;

        apps.pong = flake-utils.lib.mkApp {
          drv = packages.pong;
        };
        defaultApp = apps.pong;

        devShell = pkgs.mkShell.override { stdenv = pkgs.clangStdenv; } {
          buildInputs = with pkgs; [ llvmPackages.libclang ];
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
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
          inputsFrom = [ packages.pong ];
          nativeBuildInputs = [
            pkgs.mangohud
            pkgs.cargo-edit
            (toolchain.withComponents [ "cargo" "rustc" "rust-src" "rustfmt" "clippy" ])
            pkgs.rust-analyzer
          ];
        };
      }
    );
}
