---
id: "backlog-projection-reconcile-actionable-sawtooth"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Projection reconcile never rests on alpha — ~50 actionable divergences reappear on every storage peer each 20–30 minutes, so the quiesce sustain window is a coin toss"
slug: "projection-reconcile-actionable-sawtooth"
written: "2026-09-19"
author: "pipeline-shakeout shift pickup (2026-09-19)"
status: "backlog"
priority: "high"
tags: [dataplane, projection-reconcile, quiesce, alpha-fleet, measurement]
jobs: [elohim-edge]
cites:
  - scripts/ci/fleet-quiesce-gate.sh
  - genesis/data/timeline/backlog/fleet-quiesce-pass-not-convergence.md
  - genesis/data/timeline/backlog/edge-quiesce-gate-timeout-aborts.md
---

# Projection reconcile sawtooth on the alpha fleet

## What was observed

Prometheus, `elohim_projection_reconcile_divergent_actionable`, namespace `elohim-alpha`, read 2026-09-19 05:20Z:

- 10-minute samples over 90 minutes — eve: `0 51 51 14 0 0 0 51 0 51`; susan: `0 47 47 6 0 0 0 47 0 47`;
  adam: `0 49 0 0 0 53 0 6 0 4`; jessica held 51–65 for an hour, then 0.
- Hourly maxima over the retained window (about 14 hours, reaching back well before the 6ab3c790 storage
  roll): matthew, adam and eve each peak at 50–57 **every hour**.
- Doorway `/p2p/status` `projectionReconcile`: `divergentAnchor` fixed at 58, `healedTotal` 0,
  `converged` false, on both public doorways, across six polls.

The same ~50 items are declared actionable, drop out, and come back. Nothing records them as healed.

## Why it matters

The edge quiesce gate reads matthew alone and needs a sustained quiet window. With a peak every 20–30
minutes, whether a build's Dataplane Validation measures depends on where in the cycle it starts —
edge/dev 1462 quiesced, diverged for 34 minutes, quiesced again and ran out 27 seconds short. That build's
investigator could not rule out the new storage code; the pre-roll history above does: the pattern predates it.

It also makes "wait for the fleet to settle before rolling" unanswerable by this metric.

## Missing node

- chain / between "reconcile sweep classifies an anchor as actionable" → "fleet reports quiesced" /
  missing node "an actionable anchor is either healed (counted) or reclassified (named reason)":
  assertion — across one hour with no authoring, `divergent_actionable` on a peer is non-increasing and
  every drop is matched by `healedTotal` or a labelled reclassification counter; probe — the range query
  above plus `GET /db/p2p/conductor-diagnostics`. State: **not built; cause unknown**.

## Where to look first

Which ~50 anchors they are (the count is close to the 51 that `elohim.host` reports and to the REA
stream counts named in the 09-19 handoff), and whether the rise coincides with a periodic sweep phase
that reports "pending, not yet compared" as "actionable".

## 2026-09-19 — it already costs a scenario

Two validate-only measurements of the same fleet, same commit, one hour apart: edge/dev 1465 (08:31Z) ran
122 scenarios with 10 passed and 0 failed; edge/dev 1466 (09:3xZ) ran the same 122 with 9 passed and 1 failed —
`inventory-convergence.feature:42`, "the seed-facing doorway peer catches its projection up under sustained
gossip", on `alpha-A /health: p2p.caughtUp is false`. Nothing was deployed between them. The step reads the flag
once; the flag follows the sweep phase (doorway `/p2p/status` `projectionReconcile.caughtUp` was seen true,
false, true at five-minute polls with `pending` 0, 34, 0). The scenario's verdict is the phase of the sawtooth at
the instant it reads.

## 2026-09-19 — the CI-side concern this blocks

`backlog/ci-edge-inventory-convergence-caughtup-sawtooth-flap.md` is the CI half:
`inventory-convergence.feature:42` samples `p2p.caughtUp` once and has now failed
in elohim-edge 1444, 1449, 1456, 1457 (peer `elohim.host`) and 1466 (peer
`alpha-A`), green in 1467 — the coin toss above, counted. It is parked
`ci_status: blocked` naming THIS doc's missing node as what unblocks it, and it
deliberately declines to add a settle window (a 20-30 minute cycle means a
bounded poll either does not help or masks the sawtooth). When the reconcile
cure lands, that scenario is its free confirmation probe.
