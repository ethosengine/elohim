{
  description = "Elohim hApp - Multi-DNA Holochain application for decentralized learning";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.6";
    nixpkgs.follows = "holonix/nixpkgs";
  };

  outputs = inputs @ { holonix, ... }:
    holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames holonix.devShells;
      perSystem = { inputs', pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [ inputs'.holonix.devShells.default ];
          packages = with pkgs; [
            # wasm-pack for building WASM modules
            wasm-pack
            # liblzma (required by wasm-pack)
            xz
          ];
          # Required for getrandom 0.3.x on wasm32-unknown-unknown
          # Holochain provides a custom random implementation via host functions
          RUSTFLAGS = "--cfg getrandom_backend=\"custom\"";
        };
      };
    };
}
