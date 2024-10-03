{ self, pkgs, poetry2nix, ... }:

let
  common = {
    projectDir = "${self}/src/probe";
    preferWheels = true;
    checkGroups = [ ];
    overrides = poetry2nix.defaultPoetryOverrides.extend (final: prev: {
      numpy = prev.numpy.overridePythonAttrs (old: {
        buildInputs = (old.buildInputs or [ ]) ++ (with pkgs; [
          libgcc
        ]);
      });

      pyright = prev.pyright.overridePythonAttrs (old: {
        postInstall = (old.postInstall or "") + ''
          wrapProgram $out/bin/pyright \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
          wrapProgram $out/bin/pyright-langserver \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.nodejs ]}
        '';
      });
    });
  };

  mkEnvWrapper = env: name: pkgs.writeShellApplication {
    name = name;
    runtimeInputs = [ env ];
    text = ''
      export PYTHONPREFIX=${env}
      export PYTHONEXECUTABLE=${env}/bin/python

      # shellcheck disable=SC2125
      export PYTHONPATH=${env}/lib/**/site-packages

      ${name} "$@"
    '';
  };
in
{
  inherit common mkEnvWrapper;
}
