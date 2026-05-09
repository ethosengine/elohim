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
            # Native build chain for sweettest (datachannel-sys → libdatachannel
            # built from source). Required by the DNA Integration stage; holonix
            # default devShell does not include these.
            cmake
            pkg-config
            clang
            libclang.lib
            openssl
            zlib
            libsodium
          ];
          # bindgen needs to find the clang resource directory for stdbool.h etc.
          LIBCLANG_PATH = "${pkgs.libclang.lib}/lib";
          # Required for getrandom 0.3.x on wasm32-unknown-unknown
          # Holochain provides a custom random implementation via host functions
          RUSTFLAGS = "--cfg getrandom_backend=\"custom\"";
          # Pick up sccache from /usr/local/bin when present (Jenkins CI image
          # ci-builder-nix and devspace rust-nix-dev both ship sccache 0.15.0
          # there, outside the Nix store on purpose so it survives `nix develop`
          # PATH stripping). The conditional keeps this safe on dev machines
          # without sccache — they get the un-wrapped cargo path. SCCACHE_*
          # env vars (SCCACHE_BUCKET, SCCACHE_ENDPOINT, AWS_*) are auto-mounted
          # in the consuming pod's namespace via the sccache-credentials Secret;
          # we don't need to declare them here.
          shellHook = ''
            if [ -x /usr/local/bin/sccache ]; then
              export PATH=/usr/local/bin:$PATH
              export RUSTC_WRAPPER=sccache
              export SCCACHE_LOG=''${SCCACHE_LOG:-warn}
            fi
          '';
        };
      };
    };
}
