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

## UPDATE 2026-08-20 — it is not the projector, and it is not one-sided

Three claims in the record above are now measured to be wrong. Keeping the
title for continuity, but the diagnosis moves planes.

**1. No projector admission gate exists.** `caught_up` is a REPORTING field
only — read by `/p2p/status` and doorway `/health`, consulted by no admission
predicate anywhere in either crate. There are four independent shed gates and
none of them reads a projector cursor: storage per-request concurrency
(`elohim-storage/src/http.rs:1130-1163`, Retry-After 2), the conductor permit
pool (`conductor_admission.rs:344-406`, Retry-After 2), doorway's inbound
concurrency (`doorway/src/server/http.rs:4357-4398`, Retry-After 2), and
doorway's per-upstream circuit breaker (`routes/upstream_health.rs:83-126`,
Retry-After 30). The words "catching up" in the shed body are page copy, not a
description of the mechanism.

**2. It is the doorway→storage breaker.** MEASURED on `/admin/self-healing`,
both doorways, 2026-08-20 ~13:35-13:45Z: `admission.shedTotal: 0` on BOTH (the
admission gates shed nothing at all), while doorway-alpha's upstream circuit
was observed cycling `open` → `half-open` with `errorStreak: 3`. Server-side
confirmation: `doorway_upstream_breaker_open_total` on the A-side doorway pod
went 66 → 69 in six minutes. Note `ProxyOutcome::classify`
(`storage_proxy.rs:58-79`) counts only connect/timeout errors and real 5xx as
Failure — an upstream 429/503 is Neutral — so the breaker opening means the
doorway→storage hop is genuinely erroring or timing out, not that storage is
busy. Consistent with the client-side shape measured the same window:
`200 → 000 (connect/read timeout) → 503`, in ~2min windows.
(Do NOT use `lastGood: null` as evidence — `upstream_health.rs:282` documents
that field as never tracked; it is structurally always null.)

**3. Bilateral, not alpha-A-only.** Both doorways flap, largely independently.
Build #1366's own report already recorded it bilaterally
(`alpha-A=shedding, elohim.host=shedding`); the "A vs B" framing in the record
above and in later triage was reading one side of a two-sided condition.

### What this narrows — the chapters are pinned by AVAILABILITY ONLY

The decisive experiment (paired read taken inside a window where both doorways
answered 200 — window opened on the first attempt at 13:51:20Z):

| assertion | alpha-A | elohim.host | equal |
|---|---|---|---|
| `dhtAnchorHash` | `uhCkkKWBJv74VxbNzVT1jIQTwOIQbxTrJG5DX1PedqUaCfO228Hj5` | same | yes |
| `headActionHash` | `uhCkkKWBJv74VxbNzVT1jIQTwOIQbxTrJG5DX1PedqUaCfO228Hj5` | same | yes |
| `declared` | true | true | yes |
| `stewardingCollectives` | 1 | 1 | yes, and > 0 |

So the 2026-07-26 two-root split is GONE, the notary HEAD is declared and
agreed, and ch10's numeric compare passes. **ch04, ch06's two failing
scenarios, and ch10 fail only because a doorway is not serving at the instant
the probe runs.** Their value assertions hold. Caveat worth keeping:
`stewardingCollectives` reads 1, which is also its documented structural
ceiling (`services/peer_status_fanout.rs:5-10` — `peer_statuses` is self-only,
so the count caps at 1). The assertion passes at its floor; that is a separate,
real limit, not a pass to celebrate.

### Still unknown

What actually opens the breaker. Storage-side capacity is clean during the
episodes (`elohim_conductor_admission_in_flight` 0-5, `divergent_actionable`
single digits on matthew/adam, all 7 storage pods `up=1` with 0 restarts and
reconcile sweeps advancing steadily). Loki returned 502 on every query
attempted, so the doorway's own upstream error text was NOT retrievable —
that is an instrument outage, not an absence of errors. The B-side doorway pod
also has no Prometheus target (`up{pod=~"elohim-doorway-alpha-b-.*"}` empty),
so half the pair has no breaker telemetry at all. Fixing that scrape gap is
probably the cheapest next step toward naming the trigger.
