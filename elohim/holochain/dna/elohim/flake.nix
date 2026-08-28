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
            # Diagnostic — surface sccache state on EVERY nix develop entry so
            # build logs answer "is sccache visible here?" without us having
            # to spelunk. Pipe to stderr so it doesn't pollute stdout-consuming
            # commands.
            {
              echo "[flake-shellHook] sccache probe:"
              if [ -x /usr/local/bin/sccache ]; then
                echo "  /usr/local/bin/sccache: present ($(/usr/local/bin/sccache --version 2>&1 | head -1))"
                export PATH=/usr/local/bin:$PATH
                export SCCACHE_LOG=''${SCCACHE_LOG:-warn}
                # Arm the wrapper only if the cache SERVER can actually start.
                # A dead S3/Garage credential makes `sccache --start-server`
                # fail with AccessDenied; if RUSTC_WRAPPER is set anyway, the
                # very first `cargo metadata` dies (`sccache rustc -vV` exit 2)
                # and the whole DNA build is red in 85 s before any compile
                # (elohim-holochain #1403, 2026-08-27). A dead cache must cost a
                # cold compile, never a red build — so probe, and fall through
                # to un-wrapped cargo with a loud line when the probe fails.
                # `--start-server` forks the daemon and returns 0 BEFORE the daemon
                # touches storage (elohim-holochain #1404 armed the wrapper on that
                # false OK and died one line later), so the probe must be the same
                # call cargo makes first: `sccache rustc -vV`. That reaches the
                # storage read and fails exactly when the build would.
                /usr/local/bin/sccache --stop-server >/dev/null 2>&1 || true
                if timeout 90 /usr/local/bin/sccache rustc -vV >/dev/null 2>&1; then
                  export RUSTC_WRAPPER=sccache
                  echo "  RUSTC_WRAPPER=$RUSTC_WRAPPER (sccache rustc -vV probe OK)"
                else
                  unset RUSTC_WRAPPER
                  /usr/local/bin/sccache --stop-server >/dev/null 2>&1 || true
                  echo "  RUSTC_WRAPPER NOT SET — sccache cannot reach its cache storage (dead credential?); cold compile instead of a red build"
                  echo "  probe stderr: $(timeout 60 /usr/local/bin/sccache rustc -vV 2>&1 | grep -iE 'error|denied' | head -2 | tr '\n' ' ')"
                fi
                echo "  SCCACHE_BUCKET=''${SCCACHE_BUCKET:-<unset>}"
                echo "  SCCACHE_ENDPOINT=''${SCCACHE_ENDPOINT:-<unset>}"
                echo "  PATH=$PATH"
              else
                echo "  /usr/local/bin/sccache: ABSENT — RUSTC_WRAPPER not set, cargo will not wrap"
                echo "  ls /usr/local/bin/sccache: $(ls -la /usr/local/bin/sccache 2>&1)"
                # Defensive: if RUSTC_WRAPPER is set in the inherited env but the
                # binary is missing, unset it to avoid the os-error-2 trap that
                # blew up #1216 / #1218.
                if [ -n "''${RUSTC_WRAPPER:-}" ]; then
                  echo "  RUSTC_WRAPPER was inherited as=$RUSTC_WRAPPER — UNSETTING to avoid exec failure"
                  unset RUSTC_WRAPPER
                fi
              fi
            } >&2
          '';
        };
      };
    };
}
