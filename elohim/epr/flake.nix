{
  description = "elohim-epr — native Rust CI toolchain (canonical CBOR + CIDv1 + Ed25519 codec)";

  # Why this flake exists: the elohim/ cargo workspace has no flake of its own,
  # and the cluster's CI images provide Rust ONLY via `nix develop` (ci-builder is
  # node-only; ci-builder-nix bundles nix + a cargo-nextest binary but no rustup).
  # This is the smallest devShell that gives the elohim-epr CI pipeline a native
  # (non-WASM) Rust toolchain, mirroring the pattern used by steward/node and
  # steward/device. It intentionally provides ONLY a toolchain — no package/
  # docker outputs — so `nix develop path:elohim/epr` copies a tiny tree.
  #
  # NOTE: flake.lock is generated on first `nix develop` (no nix in the dev
  # container to pre-lock). To pin inputs reproducibly, run `nix flake lock`
  # in a nix-capable env and commit the resulting flake.lock.

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # Pinned, never `stable.latest`: the version is declared ONCE in
        # ./rust-toolchain.toml (channel + `default` profile + rust-src/clippy/rustfmt)
        # and equals the local gate's RUSTUP_TOOLCHAIN (genesis/agentic/pool-policy.json;
        # genesis/orchestrator/rust-toolchain-pin.test.mjs refuses a mismatch). With no
        # flake.lock, `stable.latest` resolved to whatever rust-overlay HEAD carried on
        # each run (1.99.0 on elohim-eprfs #43), so CI ran a newer clippy than the gate.
        # rust-overlay keeps every past stable manifest, so an unlocked overlay still
        # yields exactly the declared version.
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in {
        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            # The eprfs publisher uses tomllib in this shared CI devShell.
            python311
          ];

          # Native crate — the Holochain WASM getrandom flag must NOT leak in
          # (root gotcha "RUSTFLAGS Override Required"). Belt to the pipeline's
          # own `export RUSTFLAGS=""`.
          RUSTFLAGS = "";
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

          shellHook = ''
            echo "elohim-epr CI toolchain: $(cargo --version)"
          '';
        };
      });
}
