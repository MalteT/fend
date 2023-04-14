{
  inputs.dream2nix.url = "github:nix-community/dream2nix";
  inputs.devshell.url = "github:numtide/devshell";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.treefmt-nix.url = "github:numtide/treefmt-nix";
  inputs.pre-commit-hooks-nix.url = "github:cachix/pre-commit-hooks.nix";
  inputs.nixpkgs.url = "nixpkgs";

  outputs = inputs @ {
    flake-parts,
    dream2nix,
    devshell,
    treefmt-nix,
    pre-commit-hooks-nix,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      imports = [
        dream2nix.flakeModuleBeta
        devshell.flakeModule
        treefmt-nix.flakeModule
        pre-commit-hooks-nix.flakeModule
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem = {
        self',
        pkgs,
        config,
        ...
      }: {
        dream2nix.inputs = {
          fend = {
            source = ./.;
            projects = fromTOML (builtins.readFile ./projects.toml);
          };
        };
        packages.fend = config.dream2nix.outputs.fend.packages.fend;
        packages.default = self'.packages.fend;
        devShells = {
          default = config.dream2nix.outputs.fend.devShells.default.overrideAttrs (old: {
            buildInputs =
              old.buildInputs
              ++ [
                # Additional packages for the shell
                config.treefmt.package
                pkgs.nil
                pkgs.cargo-workspaces
                pkgs.rust-analyzer
              ];
          });
        };
        treefmt.projectRootFile = "flake.nix";
        treefmt.programs = {
          rustfmt.enable = true;
          alejandra.enable = true;
        };
      };
    };
}
