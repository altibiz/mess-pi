{ pkgs, ... }:

pkgs.writeShellApplication {
  name = "yapf";
  runtimeInputs = [ pkgs.poetry ];
  text = ''
    # shellcheck disable=SC1091
    source "$(poetry env info --path)/bin/activate"
    yapf "$@"
  '';
}

