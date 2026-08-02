---
id: "backlog-elohim-sdk-native-mode-silent-write-loss"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-sdk ClientMode::Native without sync_url silently discards queued writes on flush() — C4/C14-class silent data loss"
slug: "elohim-sdk-native-mode-silent-write-loss"
written: "2026-08-02"
author: "readme-revision (seam-concern architecture sprint, Wave B/C)"
status: "backlog"
priority: "high"
tags: [elohim-sdk, crates, data-loss, silent-corruption, client-mode, native, concern-c4, concern-c14]
cites:
  - crates/elohim-sdk/src/
  - crates/elohim-sdk/README.md
  - genesis/data/timeline/backlog/elohim-sdk-sync-full-features-broken.md
---

# elohim-sdk `ClientMode::Native` without `sync_url` silently discards queued writes

## Symptom (verified 2026-08-02, found by the README blind-reader honesty loop)

`ClientMode::Native` has **no local-storage implementation at all** in the crate today:

- `Native { sync_url: Some(url) }` routes through the *identical* code path as `Node`
  (same `flush_to_storage` / `get_from_storage`) — it is `Node` wearing a different name.
- `Native { sync_url: None }`: `get()` returns `Err(SdkError::InvalidMode)` — honest —
  but `flush()` calls `take_batch()`, which **removes the ops from the write buffer**,
  and then the `Native { sync_url: None }` match arm **silently drops them**. Writes a
  caller queued in good faith are destroyed with no error, no counter, no log.
- The `native` cargo feature adds a `rusqlite` dependency that is **never referenced**
  anywhere in `src/` — an advertise/serve asymmetry (C7) at the feature-flag surface.

## Concern-class reading

- **C4 (honest absence)**: a mode that cannot store must refuse the write at `queue()`
  or error at `flush()` — destruction-as-success conflates "flushed" with "dropped".
- **C14 (witnessed residual)**: the drop happens on a path with no witness — no counter,
  no capsule, no ledger entry. Exactly the "fails 100% and says nothing" shape.
- **C7**: the `native` feature + API table advertised a capability the crate does not
  serve (the README now tells the truth; the code still carries the false affordance).

## Fix directions (needs an owner; not attempted in the sprint that found it)

1. Minimum honest cure: `flush()` on `Native { sync_url: None }` returns
   `Err(SdkError::InvalidMode)` **before** `take_batch()` — refuse, don't destroy.
2. Either implement the local-storage path the `rusqlite` dep implies, or remove the
   dep and the mode's storage claim (decide deliberately; see the sibling entry for
   the crate's broken `sync`/`full` features — same neglected-surface family).
3. Add a `ReasonLabel`-carrying counter on every discard/refusal path in the write
   buffer (C8), so the next silent path cannot exist unwitnessed.

The README (`crates/elohim-sdk/README.md`, revised 2026-08-02) now documents the true
behavior including the data-loss arm — documentation and code currently agree that the
code is wrong.

Status: open, unowned.
