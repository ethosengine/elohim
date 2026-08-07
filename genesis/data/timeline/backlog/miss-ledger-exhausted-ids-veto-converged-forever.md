---
id: "backlog-miss-ledger-exhausted-ids-veto-converged-forever"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "MissLedger-exhausted ids veto converged forever — GapCounts.converged in reconcile_rails.rs counts adjudicated-dormant ids as blocking, structurally pinning elohim_projection_reconcile_converged at 0 since 2026-08-01"
slug: "miss-ledger-exhausted-ids-veto-converged-forever"
written: "2026-08-07"
author: "claude (dataplane convergence gate, operator-directed)"
status: "documented"
priority: "high"
tags: [projection-reconcile, converged, gap-counts, miss-ledger, exhausted, gauge-honesty, ch06, quiesce-gate, elohim-storage]
cites:
  - elohim/elohim-storage/src/p2p/reconcile_rails.rs
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - content-gap-limit-cycle-blocks-convergence
  - rea-heal-classify-write-toctou-transactionalize
---

# MissLedger-exhausted ids veto converged forever (RESOLVED not-a-defect, 2026-08-07)

**Status:** documented — premise falsified same-day; closed not-a-defect, see
Resolution below. Superseded as the leading suspect for the `converged` plateau by
`content-gap-limit-cycle-blocks-convergence` (the real, still-open pin).
**Owner surface:** `elohim/elohim-storage/src/p2p/reconcile_rails.rs` (`GapCounts::converged`,
`GapTracker::counts`, line ~83) — the SHARED generic gap state machine both the replication
stream and the acquisition stream dispatch through.

## Finding

`GapCounts.converged` is computed as `self.pending.is_empty() && exhausted == 0`
(`reconcile_rails.rs:83`). `exhausted` counts every id whose retry budget is spent
(`exhausted_count()`, ~line 65) — with no distinction between "still worth retrying
later" and "adjudicated dormant, will never resolve on this arm." That second class —
foreign-app ids never gossiped by any connected peer, and ids adjudicated dormant by
the MissLedger — sits in the low hundreds fleet-wide (content 193-398, REA 0-12 live)
but is nonzero essentially always, so `converged` has been structurally unreachable
since this gauge's gate was added 2026-08-01.

Consequence: every CI Dataplane Validation run since 2026-08-01 that gates on
`converged` **did not measure** anything — the recorded saga state has been frozen at
build #1267, and ch06 (heads-converge) cannot reach its finish line no matter how well
the heal leg performs, because a handful of permanently-dormant ids veto the gauge
forever regardless of drain progress.

**Evidence:** edge #1318 held 45 minutes of `A` reading `converged=false` (raw gauge
0.0) while `caughtUp=True` — the sweep-complete signal was true, the stronger
convergence signal could not move. Loki heal-complete lines 16:38-17:27Z show REA
`refused=12/12` every sweep (the full divergent set adjudicated-refused, correctly,
every cycle) with the gauge still pinned at 0.

## Decision (2026-08-07)

Extend the `b19f12014` actionable-vs-total honesty pattern (which already separated
*divergence blocking convergence* from *divergence a canonical channel must resolve*,
per commit `fix(storage): convergence excludes adjudicated divergence`) to the
exhausted bucket: an exhausted id becomes **adjudicated-unresolvable**, visible in its
own metric (`elohim_projection_reconcile_exhausted`), but non-blocking for
`converged`. This mirrors the REA/content discovery arms in
`projection_reconcile.rs`, which already carve retry-exhausted ids out of their local
trackers via `exhausted_persistent` (see `ReaDiscovery`/`ContentDiscovery`,
~line 974-992) — `reconcile_rails.rs`'s generic `GapTracker` (shared by the
replication + acquisition streams) is the one arm that has not yet received this
cure.

## Fix direction

`GapCounts::converged` should read `pending.is_empty()` alone (matching `caught_up`'s
existing semantics) once exhausted ids are reclassified as adjudicated-dormant and
reported solely via the `exhausted` gauge — never folded into `converged`'s veto term.
Verify against the existing `reconcile_rails.rs` unit tests
(`retry_exhausted_gaps_are_caught_up_but_not_converged` at ~line 242 currently asserts
the OLD, structurally-pinned behavior and will need to flip alongside the fix).

