{
  description = "Elohim Steward - Run your own Holochain node as a steward of co-creation";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.6";

    nixpkgs.follows = "holonix/nixpkgs";
    flake-parts.follows = "holonix/flake-parts";

    tauri-plugin-holochain.url = "github:darksoil-studio/tauri-plugin-holochain/main-0.6";
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
          # Deduplicate NIX_CFLAGS_COMPILE to avoid "Argument list too long"
          # The combined devshells repeat -isystem paths hundreds of times
          shellHook = ''
            dedup_flags() {
              local seen=""
              local result=""
              local prev=""
              for arg in $1; do
                if [ "$prev" = "-isystem" ] || [ "$prev" = "-idirafter" ] || [ "$prev" = "-L" ]; then
                  local pair="$prev $arg"
                  if [[ ! " $seen " =~ " $pair " ]]; then
                    seen="$seen $pair"
                    result="$result $pair"
                  fi
                  prev=""
                elif [ "$arg" = "-isystem" ] || [ "$arg" = "-idirafter" ] || [ "$arg" = "-L" ]; then
                  prev="$arg"
                else
                  result="$result $arg"
                  prev=""
                fi
              done
              echo "$result"
            }
            export NIX_CFLAGS_COMPILE="$(dedup_flags "$NIX_CFLAGS_COMPILE")"
            export NIX_LDFLAGS="$(dedup_flags "$NIX_LDFLAGS")"
          '';
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
