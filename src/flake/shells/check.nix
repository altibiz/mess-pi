{ pkgs, poetry2nix, pidgeonLib, ... }:

let
  env = poetry2nix.mkPoetryEnv pidgeonLib.poetry.common;
in
pkgs.mkShell {
  RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

  packages = with pkgs; [
    # Python - first because DVC python gets first in path
    poetry
    (pidgeonLib.poetry.mkEnvWrapper env "pyright")
    (pidgeonLib.poetry.mkEnvWrapper env "pyright-langserver")
    env

    # Nix
    nixpkgs-fmt

    # Rust
    rustc
    cargo
    clippy
    rustfmt

    # Shell
    shfmt
    shellcheck

    # Spelling
    nodePackages.cspell

    # Misc
    nodePackages.prettier

    # Tools
    nushell
    just
    pkg-config
    openssl
    zip
    unzip
  ];
}
