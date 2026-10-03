---
id: "backlog-self-heal-alpha-projector-one-sweep-read-as-three"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The projector predicate counted one published sweep as three: three polls read matthew's first post-restart sweep and filed exhaustion on a projector that converged on its next sweep (FIXED — repeated reads of one sweep are one observation)"
slug: "self-heal-alpha-projector-one-sweep-read-as-three"
written: "2026-09-27"
author: "runtime-triage"
status: "wip"
priority: "medium"
self_heal_status: in-progress
severity: low
fingerprints: [6cdded115d74]
nodes: [alpha]
relatedNodeIds: []
tags: [self-heal, projector-reconcile, sensing-gap, elevate-arm, runtime-harvest, false-positive, post-restart, poll-is-not-a-sweep, matthew, performance, perf-telemetry, perf-convergence, friction-blind, phase-steady, lane-operator]
cites:
  - https://doorway-alpha.elohim.host/p2p/status
  - https://doorway-alpha.elohim.host/admin/self-healing
  - .claude/scripts/_lib/runtime_harvest.py
  - .claude/scripts/_lib/__tests__/runtime_harvest_test.py
  - .claude/data/runtime-cursor.json
  - elohim/elohim-storage/src/p2p/projection_reconcile.rs
  - elohim/elohim-storage/src/metrics.rs
  - genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md
---

# One published sweep, read three times, is not three sweeps

## What is exhausted

Nothing was exhausted on the node. The ledger line, `.claude/data/runtime-findings.jsonl`
(poll 203):

```json
{"fp": "6cdded115d74", "class": "self-heal-exhaustion", "node": "alpha",
 "provenance": "projector:reconcile",
 "line": "projector healed NOTHING (healedTotal 0 over 5-5 sweeps, divergentAnchor 60, converged=false) sustained >= 3 polls",
 "status": "open", "seen": 1, "first_poll": 203, "last_poll": 203,
 "ts": "2026-09-27T22:53:56+00:00"}
```

"Over 5-5 sweeps" is the tell: the three samples behind the finding are one sweep. The
stored alpha ring in `.claude/data/runtime-cursor.json` (polls 196–203), read as
`p2p_status.projectionReconcile`:

| ring slot | sweeps | pending | failed | peersAsked | divergentAnchor | converged |
|---|---|---|---|---|---|---|
| 1 | 95 | 1 | 0 | 6 | 59 | false |
| 2 | 119 | 0 | 0 | 5 | 59 | **true** |
| 3 | 134 | 0 | 0 | 5 | 59 | **true** |
| 4 | 134 | 0 | 0 | 5 | 59 | **true** |
| 5 | 152 | 0 | 0 | 5 | 59 | **true** |
| 6 | **5** | 48 | 3 | 6 | 60 | false |
| 7 | **5** | 48 | 3 | 6 | 60 | false |
| 8 | **5** | 48 | 3 | 6 | 60 | false |

Slots 2–5 are one process that had converged. Slots 6–8 are a new storage process (the
counter reset from 152 to 5), and they are byte-identical: its fifth sweep, published with
48 pending rows and 3 failed, re-read by three SessionStart polls while sweep 6 was still
running. `converged` is published once per completed sweep
(`elohim/elohim-storage/src/metrics.rs` `converged_blockers`: `pending` and `failed` both
block it), so a single sweep that ends with pending rows reads `converged: false` for as long
as the next sweep takes.

## Re-fetch at triage — self-resolved

`https://doorway-alpha.elohim.host/p2p/status`, 2026-09-27:

```
22:55:39Z {"pending": 0, "failed": 0, "caughtUp": true, "peersAsked": 6, "divergentAnchor": 60, "healedTotal": 0, "sweeps": 6, "converged": true}
22:55:59Z (identical)
22:56:19Z (identical)
22:59:04Z (identical)
```

`https://doorway-alpha.elohim.host/admin/self-healing` at 22:55:21Z and 22:59:04Z:
`admission {maxInflight: 256, available: 256, shedTotal: 0}`, upstream circuit `closed`,
`projector {caughtUp: true, divergentAnchor: 60}`, `conductor {connected: true,
connectedWorkers: 4/4}`. Sweep 6 converged within minutes of the filing, so the fresh process
caught up on its sixth sweep. `healedTotal: 0` beside `converged: true` is the fleet's normal
reading: divergent anchors are adjudicated, not healed (see the 2026-09-24 section of the adam
record cited above).

## Root-cause inventory

