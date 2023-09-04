{ pkgs, username, ... }:

let
  poetryPylsp = pkgs.writeScriptBin "poetry-pylsp"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      export VIRTUAL_ENV="$("${pkgs.poetry}/bin/poetry" env info --path)"

      "${pkgs.python310Packages.python-lsp-server}/bin/pylsp" "$@"
    '';

  poetryPython = pkgs.writeScriptBin "poetry-python"
    ''
      #!${pkgs.stdenv.shell}
      set -eo pipefail

      "${pkgs.poetry}/bin/poetry" run python "$@"
    '';
in
{
  programs.home-manager.enable = true;
  xdg.configFile."nixpkgs/config.nix".source = ./assets/config.nix;

  xdg.configFile."pidgeon/config.yaml".source = ./assets/pidgeon.yaml;

  home.username = "${username}";
  home.homeDirectory = "/home/${username}";
  home.sessionVariables = {
    VISUAL = "hx";
    EDITOR = "hx";
    PAGER = "bat";
  };
  home.shellAliases = {
    lg = "lazygit";
    cat = "bat";
    grep = "rg";
    rm = "rm -i";
    mv = "mv -i";
    la = "exa";

    pls = "sudo";
    bruh = "git";
    sis = "hx";
    yas = "yes";
  };
  home.packages = with pkgs; [
    # dev
    meld
    nil
    nixpkgs-fmt
    python310
    (poetry.override { python3 = python310; })
    python310Packages.python-lsp-server
    ruff
    python310Packages.python-lsp-ruff
    python310Packages.pylsp-rope
    python310Packages.yapf
    poetryPylsp
    poetryPython
    llvmPackages.clangNoLibcxx
    llvmPackages.lldb
    rustc
    cargo
    clippy
    rustfmt
    rust-analyzer
    nodePackages.bash-language-server
    nodePackages.yaml-language-server
    taplo
    marksman

    # tui
    direnv
    nix-direnv
    pciutils
    lsof
    dmidecode
    inxi
    hwinfo
    ncdu
    file
    fd
    duf
    unzip
    unrar
    sd
    tshark
    sqlx-cli
  ];

  # dev
  programs.git.enable = true;
  programs.git.delta.enable = true;
  programs.git.attributes = [ "* text=auto eof=lf" ];
  programs.git.lfs.enable = true;
  programs.git.extraConfig = {
    interactive.singleKey = true;
    init.defaultBranch = "main";
    pull.rebase = true;
    push.default = "upstream";
    push.followTags = true;
    rerere.enabled = true;
    merge.tool = "meld";
    "mergetool \"meld\"".cmd = ''meld "$LOCAL" "$MERGED" "$REMOTE" --output "$MERGED"'';
    color.ui = "auto";
  };
  programs.helix.enable = true;
  programs.helix.languages = {
    language = [
      {
        name = "python";
        auto-format = true;
        formatter = { command = "${pkgs.yapf}/bin/yapf"; };
        language-server = { command = "${poetryPylsp}/bin/poetry-pylsp"; };
        config.pylsp.plugins = {
          rope = { enabled = true; };
          ruff = {
            enabled = true;
            executable = "${pkgs.ruff}/bin/ruff";
          };
          yapf = { enabled = false; };
          flake8 = { enabled = false; };
          pylint = { enabled = false; };
          pycodestyle = { enabled = false; };
          pyflakes = { enabled = false; };
          mccabe = { enabled = false; };
          autopep8 = { enabled = false; };
        };
      }
      {
        name = "nix";
        auto-format = true;
        formatter = { command = "${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt"; };
      }
    ];
  };
  programs.helix.settings = {
    theme = "transparent";
    editor = {
      true-color = true;
      scrolloff = 999;
      auto-save = true;
      rulers = [ ];
      gutters = [ "diagnostics" "spacer" "diff" ];
    };
  };
  programs.helix.themes.transparent = {
    inherits = "everforest_dark";

    "ui.background" = { };
    "ui.statusline" = { fg = "fg"; };
  };

  # tui
  programs.direnv.enable = true;
  programs.direnv.enableNushellIntegration = true;
  programs.direnv.nix-direnv.enable = true;
  programs.nushell.enable = true;
  programs.nushell.extraConfig = ''
    $env.config = {
      show_banner: false

      edit_mode: vi
      cursor_shape: {
        vi_insert: line
        vi_normal: underscore
      }

      hooks: {
        pre_prompt: [{ ||
          let direnv = (direnv export json | from json)
          let direnv = if ($direnv | length) == 1 { $direnv } else { {} }
          $direnv | load-env
        }]
      }
    }
  '';
  programs.nushell.environmentVariables = {
    PROMPT_INDICATOR_VI_INSERT = "' '";
    PROMPT_INDICATOR_VI_NORMAL = "' '";
  };
  programs.starship.enable = true;
  programs.starship.enableNushellIntegration = true;
  xdg.configFile."starship.toml".source = ./assets/starship.toml;
  programs.zoxide.enable = true;
  programs.zoxide.enableNushellIntegration = true;
  programs.lazygit.enable = true;
  programs.lazygit.settings = {
    notARepository = "quit";
    promptToReturnFromSubprocess = false;
    gui = {
      showIcons = true;
    };
  };
  programs.htop.enable = true;
  programs.lf.enable = true;
  programs.bat.enable = true;
  programs.bat.config = { style = "header,rule,snip,changes"; };
  programs.ripgrep.enable = true;
  programs.ripgrep.arguments = [
    "--max-columns=100"
    "--max-columns-preview"
    "--colors=auto"
    "--smart-case"
  ];
  programs.exa.enable = true;
  programs.exa.extraOptions = [
    "--all"
    "--list"
    "--color=always"
    "--group-directories-first"
    "--icons"
    "--group"
    "--header"
  ];

  # services
  programs.gpg.enable = true;
  services.gpg-agent.enable = true;
  services.gpg-agent.pinentryFlavor = "tty";
  programs.ssh.enable = true;

  home.stateVersion = "23.11";
}
