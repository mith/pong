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
          filter = path: type: nixpkgs.lib.all
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
          craneWasm = (crane.mkLib pkgs).overrideToolchain toolchain;
        in
          craneWasm.buildPackage {
            src = pong-src;
            CARGO_BUILD_TARGET = target;
            CARGO_PROFILE = "release";
            nativeBuildInputs = nativeBuildInputs;
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
