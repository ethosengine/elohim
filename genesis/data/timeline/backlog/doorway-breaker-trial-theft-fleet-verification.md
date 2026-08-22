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

**The amplifier — found after the first pass, and larger than any single link above.** The
warm-boot shell cache that was supposed to make `/` cache-first has been **dead code on every
deployed doorway since it landed**. `main.rs` builds AppState via `with_pool` (343) or
`with_services` (345); both set `warm_shell: WarmShellStore::inert()` because no archive exists at
construction, and the only constructor that ever built a live store (`with_projection`) has zero
production callers. The archive arrives later in `init_projection`, which installed
`app_file_cache` and never rebuilt the store. An inert store's `lookup_with_declared` returns
`Cold` *before* it consults the hot map, so `stock()` writes are unreadable and `hydrate()` returns
0 unconditionally — the boot log's `hydrated: 0` reads as a cold archive rather than a disabled
one. Hence `decide_shell_serve(Cold, true) = Fetch` and `/` paid a full 10 s fetch per request plus
a second through the `ProjectedEpr` fallback: the measured 20.751 s. Cured by a named
`bind_warm_shell_to_archive()`; the suite's nine existing tests all built stores *with* an archive,
so nothing tested the shape production used.

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

**Decided 2026-08-21 (operator):** the breaker key stays per-endpoint; it is the *shed* that becomes
class-aware, not the circuit. Freshness tolerance is graded by the read's declared stakes —
`Knowledge` (content/blob/app-bundle) → serve the last-good bytes marked `x-elohim-freshness: amber`;
`ValueCoupled` → amber with warning; `Authority` (auth/session/identity/governance, head-declare,
all writes) and `CounterEvidence` → green-only, i.e. the existing honest 503 (plus
`x-elohim-freshness-required: green`). Stage (`ELOHIM_NETWORK_STAKES`, Bootstrap default) prices the
non-floor rows. Being behind becomes a trust signal on the wire, not an outage. Predicate lives in
`crates/seam-contracts` (`freshness::verdict`), consumed by the doorway first; storage's own
projector-lag shed consuming the same predicate is rung 2. Spec of record: memory
`project_freshness_graded_by_declared_stakes`; a2o flip authority: `@concern:doorway-failover`.

**2b. The unexplained ignition, and the one thing this investigation did NOT close.** `/p2p/status`
is served by a `tokio::watch` borrow — zero I/O, no DB, no conductor — yet three independent
verifiers measured it at **12.018–12.071 s against BOTH matthew and adam** at different times. A
handler doing one lock-read cannot take 12 s unless the storage HTTP runtime could not *schedule*
it for 12 s. That runtime is `worker_threads(2)` (`elohim-storage/src/main.rs:270-275`), shared via
`server_rt.block_on(async_main)` with the libp2p swarm loop and everything it spawns, inside a
cgroup measured at 90–100 % of quota. A bump is the obvious lever and is **deliberately not taken
here**: this project's own conductor-admission work established that pinned occupancy is necessary
but not sufficient, and that oversubscription made throughput *worse* (`d(lambda)/d(capacity) <= 0`).
Measure before tuning — that is the whole point of `conductor-capacity-represented`.

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

## Mesh evidence 2026-08-21 19:38 — the breaker under concurrent suite load

