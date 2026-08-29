---
id: "backlog-pull-leg-drains-before-iroh-book-warms"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A warm-restarted peer drains its whole pull queue in the first seconds after boot, before its iroh peer book has learned any peer — so the transport selector sees single-plane peers and libp2p carries every pull with no decision to make"
slug: "pull-leg-drains-before-iroh-book-warms"
written: "2026-08-29"
author: "M4 transport self-awareness cut"
status: "wip"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "habit:dataplane-convergence"
tags: [dataplane, iroh, peer-book, boot-ordering, transport-selection, ratchet-lane-P]
---

## Measured (dual household mesh, 2026-08-29, warm recovery jessica<-matthew, cut m4-select-v2)

- jessica booted ~02:53:00Z; `iroh peer learned from transport manifest` at 02:53:08Z (james) and 02:53:10Z
  (matthew, relayed); the acquisition drain (5 s cadence, budget 50) had already dispatched all 11 pulls —
  `elohim_acquisition_dispatch_total{transport="libp2p"} 11`, **zero** `elohim_transport_route_total` rows:
  the planner saw no dual peer, so no Bulk decision existed to make.
- The same run's Small races DID sample iroh (jessica->matthew 15 ms vs libp2p 816 ms degraded), and the
  write-time push routed every shard `transport=iroh` — selection works once the book is warm; the pull
  leg simply runs before it is.

## Cure shapes

1. Persist the iroh peer book across restarts (a warm cache the announcer refreshes), or
2. gate the FIRST acquisition drain on `doorway_bootstrap` / the first manifest round (bounded wait, e.g.
   ≤10 s, never indefinite), or
3. seed the book from the doorway's `/p2p/manifests` BEFORE the drain loop starts (the T0' bootstrap already
   exists but reports "watching for an empty peer book" after the drain).

Fleet relevance: a long-running peer's book is warm and its pull leg runs continuously, so the fleet
reading of `route_total{reason}` is where the selector's Bulk behaviour actually shows.

## 2026-08-29 cure shape 2 landed (local evidence; mesh measurement pending)

`acquisition::first_drain` (pure, 5 tests) gates the FIRST dispatch tick: hold while the iroh book is empty and
`< FIRST_DRAIN_HOLD` (10 s) has elapsed; release on `book_warm` (the moment the book learns a peer), `no_iroh_leg`
(nothing to wait for) or `expired` (bounded, never indefinite). Counted in
`elohim_acquisition_first_drain_total{outcome}` (pre-touched). Read beside `route_total`: a boot released `expired`
routed with single-plane peers, so a flat route series there is the book, not the selector. To measure: warm
recovery on the dual mesh — expect `route_total{op_class="bulk"}` rows to appear on the recovering peer's first drain.

## 2026-08-29 MEASURED: the hold is honest but too short on this mesh — next cut is shape 3

Warm recovery jessica (dual, survivors' books warm): `P2P node started` 11:22:36.49 → `first acquisition drain
released reason=expired since_boot_ms=10004` → first `iroh peer learned from transport manifest` at **+28 s**
(11:23:04.81); manifest rounds then land every 30 s (:04/:07/:34/:37). All three peers on the simultaneous
`storage-restart` read the same `held 2 → expired 1`. So `FIRST_DRAIN_HOLD` = 10 s covers the row's +8..10 s
measurement but not this mesh's ~30 s announcer cadence; a hold sized to the cadence would tax every boot by 30 s.
The pull leg nevertheless ROUTED this run — the reconcile keeps re-enqueueing across a 3.4k-row recovery, so the
selector saw dual peers from the second minute on: 34 bulk decisions, `best_rtt` iroh 27 : libp2p 3 (EWMA 18 ms
vs 367 ms), dispatch iroh 225 / libp2p 73. The one-shot small-queue shape (11 pulls, gone before the book warms)
is what shape 3 cures: seed the book from the doorway's `/p2p/manifests` BEFORE the first drain (today the T0'
bootstrap waits its 30 s grace and then only watches for an EMPTY book). Keep the 10 s hold as the bounded floor;
make the doorway seed the release, not the deadline.