- `.claude/scripts/_lib/runtime_harvest.py` `_projector_lag`, measured arm: it took the last
  `LAG_POLLS` (3) measured reports from the ring and required `_heals_nothing` on each. It
  never asked whether those reports were different sweeps. A poll is a SessionStart, not a
  sweep, and bursts of sessions put several polls inside one sweep.
- The unmeasured arm has required "the sweep counter MOVED inside the window" since
  2026-09-26 (`len(set(sweeps)) > 1`). The measured arm never got the same rule.
- The risk was named and left open in
  `genesis/data/timeline/backlog/self-heal-adam-projection-catchup-exhaustion-full-arc.md`,
  2026-09-24 section: polls 159 and 160 there were one sweep, and "three polls that each land
  on an unconverged sweep would file a matthew finding … The risk is latent and named here. It
  is not fixed." This filing is that risk occurring, in a sharper form: three polls on *one*
  unconverged sweep.
- The node side is healthy. `elohim/elohim-storage/src/p2p/projection_reconcile.rs` publishes
  exactly what it documents. A first post-restart sweep that ends with pending rows is
  expected churn.

## Fix path

`_distinct_sweeps` collapses each run of adjacent reports with the same `sweeps` value into
one observation (the last of the run). The measured arm then takes its last `LAG_POLLS`
observations from the collapsed ring. As before, it reads the whole ring, so repeated reads
of one sweep cannot build a clean streak that deletes a live line either.

- The multi-process lesson still holds: a non-monotonic 88, 109, 21 is three observations.
  The check is equality between neighbours, with no subtraction across processes.
- A genuine ceiling with slow sweeps still files. The fixture 45, 46, 46, 47, 47, 47 fires.
  A blocked ceiling line is neither deleted nor re-filed while polls repeat one sweep of it.
- Tests: `.claude/scripts/_lib/__tests__/runtime_harvest_test.py`, 5 new assertions
  (69 pass). The first uses the live alpha ring above. It fails with the collapse disabled
  and passes with it. `residual_channel_test.py` and `harvester_blind_test.py` also pass.

**The trade this fix makes:** a sweep that wedges (never completes, so its counter never
moves) no longer files on this arm, because a frozen counter collapses to one observation.
Before this fix, that case filed only because a frozen report happened to be read three
times, the same shape as this false positive, so the poller could not tell a wedge from a
slow sweep anyway. An honest wedge sensor needs a duration: storage would have to publish when
the current sweep started, or the process age. The 2026-09-24 section of the adam record names
that change (`projectionReconcile.startedAt` / `uptimeSecs`, which needs a storage change and
a fleet roll). This pass does not build it.

**Still latent, not fixed:** matthew's `converged` can flip from one sweep to the next as new
pending rows appear and are adjudicated. Three *distinct* consecutive unconverged sweeps would
still file. That would at least be three sweeps of evidence, and the duration-denominated
grace named above is its cure too.

## Current decision

**FIXED (poller-side).** No node or cluster change is needed. Ledger line `6cdded115d74` is set
to `status: triaged` and cites this file. With the fix, the stored alpha ring replays silent
(`rh.evaluate` over `.claude/data/runtime-cursor.json` returns no alpha finding), so the poller
will close the line by disappearance after `CLOSE_STREAK` (5) clean polls. If this fp re-files,
check the ring first. It should hold three *distinct* unconverged sweeps from one process;
that is the latent case above, and it is real evidence. If the reads are repeats of one sweep,
this fix has regressed.

## Verification

- 2026-09-27 22:55:21Z–22:59:04Z: re-fetched `https://doorway-alpha.elohim.host/p2p/status`
  four times and `/admin/self-healing` twice. All were HTTP 200; the readings are quoted above.
  Sweep 6 reported `converged: true, pending: 0, failed: 0`.
- Read-only replay of the committed predicate over the stored ring gave `alpha → []`.
  The same replay still files `79f357281ca5` for alpha-b from distinct sweeps: that is a
  separate concern with its own record, and this pass did not touch it.
- `python3 .claude/scripts/_lib/__tests__/runtime_harvest_test.py`: 69 assertions passed,
  exit 0.
- No cargo, mesh, cluster action or push in this pass.

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): cured poller-side in code; tests not re-run. `_distinct_sweeps` collapses repeated reads of one sweep and the measured arm applies it before the LAG_POLLS window (.claude/scripts/_lib/runtime_harvest.py:244-273,350-358). The wedge-duration signal is still absent: no startedAt or uptime in projection_reconcile.rs or metrics.rs. Same mechanism as: self-heal-adam-projection-catchup-exhaustion-full-arc (sensing, not cost). Confirming measurement: replay the poller over three identical sweeps reads and over three distinct unconverged sweeps.
