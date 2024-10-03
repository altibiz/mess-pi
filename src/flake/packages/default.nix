{ ... } @inputs:

{
  pyright-langserver = ((import ./pyright-langserver.nix) inputs);
  pyright = ((import ./pyright.nix) inputs);
  python3 = ((import ./python.nix) inputs);
  # FIXME: can't run dynamically linked executables
  # ruff = ((import ./ruff.nix) inputs);
  usql = ((import ./usql.nix) inputs);
  yapf = ((import ./yapf.nix) inputs);
}
