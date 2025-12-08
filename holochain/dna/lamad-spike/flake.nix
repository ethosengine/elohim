{
  description = "Lamad Spike - Minimal Holochain DNA for browser connectivity testing";

  inputs = {
    nixpkgs.follows = "holochain/nixpkgs";
    versions.url = "github:holochain/holochain?dir=versions/0_4";
    holochain = {
      url = "github:holochain/holochain";
      inputs.versions.follows = "versions";
    };
  };

  outputs = inputs @ { ... }:
    inputs.holochain.inputs.flake-parts.lib.mkFlake
      { inherit inputs; }
      {
        systems = builtins.attrNames inputs.holochain.devShells;
        perSystem = { config, pkgs, system, ... }: {
          devShells.default = pkgs.mkShell {
            inputsFrom = [ inputs.holochain.devShells.${system}.holonix ];
            packages = with pkgs; [
              # Add any additional dev tools here
            ];
          };
        };
      };
}
