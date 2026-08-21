---
id: "backlog-doorway-breaker-trial-theft-fleet-verification"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway upstream-breaker trial theft + /apps extraction-flight herd: cures landed host-green, fleet verification and the route-class question remain"
slug: "doorway-breaker-trial-theft-fleet-verification"
written: "2026-08-21"
author: "claude (doorway-B 503 root-cause session)"
status: "wip"
priority: "high"
jobs: [elohim-edge]
nodes: [elohim-doorway-alpha, elohim-doorway-alpha-b, elohim-adam-alpha, elohim-matthew-alpha]
relatedNodeIds:
  - "memory:project_alpha_substrate_probe_rails"
  - "memory:project_doorway_ops_incidents"
  - "memory:feedback_reach_head_replication_distinct_planes"
tags: [doorway, breaker, dataplane, availability, resiliency-saga, doorway-failover, extraction-cache, observability]
cites:
  - doorway/doorway-service/src/routes/upstream_health.rs
  - doorway/doorway-service/src/routes/storage_proxy.rs
  - doorway/doorway-service/src/routes/health.rs
  - doorway/doorway-service/src/server/http.rs
  - elohim/elohim-storage/src/http.rs
  - elohim/elohim-cache-core/src/extraction/cache.rs
  - genesis/data/timeline/backlog/alpha-a-projector-chronic-catchup-flap.md
  - genesis/data/timeline/backlog/self-heal-doorway-alpha-storage-breaker-matthew-rekey.md
  - genesis/data/timeline/backlog/ci-rbac-jenkins-deployer.md
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
shift_objective: |
  Verify on the alpha fleet that the four 2026-08-21 doorway/storage cures hold:
  (1) no breaker latches past one 30s cooldown while its peer answers,
  (2) /apps/<id>/index.html stays sub-second on both peers under concurrency,
  (3) /health and /health/serving demote in both the shedding and the slow regime,
  (4) doorway B has a Prometheus scrape target.
  Measure with `[build:edge] [edge:validate-only]` — never a bare [build:edge].
  Then decide the route-class question in "Open design decision" below.
---

# What was cured, and what is still open

## The chain, as measured 2026-08-20/21

`elohim.host` served `503 {"cause":"upstream","circuit":"half-open","errorStreak":3}` on every
content route while its own `/health` read `healthy:true, status:"online"`. Four defects stacked,
each proven separately:

1. **Storage: the `/apps` extraction flight leaked a herd.** In `serve_app_file`
   (`elohim/elohim-storage/src/http.rs`) a waiter whose post-wait cache re-check MISSED fell
   through and extracted **without ever registering** in `in_flight` — and then created an
   `extraction_guard` that broadcast `finish_extraction` for a flight it never held. So when the
   first extractor's `put_app` failed, every waiter missed, all became simultaneous extractors,
   and their concurrent `put_app` calls raced `evict_app`'s `remove_dir_all` against each other's
   directory writes → `Directory not empty (os error 39)`. `put_app` removes the index entry
   BEFORE writing and restores it only on success, so each failure left the app permanently
   uncached → every later request re-extracted the whole Angular bundle → more concurrency.
   Self-sustaining. Live evidence: two `"Failed to cache extraction (non-fatal)"` /
   `identifier: "elohim-host-landing"` failures **8 ms apart** on 2026-08-20 20:11:27Z.
2. **Doorway: those slow `/apps` reads open the breaker.** The SSR shell fetch rides
   `EPR_DISPATCH_TIMEOUT_SECS = 10` — SHORTER than `STORAGE_PROXY_REQUEST_TIMEOUT_SECS = 12` — so
   the homepage render trips the breaker first. Three timeouts in ~25s = threshold.
3. **Doorway: the breaker is keyed by ENDPOINT, so it sheds every route on that peer.**
   `/db/content/elohim-host-landing` answers in **40–65 ms** throughout; it was shed as collateral
   for `/apps` failures. Two sites also recorded a `Failure` for an upstream 429/503 in violation
   of `ProxyOutcome::classify` (which makes backpressure Neutral precisely to stop this).
