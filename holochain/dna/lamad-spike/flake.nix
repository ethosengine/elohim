{
  description = "Lamad Spike - Minimal Holochain DNA for browser connectivity testing";

  inputs = {
    holonix.url = "github:holochain/holonix?ref=main-0.5";
    nixpkgs.follows = "holonix/nixpkgs";
  };

  outputs = inputs @ { holonix, ... }:
    holonix.inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = builtins.attrNames holonix.devShells;
      perSystem = { inputs', pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          inputsFrom = [ inputs'.holonix.devShells.default ];
          packages = with pkgs; [
            # Add any additional dev tools here
          ];
        };
      };
    };
}
