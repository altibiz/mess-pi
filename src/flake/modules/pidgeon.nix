{ self, ... }:

{
  services.pidgeon.enable = true;

  services.pidgeon.configPath = "/etc/pidgeon/config.toml";
  environment.etc."pidgeon/config.toml" = {
    source = "${self}/assets/config.toml";
    user = "pidgeon";
    group = "pidgeon";
    mode = "0644";
  };

  services.pidgeon.envPath = "/etc/pidgeon/.env";
  sops.secrets."pidgeon.env" = {
    path = "/etc/pidgeon/.env";
    owner = "pidgeon";
    group = "pidgeon";
    mode = "0600";
  };
}
