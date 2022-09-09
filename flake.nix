{
  description = "pong";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
  };

  outputs = inputs @ {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    crane,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = nixpkgs.legacyPackages."${system}";
        toolchain = fenix.packages.${system}.stable;
        crane-lib = crane.lib."${system}";
        pong-src = builtins.path {
          path = ./.;
          name = "pong-src";
          filter = path: type:
            nixpkgs.lib.all
            (n: builtins.baseNameOf path != n)
            [
              "web"
              "assets"
              "flake.nix"
              "flake.lock"
              "README.md"
              ".envrc"
              ".direnv"
              ".gitignore"
            ];
        };
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
        packages.pong-bin = crane-lib.buildPackage {
          name = "pong-bin";
          src = pong-src;
          inherit buildInputs;
          inherit nativeBuildInputs;
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
              stable.rustc
              stable.cargo
              targets.${target}.stable.rust-std
            ];
          craneWasm = (crane.mkLib pkgs).overrideToolchain toolchain;
        in
          craneWasm.buildPackage {
            src = pong-src;
            CARGO_BUILD_TARGET = target;
            CARGO_PROFILE = "release";
            inherit nativeBuildInputs;
            doCheck = false;
          };

        packages.pong-web =
          pkgs.stdenv.mkDerivation {
            name = "pong-web";
            src = ./.;
            nativeBuildInputs = [
              pkgs.wasm-bindgen-cli
            ];
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name pong --target web ${self.packages.${system}.pong-wasm}/bin/pong.wasm
              cp web/* $out/
              cp -r assets $out/assets
            '';
          };

        packages.pong-server = pkgs.writeShellScriptBin "run-pong-server" ''
          ${pkgs.python3}/bin/python -m http.server --directory ${self.packages.${system}.pong-web}
        '';

        defaultPackage = self.packages.${system}.pong;

        apps.pong = flake-utils.lib.mkApp {
          drv = self.packages.${system}.pong;
          exePath = "/pong";
        };
        defaultApp = self.apps.${system}.pong;

        checks = {
          pre-commit-check = inputs.pre-commit-hooks.lib.${system}.run {
            src = ./.;
            hooks = {
              alejandra.enable = true;
              statix.enable = true;
              rustfmt.enable = true;
              clippy = {
                enable = true;
                entry = let
                  rust = toolchain.withComponents ["clippy"];
                in
                  pkgs.lib.mkForce "${rust}/bin/cargo-clippy clippy";
              };
            };
          };
        };

        devShell = pkgs.mkShell {
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${pkgs.lib.makeLibraryPath buildInputs}"
            ${self.checks.${system}.pre-commit-check.shellHook}
          '';
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
