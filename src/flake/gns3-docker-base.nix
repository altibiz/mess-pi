{ pkgs, ... }:

pkgs.dockerTools.buildImage {
  name = "altibiz/gns3-base";
  tag = "latest";
  created = "now";
  copyToRoot = with pkgs.dockerTools; [
    usrBinEnv
    binSh
    caCertificates
    fakeNss
  ];
  runAsRoot = ''
    mkdir -p /var/run
  '';
}
