{
  description = "Elohim Steward - Run your own Holochain node as a steward of co-creation";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.6";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    tauri-plugin-holochain.url = "github:darksoil-studio/tauri-plugin-holochain/main-0.6";
    tauri-plugin-holochain.inputs.holonix.follows = "holonix";
  };

  nixConfig = {
    extra-substituters = [
      "https://holochain-ci.cachix.org"
      "https://darksoil-studio.cachix.org"
    ];
    extra-trusted-public-keys = [
      "holochain-ci.cachix.org-1:5IUSkZc0aoRS53rfkvH9Kid40NpyjwCMCzwRTXy+QN8="
      "darksoil-studio.cachix.org-1:UEi+aujy44s41XL/pscLw37KEVpTEIn8N/kn7jO8rkc="
    ];
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames inputs.holonix.devShells;
      perSystem = { inputs', config, pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [
            inputs'.tauri-plugin-holochain.devShells.holochainTauriDev
            inputs'.holonix.devShells.default
          ];
          packages = [
            pkgs.nodejs_22
            pkgs.pnpm
            pkgs.imagemagick  # For making icon square
            pkgs.patchelf     # For fixing interpreter in built binaries
          ];
        };

        # Android development shell (future)
        devShells.androidDev = pkgs.mkShell {
          inputsFrom = [
            inputs'.tauri-plugin-holochain.devShells.holochainTauriAndroidDev
            inputs'.holonix.devShells.default
          ];
          packages = [
            pkgs.nodejs_22
            pkgs.pnpm
          ];
        };
      };
    };
}
