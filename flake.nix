{
  description = "Pidgeon - Raspberry Pi message broker.";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/release-24.05";
    nixpkgs-stable.url = "github:NixOS/nixpkgs/release-24.05";

    flake-utils.url = "github:numtide/flake-utils";

    nixos-hardware.url = "github:NixOS/nixos-hardware/master";

    sops-nix.url = "github:Mic92/sops-nix";
    sops-nix.inputs.nixpkgs.follows = "nixpkgs";
    sops-nix.inputs.nixpkgs-stable.follows = "nixpkgs-stable";

    poetry2nix.url = "github:nix-community/poetry2nix";
  };

  outputs = { self, nixpkgs, flake-utils, ... } @ rawInputs:
    let
      overlay = (import ./src/flake/overlay.nix) rawInputs;

      nixosModule = import ./src/flake/service.nix;

      systems = flake-utils.lib.defaultSystems;

      ids = builtins.map
        (x: x.name)
        (builtins.filter
          (x: x.value == "regular")
          (
            let
              dir = builtins.readDir ./src/flake/enc;
            in
            builtins.map
              (name: {
                inherit name;
                value = dir.${name};
              })
              (builtins.attrNames dir)
          ));
    in
    builtins.foldl'
      (outputs: system:
        let
          pkgs = import nixpkgs {
            inherit system;
            config = { allowUnfree = true; };
            overlays = [ overlay ];
          };

          poetry2nix = rawInputs.poetry2nix.lib.mkPoetry2Nix { inherit pkgs; };

          inputs =
            let
              libInputs = rawInputs // {
                inherit pkgs;
                inherit poetry2nix;
              };
            in
            libInputs // {
              pidgeonLib = (import ./src/flake/pidgeonLib/default.nix) libInputs;
            };

          devShellInputs = inputs // {
            pkgs = inputs.pkgs //
              ((import ./src/flake/packages/default.nix) inputs);
          };

          configInputs = inputs // {
            self = inputs.self // {
              nixosModules = inputs.self.nixosModules //
                (import ./src/flake/modules/default.nix);
            };
          };

          cli = (import ./src/flake/cli.nix) inputs;
          probe = (import ./src/flake/probe.nix) inputs;
        in
        outputs // {
          packages = (outputs.packages or { }) // {
            ${system} = {
              default = cli;
              default-docker = (import ./src/flake/cli-docker.nix) inputs;
              probe = probe;
              probe-docker = (import ./src/flake/probe-docker.nix) inputs;
            };
          };

          apps = (outputs.packages or { }) // {
            ${system} = {
              default = { type = "app"; program = "${cli}/bin/pidgeon"; };
              probe = { type = "app"; program = "${probe}/bin/pidgeon-probe"; };
            };
          };

          devShells = (outputs.devShells or { }) // {
            ${system} = builtins.mapAttrs
              (name: value: value devShellInputs)
              (import ./src/flake/shells/default.nix);
          };

          nixosConfigurations = builtins.foldl'
            (configs: id: configs // {
              "pidgeon-${id}-${system}" =
                (import ./src/flake/configuration.nix)
                  (configInputs // { inherit id; });
            })
            (outputs.nixosConfigurations or { })
            ids;
        })
      {
        overlays.default = overlay;

        nixosModules.default = nixosModule;
      }
      systems;
}
