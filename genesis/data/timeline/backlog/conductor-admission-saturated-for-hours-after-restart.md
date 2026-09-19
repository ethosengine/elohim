---
id: "backlog-conductor-admission-saturated-for-hours-after-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "After a storage restart the conductor-admission gate stays full for hours, so nobody can author a head — the app pipeline cannot deliver behind an edge roll"
slug: "conductor-admission-saturated-for-hours-after-restart"
written: "2026-09-19"
author: "pipeline-shakeout shift pickup (2026-09-19)"
status: "backlog"
priority: "high"
tags: [dataplane, conductor-admission, restart, app-delivery, alpha-fleet]
jobs: [elohim, elohim-edge]
cites:
  - elohim/elohim-storage/src/conductor_admission.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - scripts/ci/stage-spa-blob.sh
  - genesis/data/timeline/backlog/storage-sqlite-locked-surfaces-as-500-despite-busy-timeout.md
---

# The admission gate does not drain after a restart

## What was observed

`elohim_conductor_admission_in_flight`, namespace `elohim-alpha`, capacity 5, 30-minute samples 2026-09-19:

- matthew: `5 5 5 5 4 4 4 5 5` from 03:00Z to 07:00Z (storage restarted by edge/dev 1462 at about 03:00Z), then
  `0 0 1 1 1 1 0 1` until its next restart at 11:04Z (edge/dev 1467), then `5 5 5 5` through 12:50Z and counting.
- susan: `5 4 5 4 5 5 5 5 5` from 07:00Z to 11:00Z after the 06:24Z restart, then 0.
- adam, james, jessica, eve: 3–5 for the two hours following each restart.

Loki, `elohim-matthew-alpha-0`, 12:05:48Z–12:50:38Z, repeating:
`conductor admission: shed: no conductor permit for infrastructure within 5000ms (class=interactive, capacity=5, in_flight=5) — nothing was dispatched`
— from the heartbeat, discovery enumeration, peer-status fan-out, household replayer, identity fill, shard
registration, projection reconcile, and at 12:40:46Z from `elohim_storage::provide` for `head_ref: "elohim-host-landing"`.
Alongside it, `apply_snapshot: database is locked` (the already-captured SQLite gap).

The runbook's "restart churn ≈ 20 min" does not describe this. The gate is full for four hours.

## What it breaks

elohim/dev 1712 ran directly behind the 1467 roll. For more than forty minutes on both public doorways every
`blobHash` / `serverBlobHash` PATCH came back `503 {"status":"catching-up","cause":"upstream","circuit":"closed"}`
(the doorway honouring storage's own shed, `storage_proxy.rs:816`), every 3–4.5 MB blob forward to storage timed
out, and the stage concluded `NO doorway could author elohim-host-landing — no live conductor bridge in the
fabric`. Reads were healthy throughout (`/db/p2p/conductor-diagnostics` 200 in under 150 ms, 53 agents), which is
why edge validation — all reads — passed minutes earlier on the same fleet.

It is also the likeliest reason `elohim.host` has served the 09-13 landing row while `doorway-alpha` serves the
09-14 one: heads authored behind a roll do not land.

## Two things the evidence separates

1. The heartbeat — named in the module's own doc as the example of Background work — is shedding as
   `class=interactive`. Classification at the call sites has drifted from the contract.
2. Class is only a wait bound; both classes draw from one FIFO pool with no capacity held back for a person-facing
   write. A dozen cold background loops can hold all five permits indefinitely.

The five are slow calls, not lost ones. At 12:55Z, ten-minute window: matthew releases 5.4 permits a minute with
a mean hold of **55.8 seconds** per conductor call; susan, drained, releases 14 a minute at **4 milliseconds**.
Five permits at 56 s each is exactly 5.4 a minute — the gate is full because matthew's conductor takes a minute
to answer anything, for hours after the restart. The gate is doing its job; the scarce thing is the conductor.

## Missing nodes

- chain / between "storage restarted" → "a head can be authored through this peer" / missing node "the admission
  gate has spare capacity": assertion — `in_flight < capacity` sustained for one sweep; probe — the gauge above.
  State: **unprobed**; neither the edge roll gate nor the app pipeline reads it.
- chain / between "edge deploy finished" → "app pipeline stages heads" / missing node "the fleet accepts writes":
  the orchestrator sequences app directly behind edge, and the app stage's 360 s per-doorway budget is two orders
  of magnitude shorter than the observed saturation. State: **not built**.

## Needs a design decision

Reserved interactive capacity versus a background admission ceiling versus pacing the cold-start loops is a
capacity-contract change in `conductor_admission.rs` — route to a brainstorm, not to iteration.
