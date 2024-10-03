{ poetry2nix, pidgeonLib, ... }:

poetry2nix.mkPoetryApplication pidgeonLib.poetry.common
