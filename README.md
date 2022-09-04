# Pong

This is a simple Pong clone I wrote for some practice with the [Bevy](https://bevyengine.org/) game engine and [Nix](https://nixos.org). It serves as a simple example of how use Nix to build Rust packages for webassembly, letting Nix manage the toolchain and other dependencies.
Right now the AI plays perfectly and never misses, I haven't managed to score at least. Tweaking the difficulty is the next goal.

## How to run

### Web

Play the game [here](https://mith.github.io/pong)

### Nix

To run the game locally using Nix, use the following command:
```
nix run github:mith/pong
```

## How to contribute

Should it somehow be your deepest desire to contribute to a pong game, setting up a development environment is easy thanks to Nix and [Direnv](https://direnv.net/); run `direnv allow` in the source directory and any shells and editors with support for direnv will have the toolchain and tools like rust-analyzer linked into their environment when opening the project.
