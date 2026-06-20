---
id: "backlog-deprecation-content-server-serve-url-endpoints-migration"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire deprecated serve_url field on ContentServer/RegisterContentServerInput (superseded by endpoints)"
slug: "deprecation-content-server-serve-url-endpoints-migration"
written: "2026-06-20"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["0bb129dbb203"]
relatedNodeIds: []
tags: [deprecation, rust, elohim-storage, doorway, content-server, wire-api, backwards-compat, endpoints]
cites:
  - elohim/elohim-storage/src/content_server.rs
  - doorway/doorway-service/src/services/storage_registration.rs
  - doorway/doorway-service/src/projection/subscriber.rs
  - crates/doorway-client/src/publish.rs
  - elohim/sdk/storage-client-ts/src/wire-types/infrastructure/ContentServer.ts
  - elohim/sdk/storage-client-ts/src/wire-types/infrastructure/RegisterContentServerInput.ts
---

## What is deprecated

```
content_server.rs:83:    /// URL where this server accepts content requests (DEPRECATED - use endpoints)
content_server.rs:84:    pub serve_url: Option<String>,
```

The `serve_url: Option<String>` field on `RegisterContentServerInput` (and its twin on
`ContentServerConfig`, `ContentServerOutput`, `ContentServerView`) is an intentional
backwards-compat shim, documented inline as `DEPRECATED - use endpoints`. The successor
is `endpoints: Option<Vec<StorageEndpointInput>>` (multiple reachable endpoints with
latency-based routing, bandwidth metadata, and region annotation). The field appears in
these positions across the codebase:

- `content_server.rs:19` — doc-comment example shows both side-by-side
- `content_server.rs:45,57` — `ContentServerConfig` struct + default (None)
- `content_server.rs:84,117` — `RegisterContentServerInput`, `ContentServerOutput` fields
- `content_server.rs:230-249` — shim logic: if endpoints is None, build one from `serve_url`
  (backwards-compat fallback path)
- `content_server.rs:427,444,462` — `ContentServerView` read projection + test fixture

## Usage inventory

37 references across four crates + TypeScript SDKs — this is a **live wire protocol field**,
not dead dead code:

- `elohim/elohim-storage/src/content_server.rs` — 15 hits (struct definition, shim logic, view projection)
- `elohim/elohim-storage/src/blob_store.rs:296` — doc reference to fallback behavior
- `doorway/doorway-service/src/services/storage_registration.rs:93,177` — struct + wire send (`serve_url: Some(config.storage_url.clone())` — still actively populating the field for old-storage-peer compat)
- `doorway/doorway-service/src/projection/subscriber.rs:95,857,865,867` — struct + fallback consumer (if endpoints empty, add `serve_url` to route set)
- `crates/doorway-client/src/publish.rs:104,158,170,470` — client-side builder + wire consumption
- TypeScript SDKs:
  - `elohim/sdk/storage-client-ts/src/wire-types/infrastructure/ContentServer.ts:7`
  - `elohim/sdk/storage-client-ts/src/wire-types/infrastructure/RegisterContentServerInput.ts:7`
  - `elohim/sdk/domains/infrastructure/types/bindings/ContentServer.ts`
  - `elohim/sdk/domains/infrastructure/types/bindings/RegisterContentServerInput.ts`

Notably `doorway-service/src/services/storage_registration.rs:177` STILL SENDS the field
(`serve_url: Some(config.storage_url.clone())`) with an explicit `// Deprecated but included
for compat` comment — meaning live alpha deployments currently receive the field.

## Migration path

The successor `endpoints` field is already wired. Full removal requires:

1. **Confirm floor**: All storage peers on alpha must be running a version that sends
   `endpoints` — once the `subscriber.rs` fallback path (`serve_url` as route-set input)
   can be dead-code-verified, the shim can be dropped.
2. **Remove in three steps**: (a) Stop populating `serve_url` in `storage_registration.rs`
   (b) Remove `serve_url` from all struct definitions and wire shapes (c) Remove the fallback
   consumer in `subscriber.rs:857-867` (d) Regen TypeScript bindings (`cargo test export_bindings`).
3. **Scale**: ~37 touch-points across 4 Rust files + 4 TS generated files. Not a giant
   migration, but it crosses a wire boundary and requires coordinated alpha fleet confirmation
   before the shim is safe to drop.

## Current decision

**Blocked** — the field is a **live wire compatibility shim**, not dead code. `doorway-service`
actively populates it for old-peer compat (line 177), and `subscriber.rs` consumes it as a
fallback routing source (lines 857-867). Dropping it before every alpha peer sends `endpoints`
would break routing for any storage peer that doesn't yet populate the field.

This is a rust-architect / doorway-operator decision about the supported-storage-version floor
(mirrors the same gate as `deprecation-doorway-warm-projection-cache-retire.md`). When the
supported floor is confirmed:

- Verify zero `serve_url` inbound reads needed (no peer sending only `serve_url`)
- Remove the shim in a single bounded Rust + TS codegen commit
- Run: `RUSTFLAGS="" cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings`
  in doorway-service, then `cargo test export_bindings` in elohim-storage, then `pnpm run schema:codegen:ts`

The sentinel suppresses further dispatch on fingerprint `0bb129dbb203`. The deprecation-stasis
sweep re-checks when the storage-version floor is confirmed.

## Verification

N/A — not yet retired. Will be verified when the shim is removed and the above gates pass clean
with zero `serve_url` references remaining in non-test Rust source and zero `serve_url` fields
in the TS generated types.
