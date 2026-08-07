---
id: "backlog-rea-stream-no-divergence-adjudication-drain-path"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "REA stream has no divergence-adjudication drain path — no declared-head-equivalent primitive means MissLedger cycling is the only fate for live REA anchor divergence"
slug: "rea-stream-no-divergence-adjudication-drain-path"
written: "2026-08-07"
author: "claude (dataplane convergence gate, operator-directed)"
status: "open"
priority: "medium"
tags: [projection-reconcile, rea, divergence, declared-head, adopt-deferred, contest, election, canonical-declare, design-gap, observability]
cites:
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - content-divergence-unhealable-without-canonical-heads
  - content-gap-limit-cycle-blocks-convergence
  - rea-heal-classify-write-toctou-transactionalize
  - miss-ledger-exhausted-ids-veto-converged-forever
---

# REA stream has no divergence-adjudication drain path (design gap, open)

**Status:** open — design gap, not yet a bounded code fix.
**Owner surface:** `elohim/elohim-storage/src/p2p/projection_reconcile.rs`, REA arm
(`ReaHealOutcome`, `ReaDiscovery`).

## Finding

Content divergence has a real drain path off the MissLedger: contest → DHT election
→ canonical declare. `adopt_deferred_heads` supplies candidates, `ContentHealOutcome`
carries both `adopt` and `ghost` candidate classes, and a row that resolves gets a
declared head an in-zome `select_canonical_winner` election can arbitrate (the
mechanism landed across the 2026-08-01/02 conductor-plane RCA — see
`content-gap-limit-cycle-blocks-convergence`).

`ReaHealOutcome` carries none of this. The code names the gap directly:

> "REA rows carry no declaration-ordering column, so there is no 'already declared'
> refusal on this arm (the content arm's dominant class)." — `projection_reconcile.rs`
> ~line 974

There is no REA-side equivalent of a declared head, no contest primitive, and no
election path. The only fates available to a divergent REA row are: refused this
sweep (adjudicated, correctly, but not resolved), or exhausted into the MissLedger
(see `miss-ledger-exhausted-ids-veto-converged-forever`). Neither fate ever produces
a resolved anchor. Live evidence: 12 REA rows on matthew are anchor-divergent and
have cycled refused/exhausted for multiple sweeps with no mechanism that could ever
move them — this is not a slow drain, it is a structural dead end for this arm.

## Secondary gap — observability

Per-row REA reconcile ids are only emitted at `tracing::debug!` level
(`projection_reconcile.rs` ~line 2210, e.g. the "own conductor returned None" retry
line). Production Loki ingestion runs at a level that does not capture these, so the
12 live divergent REA row ids on matthew are currently unidentifiable from Loki
alone — naming them requires either direct DB access or a temporary log-level
change. This blocks even sampling the divergent set to characterize what's actually
diverging (content type, age, which peers disagree) before designing the fix.

## Needed

A REA-side declared-head-equivalent / adjudication primitive: some column or link
that lets a REA commitment carry an ordering claim analogous to content's
`declared_head_action_hash`, plus a contest/election path (or a deliberately simpler
REA-appropriate resolution rule, since REA commitments may not need the full
content-declare ceremony) that gives divergent REA rows a route to resolution other
than cycling the MissLedger forever. Until this lands, REA anchor divergence is a
permanent (if small) drag on `converged` — bounded today by the low live count
(0-12), but with no cure, only containment.

## 2026-08-07 — red-team review confirms and sharpens the cure

Same-day red-team review (companion pass to the `miss-ledger-exhausted-ids-veto-
converged-forever` premise-falsification) confirmed this design gap stands as
described and sharpened the shape of the fix:

- **The designed drain is Stage 2 peer-probe adjudication.** `Answer::Absent` from the
  advertising peer (via `PeerHeadRecordFetcher`) graduates a divergent/`MissLedger`
  entry; `Answer::Unreachable` never does — a dead peer is not evidence of absence.
  Splitting the metric into `exhausted_peer_confirmed` vs `exhausted_unverified` gives
  REA the SAME resolution primitive content already has via `select_canonical_winner`,
  without requiring REA to grow the full content-declare ceremony this doc already
  rules out as overkill.
- **REA/collectives structurally have no `ContentHeadRecord` analog.** Content's
  adjudication path runs entirely on `ContentHeadRecord` / `declared_head_action_hash`
  plumbing; REA has no equivalent record type to carry an ordering claim, which is why
  `ReaHealOutcome` cannot express "already declared" the way the content arm does. A
  REA-side declared-head-equivalent record is the concrete artifact the "Needed" section
  above is asking for — peer-probe adjudication (above) is the drain path that record
  would unlock, not a substitute for it.
- **`PeerHeadHint.alternates` (cap 3) is the existing advertiser set for content** and is
  the shape a REA-side hint would want to mirror: other peers that advertised the same
  head this sweep, recorded as substitute couriers when the primary advertiser can't
  serve the bytes (`head_adoption.rs` ~line 300). A REA hint carrying an analogous
  alternates set gives Stage 2 adjudication more than one peer to probe before it can
  graduate an entry to peer-confirmed-absent.

This doc's status stays `open` — the design gap is confirmed, not yet a bounded code
fix; Stage 2 above is the shape the fix should take when picked up.
