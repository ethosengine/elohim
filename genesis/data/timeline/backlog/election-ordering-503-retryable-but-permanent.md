---
id: "backlog-election-ordering-503-retryable-but-permanent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The election-ordering 503 reads as retryable backpressure but is a permanent coordinator-version fact — and the prologue's own log line about it is false"
slug: "election-ordering-503-retryable-but-permanent"
written: "2026-09-21"
author: "serving-edge household-acceptance campaign, 2026-09-20/21 review"
status: "open"
priority: "medium"
jobs: [elohim, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, election-ordering, http-503, retry-semantics, coordinator-hotswap]
---

**The fact.** `elohim/elohim-storage/src/http.rs:9065-9073` answers `503 "Conductor returned no
complete canonical election ordering; refusing an unordered eager stamp"` on the declare route
whenever `ContentHeadWire.canonical_ordering()` (`services/conductor_writes.rs:566-568`, `self.canonical_declared_at.zip(self.canonical_earned)`)
is `None`. On this route that is provably a coordinator-version fact, not a transient state: both
fields are set unconditionally on every successful `declare_canonical_head_inner` return, post-`8ebce05a3`
(DNA `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:5843-5844`) — a `None` here means the
installed `content_store` coordinator predates that commit, and retrying can never change the answer.
The 2026-09-20 prologue retried each leg for ~3 minutes (4 attempts, then 6×30 s) and logged "the
conductor was never asked" when the conductor had in fact been asked, had authored the canonical-head
link, and returned a receipt with only the two ordering fields thin.

**Evidence.** `http.rs:9065-9073` (guard) and `:1155-1161` (sibling delegated-PATCH arm).
`conductor_writes.rs:496-568` (`#[serde(default)]` on both fields; `canonical_ordering()`). DNA
`lib.rs:5843-5844` (unconditional set, landed in `8ebce05a3`, 2026-09-14 05:23:37Z). False message:
`J4-mesh-prologue.log:723-725` — doorway A: "⚠ canonical-head declare BACKPRESSURE (HTTP 503) — the
conductor was never asked" immediately after "authored action…verified…blobHash". Retry cost:
`J4-mesh-prologue.log:730-735` (doorway B, 6 attempts × 30 s). Occurrence count: `grep -c 'unordered
eager stamp'` = 21 per run across `F5-`, `G1-`, `J4-mesh-prologue.log` (0 on the pre-reset
`26-mesh-prologue.log`). Full mechanism and byte-level bundle proof: `FINDING-election-ordering-503.md` §1-2, §7.

**Why it matters.** Every caller of this route (the prologue's soft legs, any future retry logic)
currently treats a 503 as backpressure and burns real wall-clock — ~3 minutes per leg on 2026-09-20 —
on a condition that cannot heal by waiting. The log line actively misdirects diagnosis by claiming the
conductor was never asked, when a `content_store` coordinator hot-swap (`ALLOW_COORDINATOR_UPDATE`, no
reinstall, no re-key, no DHT churn) is the one action that resolves it.

**Smallest next step.** Return a non-retryable status (or at minimum a distinguishing error field) with
a cause naming the stale coordinator, at both `http.rs:9069` and `:1155` — the sibling
`head_adoption.rs:3606` line already logs the honest sentence ("conductor declared but returned no
complete canonical election ordering; holding for a later heal"); the HTTP arm should say the same
instead of "was never asked". Separately, decide whether `#[serde(default)]` on
`canonical_declared_at`/`canonical_earned` (`conductor_writes.rs:496-558`) — the mechanism that lets a
pre-cure coordinator decode silently instead of failing to deserialize — is still wanted, now that it
is the sole reason this is undiagnosable from the wire alone.

**Links.** Evidence: `genesis/a2o/reports/recovery/serving-edge-20260920/FINDING-election-ordering-503.md`.
Habit: `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`. Sibling:
`genesis/data/timeline/backlog/mesh-start-does-not-read-household-environment.md` (why the pre-cure
coordinator got installed in the first place — same incident, different fix).
