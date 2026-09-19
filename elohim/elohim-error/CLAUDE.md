# elohim-error — the storage error vocabulary

One enum, `StorageError`, and the four `From` conversions that feed it. Every layer above —
`db/`, the blob family, services, both transport stacks, the HTTP boundary — returns it, which is
what makes it the bottom of the crate stack. It is carved out of `elohim-storage/src/error.rs`
byte-identically; that path stays alive as a re-export shim, so `crate::error::StorageError` and
`crate::StorageError` still resolve upstream.

Design: `genesis/docs/superpowers/specs/2026-09-19-storage-crate-decomposition-design.md`
(§3 principles, §4 row 1 — this crate, §5 proof).

## What it refuses to know

There is no service here, no connection, no route, no runtime, no transport. The only foreign
types this crate names are the four whose failures it converts: `diesel::result::Error`,
`std::io::Error`, `serde_json::Error`, `sled::Error`.

**The boundary is the dependency graph, not a lint.** A direct `use tokio;` fails to compile; a
*transitive* pull is invisible until link time, so `tests/boundary.rs` asserts it over this
crate's own `Cargo.lock`: no `libp2p`, no `iroh`, no `holochain*`, no `tokio`, no `reqwest`, no
`hyper`/`axum`, and zero first-party dependencies. Do not widen that denylist — a heavy subtree
here is paid for by every crate above.

## Adding a variant

A new `StorageError` variant is additive and cheap. A new **conversion** is the decision:

- `From<ForeignError>` lands **HERE**, with its crate added to `[dependencies]` — Rust's orphan
  rule means an upper crate cannot implement `From<Foreign> for StorageError` at all. If the
  foreign crate is one the denylist names, that is the answer, not an obstacle.
- Otherwise reduce it at the call site to an existing `String` variant
  (`Protocol`, `Codec`, `Internal`, …). A transport or conductor error becomes a string here; it
  does not drag its crate down the stack.

Version requirements for the four dependencies mirror what `elohim-storage` declares, and the
lockfiles pin the same patch releases. A skew would make `From<diesel::result::Error>` a
different type and break every `?` upstream — match the requirement, do not float it.

## Gate

```bash
CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/<branch>/elohim-error/dev \
  just --justfile elohim/elohim-error/justfile gate
```

`gate` = `cargo fmt --check` · `cargo clippy --all-targets -- -D warnings` · `cargo test`
(which runs `tests/boundary.rs`). `RUSTFLAGS` is baked EMPTY in the justfile: this is a native
build with no wasm-facing dependency, so the ambient `--cfg getrandom_backend="custom"` the
Holochain path exports would only break linking. `just gate elohim-error` from the repo root
resolves the pool slot from `elohim/holochain/build-manifest.json`.

A change here rebuilds the storage image, so this crate is COPY'd in
`elohim/elohim-storage/Dockerfile` and watched by that manifest's `cargo-build-storage` sources.
`genesis/orchestrator/storage-build-inputs.test.mjs` holds both.
