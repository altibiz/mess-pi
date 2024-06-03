{ self, pkgs, ... }:

let
  package = self.packages.${pkgs.system}.default;

  config = pkgs.writeTextFile {
    name = "config";
    destination = "/share/config.toml";
    text = builtins.readFile "${self}/src/flake/assets/config.toml";
  };

  run = pkgs.writeShellApplication {
    name = "pidgeon-docker";
    runtimeInputs = [ package ];
    text = ''
      #shellcheck disable=SC1091
      #shellcheck disable=SC2046
      ${package}/bin/pidgeon --config '${config}/share/config.toml'
    '';
  };
in
pkgs.dockerTools.buildImage {
  name = "altibiz/pidgeon";
  tag = "latest";
  created = "now";
  copyToRoot = pkgs.buildEnv {
    name = "image-root";
    paths = [ run config ];
    pathsToLink = [ "/bin" "/share" ];
  };
  config = {
    Cmd = [ "pidgeon-docker" ];
  };
}