On the fresh mesh (stock conductor, storage debug build with all of today's fixes), with the Act I inventory
+ several scoped runs + seven dry-runs in flight (load ~22), doorway A recorded FIVE `storage forward failed
(connect/timeout)` against matthew :8090 inside one ~60 s window (19:38:31–19:39) — the 2-failure threshold
opened the circuit, `/health` went `degraded`, saga ch01 ("peer alpha-A is healthy") failed, and the circuit
sat open/half-open for minutes while matthew answered `/db/content` in **1.6 ms direct** and 3–4 ms via the
doorway a minute later, at 0.32 cores, admission in-flight 0. No peer or conductor restarted (all pids from
19:25). So: a transient stall (the storage HTTP runtime is `worker_threads(2)` shared with the libp2p loop —
the "unclosed ignition" above) is amplified into minutes of shed by the breaker's sensitivity, and **storage
exports no request-duration histogram**, so the stall itself is invisible except through the doorway's
failures. Two bounded follow-ups: (1) a `elohim_http_request_duration_ms` histogram on storage (C8);
(2) breaker threshold/window as a function of observed RTT rather than a fixed streak of 2 — or at least a
minimum window (N failures within T seconds) so a one-second stall cannot open it. Measure on the mesh with
`just test mesh` under load before changing either.

## CORRECTION 2026-08-21 20:30 — the "storage stall" is the DOORWAY stalling on its first SSR render

The continuous mesh watch (15 s cadence, direct-vs-doorway latency) resolved the 19:38 and 20:22 windows:
every request THROUGH doorway A took exactly 12.00 s (the proxy timeout) while storage answered DIRECT in
1.6 ms; `/admin/self-healing` unreachable; doorway.log throughput per 10 s 169 → 23 → 15 → 3 → 2 → 1 right
after `SSR render trace` at 20:22:22 (first render of the lamad SPA after restart; `GET /apps/lamad/index.html`
entered at 20:22:1x). The breaker then opened on the doorway's OWN timeouts (`storage forward failed
(connect/timeout)` ×3 at 20:22:56), `/health` went degraded, saga ch01 red. Storage was never the stall —
the request-duration histogram (in flight) will show that directly; the doorway's SSR render executes on
the runtime that serves proxy/health/admin and parks it (~60 s cold). Cure direction: dedicated bounded
render pool + per-render budget that sheds to the bundle path, `doorway_ssr_render_duration_ms`, pre-warm
the first render off the request path. Doorway agent dispatched with this as item 5.

## Mesh evidence 2026-08-22 00:56 — warmup false-green + the shell-fetch half of the first-render stall

Local mesh regen probes (`### probes 00:56:19`, `/tmp/elohim-local-mesh/regen6.log`):

```
:8888 / 200 5.106252s
:8889 / 503 11.111940s
:8888 warmup.completed True heads None content 10
:8889 warmup.completed True heads None content 10
```

Both doorways' warmup HAD produced content (10 rows) — this is a DIFFERENT case from the pure
empty-projection false-green (item 1 below): `warmup.completed=True` here is an honest true, not a
false one. The stall is the FIRST `GET /` on each doorway; a coordinator re-probe confirmed the shape
directly: doorway B `GET /` 503'd at 11.1s with `servedBundleHeads` already populated (materialized
~40s earlier) and the breaker CLOSED; the immediate re-probe was 200 in 13.4s then 200 in 2.5ms.
Doorway A showed the same pattern (5.1s then ~3ms). This is a THIRD stacked cause under the same
"first `/` is slow" umbrella as the 2026-08-21 20:30 correction above (cold V8 render) and the
2026-08-21 warm-shell-dead-code fix (`bind_warm_shell_to_archive()`): the renderer (registry-materialized
at boot) and the projection (warmup-produced) were both warm, but `resolve_projected_shell`'s SEPARATE
Mongo-backed `warm_shell` store — the cache holding the app's `index.html` shell document composed
into the render — was cold for this app on this doorway, because `main.rs`'s boot-time
`warm_shell.hydrate()` only covers mounts the EPR router already knew about AT THAT INSTANT, and the
periodic EPR-router self-heal refresh (every `DOORWAY_EPR_REFRESH_SECS`, default 30s) never repeated
the hydrate/fetch for mounts that appear later. `decide_shell_serve(Cold, breaker-closed) = Fetch`, so
the first real request paid the upstream shell fetch cost itself (a real few-second cost, not a hang)
and rode past the 10s `EPR_DISPATCH_TIMEOUT_SECS` shed wall on doorway B.

**Fixed** (`doorway/doorway-service/src/server/http.rs`, `src/main.rs`,
`src/projection/warm_stream.rs`, `src/routes/health.rs`):

1. `WarmupState` gains `completed_empty`/`produced` — a pass that streamed nothing no longer reports
   `completed: true`; it re-warms every `WARMUP_REWARM_POLL_SECS` (30s) until a pass produces records.
2. `/health/startup`'s warmup block and the shed decision now read the SAME breaker map
   (`state.upstream_breakers`) — the warmup task's private `WarmStreamHealth` gate is no longer
   advertised as live upstream health.
3. New `SsrFallbackReason::WarmupEmpty`: `resolve_projected_shell` short-circuits to the CSR fallback
   immediately (no fetch attempted) when this doorway's own warmup is empty — closes the case where
   the WHOLE projection, not just the shell, has nothing to serve.
4. New `prewarm_projected_shells()`: resolves every EPR-mounted app's shell proactively, off the
   request path, called once after boot's `warm_shell.hydrate()` and again after every periodic
   EPR-router refresh tick — so a mount that appears late (or a Mongo archive that starts genuinely
   cold) gets pre-fetched before a real browser's first `/`, not on it.

**Not done here** (separate, larger follow-up, already tracked above as item 5 of the 20:30
correction): a dedicated bounded render pool / per-render budget for the ACTUAL V8 render execution.
Tonight's fix closes the shell-fetch half of "first `/` is slow"; the cold-render-parks-the-shared-runtime
half (2026-08-21 20:30 correction) is unchanged.

