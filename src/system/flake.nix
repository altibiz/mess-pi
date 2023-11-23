# TODO: with nixos-generate
# TODO: CLI
# TODO: cert renewal

{
  description = "Raspberry Pi message broker";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    nixpkgs-stable.url = "github:NixOS/nixpkgs/release-23.05";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";
    sops-nix.inputs.nixpkgs-stable.follows = "nixpkgs-stable";
  };

  outputs = { self, nixpkgs, home-manager, sops-nix, ... } @ inputs: {
    nixosConfigurations.pidgeon = nixpkgs.lib.nixosSystem {
      system = "aarch64-linux";
      specialArgs = inputs;
      modules = [
        # nix
        ({ pkgs, modulesPath, ... }: {
          imports = [
            # NOTE: doesn't work without this for now
            # it should work with just `nixos-generate`
            "${modulesPath}/installer/sd-card/sd-image-aarch64.nix"
          ];

          nix.package = pkgs.nixFlakes;
          nix.extraOptions = "experimental-features = nix-command flakes";

          nixpkgs.config = { allowUnfree = true; };
          # TODO: https://github.com/NixOS/nixpkgs/issues/154163#issuecomment-1008362877  
          nixpkgs.overlays = [
            (final: super: {
              makeModulesClosure = x:
                super.makeModulesClosure (x // { allowMissing = true; });
            })
          ];

          system.stateVersion = "23.11";
        })

        # hardware
        ({ nixos-hardware, ... }: {
          imports = [
            nixos-hardware.nixosModules.raspberry-pi-4
          ];
        })

        # secrets
        ({ pkgs, ... }: {
          imports = [
            sops-nix.nixosModules.sops
          ];

          environment.systemPackages = with pkgs; [
            age
            ssh-to-age
            sops
          ];

          sops.defaultSopsFile = ./secrets/secrets.enc.yaml;
          environment.etc."sops-nix/key.txt" = {
            text = (builtins.readFile ./secrets/pidgeon-age.key);
            mode = "0600";
          };
          sops.age.keyFile = "/etc/sops-nix/key.txt";
        })

        # system
        ({ pkgs, ... }: {
          environment.systemPackages = with pkgs; [
            libraspberrypi
            raspberrypi-eeprom
            pkg-config
            openssl

            # admin
            man-pages
            man-pages-posix
            kitty
            nushell
            starship
            lazygit
            git
            helix

            # diag
            lm_sensors # NOTE: get sensor information
            dua # NOTE: get disk space usage interactively
            duf # NOTE: disk space usage overview
            du-dust # NOTE: disk space usage in a tree
            pciutils # NOTE: lspci
            lsof # NOTE: lsof -ni for ports
            dmidecode # NOTE: sudo dmidecode for mobo info
            inxi # NOTE: overall hardware info
            hwinfo # NOTE: overall hardware info
            htop # NOTE: CPU process manager
            tokei # NOTE: count lines of code
            dog # NOTE: dns client
            upower # NOTE: battery power
          ];

          environment.sessionVariables = {
            PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
          };

          location.provider = "geoclue2";
          time.timeZone = "Etc/UTC";
          i18n.defaultLocale = "en_US.UTF-8";

          services.openssh.enable = true;
          networking.hostName = "pidgeon";

          programs.direnv.enable = true;
          programs.direnv.nix-direnv.enable = true;

          users.users."pidgeon" = {
            isNormalUser = true;
            hashedPassword = (builtins.readFile ./secrets/password.pub);
            extraGroups = [ "wheel" ];
            shell = pkgs.bashInteractive;
            openssh.authorizedKeys.keys = [
              (builtins.readFile ./secrets/authorized.pub)
            ];
          };
        })

        # database
        ({ pkgs, config, ... }:
          let
            usql = pkgs.writeShellApplication {
              name = "usql";
              runtimeInputs = [ pkgs.usql ];
              text = ''
                usql pg://localhost/pidgeon?sslmode=disable
              '';
            };
          in
          {
            environment.systemPackages = [ usql ];

            services.postgresql.enable = true;
            services.postgresql.package = pkgs.postgresql_14;
            services.postgresql.extraPlugins = with config.services.postgresql.package.pkgs; [
              timescaledb
            ];
            services.postgresql.settings.shared_preload_libraries = "timescaledb";
            services.postgresql.ensureDatabases = [ "pidgeon" ];
            services.postgresql.ensureUsers = [
              {
                name = "pidgeon";
                ensureDBOwnership = true;
                ensureClauses = {
                  login = true;
                };
              }
            ];
            services.postgresql.authentication = pkgs.lib.mkOverride 10 ''
              # NOTE: do not remove local privileges because that breaks timescaledb
              # TYPE    DATABASE    USER        ADDRESS         METHOD        OPTIONS
              local     all         all                         trust
              host      all         all         samehost        trust
              hostssl   all         all         192.168.0.0/16  scram-sha-256
              hostssl   all         all         10.255.255.0/24 scram-sha-256
            '';
            services.postgresql.enableTCPIP = true;
            services.postgresql.port = 5432;
            networking.firewall.allowedTCPPorts = [ 5432 ];
            services.postgresql.settings.ssl = "on";
            services.postgresql.settings.ssl_cert_file = "/etc/postgresql/server.crt";
            sops.secrets."postgres.crt".path = "/etc/postgresql/server.crt";
            sops.secrets."postgres.crt".owner = "postgres";
            sops.secrets."postgres.crt".group = "postgres";
            sops.secrets."postgres.crt".mode = "0600";
            services.postgresql.settings.ssl_key_file = "/etc/postgresql/server.key";
            sops.secrets."postgres.key".path = "/etc/postgresql/server.key";
            sops.secrets."postgres.key".owner = "postgres";
            sops.secrets."postgres.key".group = "postgres";
            sops.secrets."postgres.key".mode = "0600";
            services.postgresql.initialScript = "/etc/postgresql/passwords.sql";
            sops.secrets."passwords.sql".path = "/etc/postgresql/passwords.sql";
            sops.secrets."passwords.sql".owner = "postgres";
            sops.secrets."passwords.sql".group = "postgres";
            sops.secrets."passwords.sql".mode = "0600";
          })

        # cli
        ({ pkgs, ... }: { })
      ];
    };
  };
}
