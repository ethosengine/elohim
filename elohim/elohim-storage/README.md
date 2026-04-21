# elohim-storage

Rust storage sidecar for Elohim Protocol nodes. Runs alongside the Holochain
conductor and serves HTTP on `:8090` (Tauri mode) or behind Doorway's proxy.

## EPR REST API (Phase 2a)

Seven routes under `/api/v1/epr` implement the Elohim Protocol Record REST
surface. Ingest is idempotent (`PUT /api/v1/epr/:cid`). All GET responses carry
`X-Epr-Source: local` in Phase 2a; Phase 2c extends to `peer:<PeerId>` when
the libp2p bridge is wired.

See **[docs/EPR_REST_API.md](docs/EPR_REST_API.md)** for the full API reference,
including request/response shapes, idempotency semantics, verification stages,
provider discovery, and the Phase 2c libp2p federation roadmap.

## Build

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

## Test

```bash
# Unit + schema contract tests (fast, no DB required)
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test

# Integration tests (require test database — Jenkins provisions automatically)
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test -- --ignored

# Regenerate TypeScript types (after changing views.rs)
cargo test export_bindings
```
