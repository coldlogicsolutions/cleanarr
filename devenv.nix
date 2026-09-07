{ pkgs, inputs, ... }:

let
  openspec = inputs.openspec.packages.${pkgs.stdenv.system}.default;
in
{
  # Rust toolchain (stable channel via rust-overlay) with the tooling cargo needs.
  languages.rust = {
    enable = true;
    channel = "stable";
    components = [ "rustc" "cargo" "clippy" "rustfmt" "rust-analyzer" "rust-src" ];
  };

  packages = [
    pkgs.pkg-config
    pkgs.openssl
    # OpenSpec CLI for spec-driven development (openspec/).
    openspec
  ];

  # Pre-commit hooks, installed into .git/hooks when the shell starts.
  git-hooks.hooks = {
    # rustfmt/clippy default to the toolchain configured in languages.rust.
    rustfmt.enable = true;
    clippy = {
      enable = true;
      settings = {
        denyWarnings = true;
        offline = false;
        extraArgs = "--workspace --all-targets";
      };
      extraPackages = [ pkgs.pkg-config pkgs.openssl ];
    };
    openspec-validate = {
      enable = true;
      name = "openspec validate";
      entry = "${openspec}/bin/openspec validate --specs";
      files = "^openspec/";
      pass_filenames = false;
    };
  };
}
