{
  description = "elohim-node - Elohim Protocol infrastructure runtime";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

      in {
        # Development shell
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            protobuf
            sqlite

            # Development tools
            cargo-watch
            cargo-edit
            cargo-audit

            # Container tools
            docker
            docker-compose
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "elohim-node development environment"
            echo "  cargo build    - Build the project"
            echo "  cargo test     - Run tests"
            echo "  cargo watch    - Watch for changes"
            echo ""
            echo "Simulation:"
            echo "  cd simulation && docker-compose up -d"
          '';
        };

        # Package
        packages.default = pkgs.callPackage ./nix/package.nix { };
        packages.elohim-node = self.packages.${system}.default;

        # Docker image
        packages.dockerImage = pkgs.dockerTools.buildLayeredImage {
          name = "elohim-node";
          tag = "latest";

          contents = with pkgs; [
            self.packages.${system}.default
            cacert
            tzdata
          ];

          config = {
            Cmd = [ "/bin/elohim-node" ];
            ExposedPorts = {
              "4001/tcp" = { };
              "4001/udp" = { };
              "8080/tcp" = { };
              "9090/tcp" = { };
            };
            Volumes = {
              "/var/lib/elohim" = { };
            };
          };
        };
      }
    ) // {
      # NixOS module
      nixosModules.default = import ./nix/module.nix;
      nixosModules.elohim-node = self.nixosModules.default;

      # Example NixOS configuration
      nixosConfigurations.example = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          self.nixosModules.default
          ({ pkgs, ... }: {
            services.elohim-node = {
              enable = true;
              clusterName = "example-family";
              clusterKey = "example-secret-key";
              openFirewall = true;
            };
          })
        ];
      };
    };
}
