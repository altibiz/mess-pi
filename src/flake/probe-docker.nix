{ self, pkgs, ... } @inputs:

let
  package = self.packages.${pkgs.system}.probe;

  base = (import ./gns3-docker-base.nix) inputs;

  config = pkgs.writeTextFile {
    name = "config";
    destination = "/share/config.toml";
    text = builtins.readFile "${self}/assets/config.toml";
  };

  run = pkgs.writeShellApplication {
    name = "pidgeon-probe-docker";
    runtimeInputs = [ package ];
    text = ''
      export PIDGEON_PROBE_ENV=production
      ${package}/bin/pidgeon-probe "$@" --config '${config}/share/config.toml'
    '';
  };
in
pkgs.dockerTools.buildImage {
  name = "altibiz/pidgeon-probe";
  tag = "latest";
  created = "now";
  fromImage = base;
  copyToRoot = pkgs.buildEnv {
    name = "image-root";
    paths = [ run config ];
    pathsToLink = [ "/bin" "/share" ];
  };
  config = {
    Cmd = [ "pidgeon-probe-docker" ];
  };
}
