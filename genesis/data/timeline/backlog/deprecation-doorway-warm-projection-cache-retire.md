---
id: "backlog-deprecation-doorway-warm-projection-cache-retire"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Retire the deprecated doorway warm.rs HTTP-pull warmup (superseded by warm_stream SSE)"
slug: "deprecation-doorway-warm-projection-cache-retire"
written: "2026-06-11"
author: "deprecation-triage"
status: "backlog"
priority: "low"
deprecation_status: blocked
severity: low
fingerprints: ["9a0bc4bd6751"]
relatedNodeIds: []
tags: [deprecation, rust, doorway, projection-cache, warm-stream]
cites:
  - doorway/doorway-service/src/projection/warm.rs
  - doorway/doorway-service/src/projection/warm_stream.rs
  - doorway/doorway-service/src/main.rs
  - doorway/doorway-service/src/routes/admin_cache.rs
---

## What is deprecated

```
src/projection/warm.rs:23:#[deprecated(since = "0.1.0", note = "Use warm_stream::stream_from_peer instead")]
```

The entire `doorway/doorway-service/src/projection/warm.rs` module is an intentional
`#[deprecated]` staged-retirement marker (module docstring lines 1-8):

> **DEPRECATED**: Use `warm_stream` instead, which streams cacheable content via SSE
> from elohim-storage, filtered by reach, with reconnect support. This module fetches
> all content from each peer's storage URL via HTTP pull with arbitrary limits. It
> doesn't respect reach levels and pulls blindly. **Kept temporarily as fallback for
> older storage versions.**

Two deprecated public functions:
- `warm_projection_cache` (`warm.rs:24`) — `#[deprecated(... Use warm_stream::stream_from_peer instead)]`
- `spawn_warm_task` (`warm.rs:141`) — `#[deprecated(... Use warm_stream::spawn_stream_task instead)]`

The module carries `#![allow(deprecated)]` (line 10) so its own internals/test compile
clean. The marker is deliberate architectural signalling placed during the warm_stream
cutover, not a surprise regression — the same pattern as the `LocalSourceChainService`
and `RelationshipInferenceSource` intentional markers already canonicalized as blocked.

## Usage inventory

The deprecated functions have **zero production callers**. The live cold-start warmup
path is `warm_stream::*` exclusively:

- `main.rs:574` — `projection::warm_stream::WarmupState::new()`
- `main.rs:875` — `projection::warm_stream::spawn_stream_task(...)` (the live boot warmup)
- `admin_cache.rs:162` — `projection::warm_stream::stream_from_peer(...)` (admin re-warm)

The only reference to the deprecated `warm_projection_cache` is at `warm.rs:155`, inside
the module's own `#[cfg(test)]` block testing the deprecated fn itself. No production code,
no other module, no manifest references `warm_projection_cache` or `spawn_warm_task`.

Blast radius if retired: one module file (`warm.rs`, 170 lines) plus its own test block.
No call-site edits required elsewhere.

## Migration path

The successor `warm_stream` already fully owns the warmup responsibility (SSE streaming
from elohim-storage, reach-filtered, reconnect-aware). There is nothing left to migrate —
the cutover happened; `warm.rs` is dead reference code retained per its docstring "as
fallback for older storage versions."

Retirement = delete `warm.rs` (and remove its `mod warm;` declaration + any re-export),
then `RUSTFLAGS="" cargo build --release && cargo test --lib --bins && cargo clippy -- -D warnings`.

## Current decision

**Blocked** (on a deliberate retirement decision, not on a technical dependency). The
module docstring explicitly retains warm.rs "as fallback for older storage versions" —
deleting it is an architectural call about whether any storage peer old enough to need
the blind HTTP-pull warmup (no SSE `/db/content` stream support) can still exist in the
fleet. That is a rust-architect / doorway-operator decision about the supported-storage-
version floor, not a background-agent mechanical fix:

- If the SSE stream endpoint is now guaranteed on every storage peer doorway can route to,
  the fallback is genuinely dead and the module can be deleted (bounded: one file + its
  test, no call-site churn — a clean follow-up).
- If older-storage compatibility is still a live concern, the marker stays and the module
  remains as documented fallback.

The deprecation is intentional and `#![allow(deprecated)]` already suppresses the build
warning; the `cargo`/eslint surface emits nothing in normal builds. The sentinel
suppresses further dispatch on this fingerprint (ledger status: blocked). The
deprecation-stasis sweep re-checks this when the supported-storage-version floor is
confirmed.

## Verification

N/A — not yet retired. Will be verified when the supported-storage floor is confirmed and
the module is deleted: `RUSTFLAGS="" cargo build --release && cargo test --lib --bins &&
cargo clippy -- -D warnings && cargo fmt --check` green in doorway-service, with no
remaining references to `warm_projection_cache` / `spawn_warm_task`.