## Resolution (2026-08-07) — NOT A DEFECT, premise falsified

The morning's finding conflated two structures that share the word "exhausted." Same-day
red-team review plus a live probe falsified the premise before the "fix direction" above
was implemented, and it should not be:

**(a) `GapCounts.exhausted` ≠ `MissLedger`-exhausted.** `MissLedger`-exhausted ids are held
out **at discovery** and never enter any `GapTracker` at all — confirmed by direct read of
`projection_reconcile.rs`: the `ReaDiscovery` arm (~1061-1064), the content arm
(~2107-2117, `divergent_refused` composition ~2760-2772), and the collectives arm
(~3881-3883) all match on `Admission::Exhausted` and increment a *_persistent counter
without ever pushing the id into the `admitted` vec that becomes `tracker.discover(admitted)`.
An id in this state structurally cannot appear in `GapCounts.pending` or
`GapCounts.exhausted` — it never reaches the tracker whose `.counts()` computes those
fields. Its divergent share is already folded into `divergent_refused`, which `b19f12014`
(the actionable-vs-total honesty pattern) already ships and already excludes from the
convergence-blocking term.

**(b) Per-sweep `GapTracker`s structurally cannot exhaust.** The reconcile arms build a
FRESH tracker every sweep and call `mark_failed` at most once per id per sweep (the in-leg
`call_with_retry` never touches the tracker). With `MAX_RETRIES = 3` this means
`GapCounts.exhausted` is 0 on every reconcile arm by construction — not by luck. Live
confirmation: alpha-A `/p2p/status` published `exhausted: 0` beside `pending: 2894` at
~2026-08-07 20:30Z, exactly matching the structural prediction, not the "handful of
permanently-dormant ids veto the gauge" story this doc originally told.

**(c) Doc + tests already landed.** `reconcile_rails.rs` now carries the
`GapCounts::exhausted` field doc distinguishing the per-tracker meaning from the
`MissLedger`/metric meaning (the name-collision trap this doc fell into), and
`tests::a_per_sweep_tracker_cannot_exhaust_under_a_multi_attempt_budget` pins (b) as a
regression guard. There is no further code change to make on this surface — the "fix
direction" section above is superseded, not pending.

**The TRUE pin on `converged` is the content-gap plateau**, not this. See
`content-gap-limit-cycle-blocks-convergence` — the standing dam is ~2894 pending content
gaps healing at 0-1/sweep against a `CONTENT_LEG_BUDGET` of 120s, plus intermittent
`divergent_actionable` noise. That doc, not this one, is where `converged` work continues.

### Follow-up plan adopted from the 2026-08-07 red-team review

The review did surface real, adjacent gaps worth carrying forward — not on this doc's
original claim, but on the honesty and adjudication surfaces nearby:

- **Stage 0 — observability.** Add gauge `converged_blocked_by{term}` so the next
  investigator can see WHICH term (`pending` vs `divergent_actionable` vs something new)
  is holding `converged` down, without re-deriving it from source each time.
- **Stage 1 — honesty floor.** Fold `failed == 0` into reconcile-level `converged` — this
  kills conductor-blind false-green class N1 (a dead conductor's failures silently drain
  from `pending` via `mark_failed`, per the `Converged softness` note in
  `content-gap-limit-cycle-blocks-convergence`). Add the measured precondition
  `peers_asked > 0 && no empty() short-circuit` — kills false-green classes N2/N3 (a sweep
  that asked nobody trivially reads "converged"). Fix the `metrics.rs:825` HELP string
  (currently describes the pre-honesty-fold semantics). Add a real guard test for the
  `Ok(None)`-sweep path (a sweep that returns no work should not silently read as
  converged).
- **Stage 2 — peer-probe adjudication.** `Answer::Absent` from the advertising peer, via
  `PeerHeadRecordFetcher`, graduates a `MissLedger` entry to peer-confirmed-absent;
  `Answer::Unreachable` never does (a dead/unreachable peer must not be read as evidence of
  absence). Split the metric into `exhausted_peer_confirmed` vs `exhausted_unverified` so
  the two classes stop sharing one number. This is the designed drain path picked up in
  `rea-stream-no-divergence-adjudication-drain-path`.
