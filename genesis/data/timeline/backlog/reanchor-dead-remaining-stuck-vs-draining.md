---
id: "backlog-reanchor-dead-remaining-stuck-vs-draining"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "dead_remaining now holds caughtUp=false (correct) — but a permanently skip-guarded dead anchor is indistinguishable from an actively-draining heal; surface stuck-vs-draining"
slug: "reanchor-dead-remaining-stuck-vs-draining"
written: "2026-08-22"
author: "code-review follow-up on anchor-liveness landing (0f2f227ed)"
status: "fixed-pending-runtime-proof"
priority: "medium"
tags: [dataplane, reanchor, anchor-liveness, p2p-status, observability, bounded-code-fix]
---

## Fix (2026-08-22, branch fix/doorway-breaker-trial-theft-and-apps-extraction-herd)

Landed as an observability split. **`caughtUp` and the pending arithmetic are
untouched** — `pending` still sums both re-anchor arms, and a standing
`dead_remaining` still holds `caughtUp=false`. What changed is that the surface
now says *which kind* of not-caught-up it is.

**Cross-sweep memory** (`services/provide_loop_status.rs`): `ProvideLoopState`
gained a private `ProvideLoopInner { status, prev_dead_remaining }` — the
previous sweep's `dead_remaining` plus a consecutive-unchanged run counter,
under the SAME lock as the status they derive (a reader can never catch a
verdict that disagrees with the count it was computed from). Ephemeral
(Category C): in-memory only, no DHT touch, no migration. On restart the run
counter resets to 0 and the node **re-earns** the verdict over the next 3
sweeps — honest reconstruction, since a genuinely wedged row re-trips it and a
seed-corrected row never does.

**Threshold** `DEAD_REMAINING_STUCK_SWEEPS = 3`. Rationale (in the const's
doc): the two arms share one sweep budget and the never-authored arm takes
first claim on it, so a large NULL population can legitimately starve the dead
arm for a sweep or two — that is draining, not stuck. Three consecutive
identical observations (two no-progress transitions) clears that window while
still surfacing a real wedge within a few sweep intervals.

**Two pure functions, both unit-tested with no sweep / conductor / clock:**

- `next_unchanged_sweeps(previous, previous_run, current) -> u32` — 0 on a
  drained population; extends on unchanged; restarts at 1 on ANY movement
  (progress *or* growth — the loop is alive either way).
- `is_dead_remaining_stuck(dead_remaining, unchanged_sweeps) -> bool` —
  `dead_remaining > 0 && unchanged_sweeps >= DEAD_REMAINING_STUCK_SWEEPS`.

**New `/p2p/status` `provideLoop` wire fields** (additive, camelCase, all
optional in the schema so an older node's payload still validates):

| Field | Meaning |
|---|---|
| `deadRemainingStuck: boolean` | the wedge verdict — *a seed-data correction is needed*, not *still healing* |
| `stuckSweeps: number` | consecutive sweeps at the same non-zero `deadRemaining`; watch it climb 1 → 2 → 3 |
| `reanchorDeadRemaining: number` | the dead arm of `reanchorPending` split out on its own |
| `reanchorSkippedReach: number` | last sweep's `SkippedNonCanonicalReach` count — names the wedge |
| `reanchorSkippedContentType: number` | last sweep's `SkippedNonCanonicalContentType` count |

The two skip counts were already recorded as `RowOutcome`s but folded into one
`skipped` total; `ReanchorReport` now also carries `skipped_reach` /
`skipped_content_type` (partitioning `skipped`, never double-counting it) and
they ride to the status surface as last-sweep values. They are what turns
"stuck" into an actionable line: *stuck at 2, both rows skipped for
non-canonical reach → fix the seed data*.

`publish_reanchor_sweep` now takes a `ReanchorSweepResult` struct carrying both
remaining arms separately; `pending()` on it computes the same
`remaining + dead_remaining` sum the call site used to pass positionally.

Tests: `unchanged_run_advances_only_while_the_value_holds`,
`stuck_verdict_needs_a_nonzero_population_held_past_the_threshold`,
`pending_is_still_the_sum_of_both_arms`,
`a_wedged_dead_population_reads_stuck_after_n_sweeps`,
`a_draining_dead_population_never_reads_stuck`,
`healing_clears_a_standing_stuck_verdict`,
`a_never_authored_backlog_alone_is_never_stuck`,
`skip_counters_split_by_cause_and_still_sum_to_skipped`,
`a_fully_skip_guarded_sweep_publishes_the_stuck_shape` (sweep-level
report → publish → verdict, no conductor), and schema contract
`p2p_status_view_with_stuck_dead_anchor_population_matches_schema`.

Verification: `cargo test --lib` 2932 passed / 0 failed; `cargo test --test
schema_contract` 228 passed / 0 failed; `cargo fmt --check` and
`cargo clippy --lib --tests -D warnings` clean. `pnpm -w run schema:codegen:ts`
regenerated only the six `p2p-status-view.ts` distributions (+20 lines each,
purely additive); `cargo test export_bindings` regenerated
`ProvideLoopStatus.ts`.

**Honest status**: code + tests landed on the branch. The live mesh still runs
the old binary, so no node has yet published a `deadRemainingStuck`. Runtime
proof (the "done when" below) waits for the next binary roll.

## Done when

A node with a permanently skip-guarded dead-anchor row reports
`deadRemainingStuck: true` with a non-zero `reanchorSkippedReach` or
`reanchorSkippedContentType` on `/p2p/status`, while `caughtUp` stays `false` —
and a node genuinely draining a dead backlog never reports it.

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
