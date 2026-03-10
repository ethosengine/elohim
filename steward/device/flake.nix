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
          # The combined devshells repeat -isystem paths hundreds of times.
          # Must also dedup _FOR_BUILD variants — proc-macro crates (serde_derive,
          # thiserror-impl, etc.) link as native code and use these flags.
          # Uses awk instead of bash [[ =~ ]] for reliable dedup in both
          # interactive shells and `nix develop --command`.
          shellHook = ''
            dedup_flags() {
              echo "$1" | tr ' ' '\n' | awk '
                /^-(isystem|idirafter|L)$/ { prefix=$0; next }
                prefix {
                  pair=prefix " " $0
                  if (!seen[pair]++) print pair
                  prefix=""
                  next
                }
                !seen[$0]++ { print }
              ' | tr '\n' ' '
            }
            export NIX_CFLAGS_COMPILE="$(dedup_flags "$NIX_CFLAGS_COMPILE")"
            export NIX_LDFLAGS="$(dedup_flags "$NIX_LDFLAGS")"
            export NIX_CFLAGS_COMPILE_FOR_BUILD="$(dedup_flags "$NIX_CFLAGS_COMPILE_FOR_BUILD")"
            export NIX_LDFLAGS_FOR_BUILD="$(dedup_flags "$NIX_LDFLAGS_FOR_BUILD")"
          '';
          packages = [
            pkgs.nodejs_22
            pkgs.pnpm
            pkgs.imagemagick  # For making icon square
            pkgs.patchelf     # For fixing interpreter in built binaries
            pkgs.just         # Task runner for root-level justfile
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
