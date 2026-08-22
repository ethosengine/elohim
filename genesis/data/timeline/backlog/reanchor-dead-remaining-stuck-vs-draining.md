---
id: "backlog-reanchor-dead-remaining-stuck-vs-draining"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "dead_remaining now holds caughtUp=false (correct) — but a permanently skip-guarded dead anchor is indistinguishable from an actively-draining heal; surface stuck-vs-draining"
slug: "reanchor-dead-remaining-stuck-vs-draining"
written: "2026-08-22"
author: "code-review follow-up on anchor-liveness landing (0f2f227ed)"
status: "open"
priority: "medium"
tags: [dataplane, reanchor, anchor-liveness, p2p-status, observability, bounded-code-fix]
---

# `dead_remaining` wedges caughtUp forever when a dead-anchor row can never re-author

Since commit `0f2f227ed` (anchor LIVENESS), the `/p2p/status` pending count includes
`dead_remaining` — rows whose `dht_anchor_hash` was signed by a key no living chain can
present. That tightening is **correct**: a re-keyed peer must not read as caught up while
its strongest provenance claims are unpresentable.

But the reanchor sweep's re-authorability skip-guards
(`elohim/elohim-storage/src/services/reanchor_backfill.rs:298-328`) make one failure mode
invisible from the status surface: a dead-anchor row with a non-canonical `reach`
(`:304`) or non-canonical `content_type` (`:320`) is skipped *loudly in the logs* but
*silently in the count* — it can never be re-authored until a seed-data correction lands,
so it stays in `dead_remaining` every sweep, `caughtUp=false` forever. From
`/p2p/status` alone, that permanently-wedged state is indistinguishable from a healthy
heal actively draining a large backlog. The quiesce gate and any operator watching
`caughtUp` read both the same way.

## Proposed

Surface **stuck vs draining** as a distinct signal: track `dead_remaining` across
sweeps and, when it is unchanged for N consecutive sweeps (every candidate hit a
skip-guard or failed identically), report that on `/p2p/status` — e.g. a
`dead_remaining_stuck: true` / `stuck_sweeps: N` field or a per-outcome breakdown
(`RowOutcome::SkippedNonCanonicalReach` / `SkippedNonCanonicalContentType` counts are
already recorded in the sweep report, `:305`, `:321`). Draining keeps its honest
`caughtUp=false`; stuck becomes visibly a *seed-data-correction needed* state instead
of an eternal "still healing".

## Notes

- Bounded code fix: the sweep report already carries the per-outcome counts; the delta
  is persistence of the previous sweep's `dead_remaining` + a comparison window.
- Do NOT relax the pending arithmetic itself — the tightening is the point; this is an
  observability split, not a gate loosening.
