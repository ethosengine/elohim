---
id: "backlog-elohim-sdk-sync-full-features-broken"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-sdk `sync`/`full` cargo features broken since introduction (E0107 x3 + E0583) — also blocks `cargo fmt --check` on the whole crate"
slug: "elohim-sdk-sync-full-features-broken"
written: "2026-08-02"
author: "fix-integration (seam-concern architecture sprint, Wave A)"
status: "backlog"
priority: "medium"
tags: [elohim-sdk, crates, cargo-features, compile-break, fmt-check, dormant-defect, automerge, sync]
cites:
  - crates/elohim-sdk/Cargo.toml
  - crates/elohim-sdk/src/sync/mod.rs
  - crates/elohim-sdk/src/traits/syncable.rs
  - crates/elohim-sdk/src/error.rs
  - genesis/data/timeline/backlog/crates-seam-contracts-ci-pipeline-coverage-gap.md
---

# elohim-sdk `sync`/`full` features broken since introduction

## Symptom (verified 2026-08-02)

`cargo build --features sync` (and `--features full`, which pulls in `sync` via `native`+`sync`)
fails to compile `elohim-sdk` with two independent, unrelated error classes — 4 compiler errors
total:

```
error[E0583]: file not found for module `automerge_sync`
 --> src/sync/mod.rs:7:1
  | mod automerge_sync;

error[E0107]: enum takes 2 generic arguments but 1 generic argument was supplied
   --> src/traits/syncable.rs:40:31   (fn to_automerge -> Result<automerge::AutoCommit>)
   --> src/traits/syncable.rs:44:55   (fn from_automerge -> Result<Self>)
   --> src/traits/syncable.rs:48:44   (fn merge -> Result<()>)
```

Worse: **`cargo fmt --check` fails on the whole crate, unconditionally** — rustfmt resolves the
syntactic `mod` tree from `lib.rs` regardless of `#[cfg(feature = ...)]` gating, so the missing
`automerge_sync` module breaks formatting-check even for a plain `cargo fmt --check` with no
features requested at all:

```
Error writing files: failed to resolve mod `automerge_sync`:
  /projects/elohim/crates/elohim-sdk/src/sync/automerge_sync.rs does not exist
```

The default build (`cargo build`, feature = `client` only — the crate's `default = ["client"]`)
and default `cargo clippy` both proceed past this specific defect (clippy does hit unrelated
pre-existing lint debt — a derivable-impl warning and an `from_str`-shadow warning, both in
`src/reach/mod.rs` — out of scope for this entry).

## Root cause — two independent bugs, same feature gate

1. **E0583 — orphaned `mod` declaration.** `src/sync/mod.rs:7` declares
   `#[cfg(feature = "sync")] mod automerge_sync;` and re-exports `automerge_sync::*`, but no
   `src/sync/automerge_sync.rs` (or `src/sync/automerge_sync/mod.rs`) has **ever existed** —
   `git log --all --oneline -- '*/elohim-sdk/src/sync/automerge_sync.rs'` returns nothing, in
   either the current `crates/elohim-sdk/` path or the prior `holochain/crates/elohim-sdk/` path.
   The module was declared but its implementation file was never created.
2. **E0107 — missing local `Result` alias import.** `src/error.rs` defines a crate-local 1-arg
   alias: `pub type Result<T> = std::result::Result<T, SdkError>;`. `src/traits/syncable.rs`
   never imports it (no `use crate::error::Result;`), so its bare `Result<T>` usages
   (lines 40, 44, 48, all inside `#[cfg(feature = "sync")]` trait methods) resolve to
   `core::result::Result<T, E>` (2 generic params) instead — hence "enum takes 2 generic
   arguments but 1 generic argument was supplied" at all three sites.

## Introducing commit — precision correction against the initial framing

Verified via `git log --follow` + `git show --stat`/`-p`: `82a2e791f` ("refactor: move shared
Rust crates to root crates/", 2026-03-10) is a **100%-similarity pure rename** (`27 files
changed, 0 insertions(+), 0 deletions(-)`) that carried this code UNCHANGED from
`holochain/crates/elohim-sdk/` to its current `crates/elohim-sdk/` path — so it is accurate to
say that commit is when the defect entered its *current location*, but it did not introduce the
defect. `git show b78796c27 --stat` shows both `src/sync/mod.rs` (+14 lines) and
`src/traits/syncable.rs` (+89 lines) were **added new** by `b78796c27` ("feat(storage): Add
Diesel ORM with multi-tenant app scoping", 2026-01-08). The `automerge_sync.rs` file was never
created at any point in git history under either path — this has been dormant/broken since
2026-01-08 (~7 months as of this writing). Nothing in CI or the default dev loop builds
`elohim-sdk` with `--features sync`/`native`/`full`, so it went unnoticed.

## Blast radius

- No current production consumer builds `elohim-sdk` with `sync`/`native`/`full` — the default
  feature (`client`) compiles clean, and that is the only feature set any in-tree consumer
  (`elohim-storage`, `steward/node`, etc.) actually selects today.
- `crates/seam-contracts` (added 2026-08-02, same sprint as this entry) is a sibling in the
  `crates/` family with its own, independent `Cargo.lock`/workspace root — unaffected by this
  break. Its new pre-push gate (see the sibling entry
  `crates-seam-contracts-ci-pipeline-coverage-gap.md`) runs `cargo fmt --check` scoped to ITS
  OWN workspace only.
- Anyone running `cargo fmt --check` (or plain `cargo fmt`) directly against `crates/elohim-sdk`
  hits this immediately, feature flags notwithstanding.
- Any future work that turns on `sync`/`native`/`full` (Automerge-based P2P sync from the SDK
  side — the crate's own doc comment describes this as "Phase B: Holochain DHT ... for
  agent-centric data") will hit this on the first build.

## Fix sketch (NOT attempted in this pass — out of scope for the fix-integration task that
discovered this; do not touch elohim-sdk src as part of that task)

1. Either implement `src/sync/automerge_sync.rs` (the module the doc comments describe), or —
   if the intended surface is fully covered by the existing
   `elohim_storage_client::{AutomergeSync, SyncResult}` re-export a few lines below in the same
   file (`src/sync/mod.rs:12-13`) — remove the dead `mod automerge_sync;` + `pub use
   automerge_sync::*;` lines as abandoned scaffolding.
2. Add `use crate::error::Result;` (or fully-qualify `crate::error::Result<...>`) in
   `src/traits/syncable.rs`.
3. Re-verify: `cargo build --features full`, `cargo test --features full`,
   `cargo fmt --check` (unconditional pass), then `cargo clippy --all-features -- -D warnings`
   to confirm no further breakage was hiding behind the compile failure.

Status: open, unowned.
