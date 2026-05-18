# elohim-views

Wire-shape Rust types for the Elohim Protocol storage API.

This crate holds the ts-rs-anchored View + InputView types that define the HTTP wire contract between elohim-storage and its clients. It is intentionally lightweight — only serde, serde_json, chrono, serde_bytes, and ts-rs — so consumers (Tauri desktop, third-party Rust SDKs) can depend on the type surface without pulling the full storage implementation.

## Generated TypeScript

Running `cargo test export_bindings` in this crate produces TypeScript types at `elohim/sdk/storage-client-ts/src/generated/`, consumed by `@elohim/storage-client`.

## Boundary

Per `deny.toml` at the repo root, only the server-side wrappers (doorway-service, steward-node, elohim-node, elohim-storage-client) may depend on `elohim-storage` directly. All other consumers depend on `elohim-views` (or `elohim-sdk` which re-exports it).
