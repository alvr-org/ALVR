{
  inputs = {
    self.submodules = true;
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs =
    inputs@{ nixpkgs, ... }:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem =
        {
          pkgs,
          ...
        }:
        {
          packages.default = (pkgs.alvr.overrideAttrs (old: {
            src = ./.;
            cargoDeps = old.cargoDeps.overrideAttrs (old: {
              vendorStaging = old.vendorStaging.overrideAttrs {
                src = ./.;
                outputHash = "sha256-PpwHda/vsVqWtrVgeVv9phddY/vmo/dlAwhkJYRAQCI=";
              };
            });
          }));
        };
    };
}
