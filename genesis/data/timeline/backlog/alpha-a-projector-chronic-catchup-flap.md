---
id: "backlog-alpha-a-projector-chronic-catchup-flap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "alpha-A projector chronically flaps catching-up ↔ serving — pins saga ch06 anchor-equality, ch09, ch10"
slug: "alpha-a-projector-chronic-catchup-flap"
written: "2026-07-31"
author: "claude (saga-final-chapters shift)"
status: "open"
priority: "high"
jobs: [elohim-edge]
tags: [dataplane, projector, alpha, doorway, reconcile, shem, hairpin, resiliency-saga]
cites:
  - genesis/a2o/features/dataplane/resiliency-saga/06-heads-converge.feature
  - genesis/data/timeline/backlog/shem-conductors-signal-hairpin-suspect-dht-silent.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
---

# alpha-A's projector never durably catches up

Evidence (2026-07-31, edge #1276 ~15:30Z and #1277 ~17:0xZ — identical
failure sets across two runs ~90min apart, with single-shot 200s in between):

- Multi-minute CI runs catch `GET /db/content` / `/db/humans` on alpha-A in
  `503 {"status":"catching-up","retryAfter":30}` phases; one-shot probes
  between runs get 200 — the projector OSCILLATES rather than converging.
- `p2p.divergentAnchor` GREW 1456 → 2031 between the runs.
- `elohim_projection_reconcile_converged` = 0 on alpha-A.
- alpha-A `/health` pools_healthy 3/7 while alpha-B reads 6/7 — A cannot
  reach the shem conductor set (adam/gertrude/susan/eve) from intel-nuc;
  suspected same class as the shem GFiber hairpin backlog (cited). With 4/7
  conductors unreachable the reconcile sweep can never complete, so the
  projector re-enters catch-up forever.

## What this pins

Saga chapters probing alpha-A's projections read red regardless of true
convergence (both doorways verifiably serve the SAME declared head + blob
for elohim-host-landing — direct probes 2026-07-31 ~14:5xZ):

- ch06 "resolves the same canonical head across peers" (1 of 7 scenarios)
- ch09 projectors-carry (reconcile_converged, /db/humans reads)
- ch10 card-tells-truth (stewardingCollectives — also intersects the
  known identity-coherence NULL-household_id inertness)

## Fix direction

Layer-routed per the p2p-vs-federation vocabulary: A→shem conductor
reachability is a dataplane/topology concern (hostAliases-style routing for
intel-nuc → shem, or pool-scoped CONDUCTOR_URLS so A's pool only contains
conductors it can reach and its reconcile can complete). Secondarily: a
projector catch-up gate that can't converge should surface as a named
exhaustion (runtime-findings), not silent oscillation.