4. **Doorway: the recovery trial was being stolen.** `UpstreamBreakers::is_open()` is a GATE, not a
   read — it advances Open→HalfOpen and consumes the one half-open trial. Two `/`-path planners
   (`http.rs` `dispatch_to_projected_epr`, `resolve_projected_shell`) called it with no
   `BreakerTrial` guard, so no outcome was ever recorded and the circuit re-latched every
   `STALE_HALFOPEN_COOLDOWN_MULTIPLIER × cooldown = 120s`. Fixed 2026-07-21 (`f5e22baa2`),
   **reintroduced 2026-08-18 (`f0b908660`, warm-boot shell cache)** — the in-tree comment at the
   second site asserted "`is_open` only READS the circuit", sitting directly on the bug it named.

Cured in-tree 2026-08-21: the flight now re-enters instead of falling through (bounded by
`MAX_EXTRACTION_COALESCE_ROUNDS`) and only the flight's owner creates the guard; `evict_app` deletes
the prefix unconditionally; both planners use a new non-mutating `would_shed()` and the raw gate is
`#[cfg(test)]` so production cannot reach it a fourth time; both `classify` violations defer to the
one classifier; `snapshot()` stops reporting a HalfOpen circuit as `skipped:false`; `/health` gains a
`serving` block and demotes on BOTH the shedding and the slow regime, and `/health/serving` carries
the status code `/health` cannot.

## Reconciliation with "Defect B"

Same CLASS, different trigger, and the amplifier that made every prior instance unreadable is gone.
Every prior instance (2026-07-18, 2026-07-23, 2026-07-31) was fed by storage's own honored 503s, so
"the breaker opened" carried no information about whether storage was alive; `69a0e336e`
(2026-08-16) cut that loop. `doorway_upstream_backpressure_honored_total == 0` on **both** doorways
proves today's opens are genuinely never-answered hops. `$arcFactorRevert`'s unresolved puzzle —
"matthew is ALREADY full-arc and STILL times out" — is answered without arc: the timing-out route
was never `/db/content`, it was `/apps/<id>/index.html`.

## Still OPEN

**1. Fleet verification.** All cures are host-green only (doorway gate, storage gate, cache-core
47/47). Nothing is deployed. Measure with `[build:edge] [edge:validate-only]`; a bare `[build:edge]`
restarts the seven pods it is measuring.

**2. Open design decision — should the breaker be route-class aware?** Today one endpoint key
governs every proxied route, so a slow *app-bundle* read sheds *content* reads that answer in 40 ms.
Splitting the key (or weighting by route class) would have contained this incident to `/apps`. The
counter-argument is real: a genuinely dead peer should shed everything, and per-route keys multiply
the state a half-open trial has to cover. Decide deliberately; do not drift into it.

**3. The instrument that would have settled this in minutes is dark by construction.**
`doorway_hop_duration_ms{hop,outcome,route_class}` reads `_count 1, _sum 0` on both doorways after
16+ hours and 250+ sheds. `derive_hop_tier` maps `1..=2 => HopTier::Off` and reads
`available_parallelism()`; doorway B is `1000m`, doorway A `2000m`, so **both land in the Off band
and no doorway can opt in below 3000m**. `DOORWAY_HOP_METRICS` appears in no manifest. This is the
one measurement that distinguishes "peer is slow" from "peer is down". *(Belongs to the separately
queued latency-instrument lane — recorded here, not absorbed.)*

**4. Operator-owned: the podmonitors RBAC grant.** `alpha-doorway-podmonitor.yaml` is now applied by
the edge pipeline (through the tolerant helper, so a Forbidden cannot red a deploy), but it only
takes effect once jenkins-deployer actually holds
`monitoring.coreos.com/podmonitors {get,create,patch}`. Until then doorway B stays unscraped and
every breaker/shed series exists for A only. See `ci-rbac-jenkins-deployer.md`.

**5. Unrelated, found while measuring:** `up{job="elohim-alpha/iroh-relay-alpha"}` and `…-alpha-b`
both read **0** — neither iroh relay pod is scrapable. Route to the dual-stack iroh lane; not
touched here.

## How closure is measured

- `doorway_upstream_breaker_open_total` stops climbing on both doorways while both peers answer.
- No `circuit:"half-open"` observed on `/admin/self-healing` for longer than one 30s cooldown.
- `/apps/lamad-spa/index.html` sub-second on adam (was 0.92 s vs matthew's 0.076 s on 2026-08-21).
- `/health` reads `healthy:false, status:"degraded"` and `/health/serving` returns 503 whenever the
  serving path is shedding OR degrading — the gap that let this run for hours unflagged.
- a2o `@concern:doorway-failover` and saga ch04 `@concern:saga-04-doorway-serves` green on a
  validate-only run.
