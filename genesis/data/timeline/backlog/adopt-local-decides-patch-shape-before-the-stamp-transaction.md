---
id: "backlog-adopt-local-decides-patch-shape-before-the-stamp-transaction"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "adopt_local decides its patch shape (metadata-only vs pointer-bearing) from a pre-transaction read — a concurrent PATCH between the read and the transaction can still produce a torn row"
slug: "adopt-local-decides-patch-shape-before-the-stamp-transaction"
written: "2026-09-21"
author: "serving-edge failover-balance-stream campaign, 2026-09-21 review"
status: "open"
priority: "medium"
jobs: [elohim]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, head-adoption, race-condition, torn-row, adopt-before-author]
---

**The fact.** In `try_adopt_canonical_head`'s caller path, `local_declared` is read once from the pool
(`elohim/elohim-storage/src/services/head_adoption.rs:1398`) before `adopt_local` (`:1577`) is invoked. Inside
`adopt_local` (`:1797-1887`), that same pre-read `local_declared` value decides the SHAPE of the patch that will
be applied: whether this is a move-or-fill that should carry the head's `blob_cid`/`content_size_bytes`
(`is_move_or_fill` at `:1834`, gated `if head.canonical && is_move_or_fill` at `:1835`), and whether the row
already declares this exact head and so should instead run the T7 pointer-heal comparison (`:1851-1886`). Only
after that shape is fixed does the function call `stamp_declared_head_mode` (`:1889`), which opens a transaction
(`db/content_diesel.rs:1850`, `conn.transaction(|conn| { stamp_declared_head_mode_transaction(...) })`) that
re-reads the row's *existing* state fresh (`:1878`, the `existing` tuple) to decide whether the move itself is
permitted. The transaction re-validates whether to MOVE, but it does not re-derive what the *patch* should
contain — that was fixed outside the lock, from a `local_declared` value that a concurrent write (a direct PATCH,
or another adoption path) could have already invalidated by the time the transaction opens.

**Evidence.** T-1 (`77cf2ba48`, story 1.4a) narrows this: `adopt_local` now carries the head's own `blob_cid` on
every move-or-fill instead of leaving metadata-only patches that let a row's declared head move while its blob
pointer stayed behind (the mechanism `torn-row-never-selected-for-repair.md` and
`chunked-blob-over-16mb-not-durable-mesh-repro.md`'s "manifest-shape" residue both describe downstream of). But
T-1's own commit message is explicit about scope: it heals no row that is already torn, and the fix is in WHICH
fields the patch carries once computed — not in WHEN the patch's shape is decided relative to the write it
protects. The codex review of the reverted 1.4b attempt independently names this residual: "the new carried
patch avoids `adopt_local`'s conditional pointer-selection race. The fallback still reaches that race at
head_adoption.rs:1834" (`codex-review-1.4b.result.md`, finding 9).

**Why it matters.** The transaction's forward-ordering/move check is sound against a concurrent writer — but a
sound move decision paired with a patch shape computed from stale data can still stamp a new head with the WRONG
pointer, silently recreating exactly the torn-row class T-1 was meant to close, just through a narrower window
(a write landing between the `:1398` read and the `:1850` transaction start, rather than between two independent
calls).

**Smallest next step.** Either move the `is_move_or_fill`/pointer-heal decision inside
`stamp_declared_head_mode_transaction` where `existing` is freshly read, or have `adopt_local` build a complete,
independently-verified replacement patch (not conditioned on the pre-read `local_declared`) so a stale read can
only under-patch, never mis-patch. Needs an interleaving test: concurrent PATCH between the `:1398` read and the
`:1850` transaction start, asserting the stamped row's pointer matches its declared head.

**Links.** T-1: commit `77cf2ba48` (story 1.4a). Review:
`genesis/a2o/reports/recovery/serving-edge-20260920/codex-review-1.4b.result.md` (finding 9). Design:
`genesis/a2o/reports/recovery/serving-edge-20260919/story-1.4a-design.md`. Distinct mechanism, same neighborhood:
`adopt-local-heal-second-guesses-arbitrated-winner.md` (a slower-convergence gap in `HealCanonical`'s
forward-ordering re-proof, not a race). Habit: `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`.
