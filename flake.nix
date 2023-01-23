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
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    pre-commit-hooks.url = "github:cachix/pre-commit-hooks.nix";
    terranix.url = "github:terranix/terranix";
    matchbox = {
      url = "github:johanhelsing/matchbox";
      flake = false;
    };
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
          cmake
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

        packages.pong-web = let
          local = import inputs.nixpkgs-local {system = "${system}";};
        in
          pkgs.stdenv.mkDerivation {
            name = "pong-web";
            src = ./.;
            nativeBuildInputs = [
              local.wasm-bindgen-cli
              pkgs.binaryen
            ];
            phases = ["unpackPhase" "installPhase"];
            installPhase = ''
              mkdir -p $out
              wasm-bindgen --out-dir $out --out-name pong --target web ${self.packages.${system}.pong-wasm}/bin/pong.wasm
              mv $out/pong_bg.wasm .
              wasm-opt -Oz -o $out/pong_bg.wasm pong_bg.wasm
              cp web/* $out/
              cp -r assets $out/assets
            '';
          };

        packages.pong-server = pkgs.writeShellScriptBin "run-pong-server" ''
          ${pkgs.simple-http-server}/bin/simple-http-server -i -c=html,wasm,ttf,js -- ${self.packages.${system}.pong-web}/
        '';

        packages.signalling-server = crane-lib.buildPackage {
          name = "matchbox-server";
          src = inputs.matchbox;
          cargoBuildCommand = "cargo build --release -p matchbox_server";
          nativeBuildInputs = with pkgs; [
            pkg-config
            cmake
            fontconfig
          ];
        };

        packages.signalling-server-image = pkgs.dockerTools.buildImage {
          name = "pong-signalling-server";
          tag = "latest";
          copyToRoot = pkgs.buildEnv {
            name = "pong-signalling-server";
            paths = [
              self.packages.${system}.signalling-server
              pkgs.busybox
            ];
          };
          config = {
            Cmd = ["sh" "-c" "${self.packages.${system}.signalling-server}/bin/matchbox_server 0.0.0.0:8080"];
          };
        };

        packages.infra = inputs.terranix.lib.terranixConfiguration {
          inherit system;
          modules = [
            {
              terraform.required_providers.heroku.source = "heroku/heroku";
              provider.heroku = {
                email = "simonvoordouw@gmail.com";
              };

              resource.heroku_app.pong = {
                name = "pong-signalling-server";
                region = "eu";
              };
            }
          ];
        };

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
                enable = false;
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
          nativeBuildInputs = with pkgs;
            [
              (toolchain.withComponents ["cargo" "rustc" "rust-src" "rustfmt" "clippy"])
              flyctl
              (pkgs.docker.override {clientOnly = true;})
            ]
            ++ nativeBuildInputs;
        };
      }
    );
}
