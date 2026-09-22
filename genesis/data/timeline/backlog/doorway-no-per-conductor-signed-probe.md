---
id: "backlog-doorway-no-per-conductor-signed-probe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "No doorway surface proves a SIGNED call through a named conductor's worker pool — pool health is a socket count, and no route lets a caller select the pool"
slug: "doorway-no-per-conductor-signed-probe"
written: "2026-09-22"
author: "serving-edge cell-membership cure, 2026-09-22 review round 4"
status: "backlog"
priority: "medium"
jobs: [elohim-edge]
tags: [doorway, conductor-pool, recovery, observability, test-oracle, open-question]
relatedNodeIds:
  - "backlog-doorway-worker-pool-optimistic-auth-storm"
  - "backlog-probe-conductor-diagnostics-doorway-404"
  - "backlog-blob-forward-confirmation-status-only-no-body-check"
  - "backlog-health-conductor-struct-missing-camelcase"
cites:
  - genesis/data/timeline/backlog/doorway-worker-pool-optimistic-auth-storm.md
  - genesis/data/timeline/backlog/blob-forward-confirmation-status-only-no-body-check.md
  - genesis/data/timeline/backlog/health-conductor-struct-missing-camelcase.md
  - doorway/doorway-service/src/worker/pool.rs
  - doorway/doorway-service/src/conductor/pool_map.rs
  - doorway/doorway-service/src/conductor/router.rs
  - doorway/doorway-service/src/routes/health.rs
  - genesis/a2o/features/doorway/peer-conductor-connection-resilience.feature
  - genesis/a2o/steps/mesh/peer-conductor-resilience.steps.ts
shift_objective: |
  Give the doorway one read-only surface that proves a SIGNED zome call through a
  NAMED conductor's worker pool returned, with failures propagated and the answering
  conductor/session attributed — so "the pool recovered after a conductor restart"
  can be asserted instead of inferred from a socket count.
---

**The fact.** The doorway publishes no evidence that a *signed* call through a *named* conductor's worker pool succeeded. Three separate limits compose into that gap:

1. `WorkerPool::is_healthy` is `self.connected_workers.load(...) > 0` (`doorway/doorway-service/src/worker/pool.rs:204`) — a socket count. `ConductorPoolMap::healthy_count` counts pools satisfying it (`doorway/doorway-service/src/conductor/pool_map.rs:225`), and `/health` reports `poolsHealthy` / `poolsTotal` from those (`doorway/doorway-service/src/routes/health.rs`). None of that establishes signing credentials, capability authorization, or a zome call that returned.
2. No HTTP route lets a caller target one conductor's pool. `ConductorRouter::route(agent_pub_key)` (`doorway/doorway-service/src/conductor/router.rs:58`) is the only agent→pool selector in the crate, and it is referenced by nothing but `/health`. Every route-level zome call goes through the process-wide singleton `ZomeCaller` pinned to the doorway's own primary conductor with the pool as ordered fallback, so even a success does not name which conductor answered.
3. The one pool-backed zome read — `GET /api/v1/cache/{type}/{id}` → `__doorway_get` — is projection-first and its 200 body carries no discriminator, so a 200 does not establish that a conductor was reached at all.

**Evidence.** Source tracing of the doorway crate on 2026-09-22 (read-only; no mesh run). `pool.rs:204` and `pool_map.rs:225` read as quoted. An exhaustive grep for `pool.request(` finds three consumers: the admin WS proxy, an app-WS proxy path that the HTTP surface does not use, and `cache/resolution.rs`'s conductor tier — all fed from the default pool. `state.conductor_router` is written once in `main.rs` and read once in `health.rs`. Every other authenticated read-only candidate either makes no zome call (`/auth/me`), silently degrades to a 200 on zome failure (`/auth/account`'s `hostedByHousehold`, `/api/v1/federation/doorways`), or is a mutation on the admin API (`POST /hc/connect`). The consequence was measured in the a2o layer, not inferred: the recovery scenario in `genesis/a2o/features/doorway/peer-conductor-connection-resilience.feature` could pass while a doorway's signed calls failed, because storage's own independently-authenticated `HcClient` answered the check.

**Why it matters.** The cost is a dishonest green, not an outage. A conductor restart invalidates every issued app-auth token; the whole point of the scenario is that the doorway re-mints one *and its pool works again*. A socket count cannot fail that assertion, so the scenario trained its readers to believe something it never measured. An endpoint or a scenario that cries wolf in the safe direction is worse than an absent one, because it is believed. This is the same lossy-measure shape the trust-contract runbook catalogues elsewhere.

**Smallest next step.** Two candidate shapes; the choice is deliberately deferred to whoever picks this up:

(a) **A read-only call through an explicitly selected pool.** Add one route that takes a conductor selector (or an agent key routed through `ConductorRouter::route`), issues a read-only zome call on that pool, propagates the failure status instead of degrading, and attributes the answering conductor id and session in the response body. That is the direct cure and it also gives the fleet a per-conductor liveness probe it does not have.

(b) **Per-conductor signed-probe evidence tied to the connection generation.** Have each pool record the outcome of the signed call it already makes when it authenticates, stamped with the connection generation, and expose that per conductor. Weaker than (a) — it is evidence about the last handshake, not about now — but it needs no new call path and would still refuse a pool whose signed calls fail.

Another connectivity boolean would not suffice; whatever lands must distinguish "sockets are up" from "a signed call returned".

**Neighbours that must be read with this one.** `doorway-worker-pool-optimistic-auth-storm` is the other half of the same sentence: it records that a worker logs `Authenticated` without reading an ack, while this one records that `is_healthy` is a socket count with no signed-call surface above it. Together they are *the pool reports healthy without proof*, and whichever cure lands should answer both. `blob-forward-confirmation-status-only-no-body-check` is the direct filing precedent for the shape (a status-only confirmation standing in for substantive proof) on a different surface, and `probe-conductor-diagnostics-doorway-404` is the same class of lying probe with a different mechanism. If cure (a) or (b) adds fields to `/health`'s `ConductorHealth`, it has to land with `health-conductor-struct-missing-camelcase` — same struct, same file, and a TS consumer in `genesis/a2o/src/framework/dataplane/surfaces.ts`.

**Why `priority: medium` and not `high`.** The dishonest green is already gone: the scenario now claims only the two things it establishes, and the missing proof is named by a `@wip` scenario that cannot pass silently. What remains is an absent capability rather than a live misleading signal, which is the difference between this and `resilience-unmeasured-vs-zero-honest-denominators`.

Until one exists, the resilience feature carries a `@wip` scenario naming the missing proof, and the passing scenario claims only the two things it establishes: that the storage peer's own conductor bridge answers a read-only zome call again, and that the doorway reports its worker pools reconnected.

**Links.**

- `doorway/doorway-service/src/worker/pool.rs:204` — `is_healthy` = `connected_workers > 0`
- `doorway/doorway-service/src/conductor/pool_map.rs:225` — `healthy_count` over that predicate
- `doorway/doorway-service/src/conductor/router.rs:58` — `route(agent_pub_key)`, unreferenced by routes
- `genesis/a2o/features/doorway/peer-conductor-connection-resilience.feature` — the corrected scenario and the `@wip` one naming the gap
- `genesis/a2o/steps/mesh/peer-conductor-resilience.steps.ts` — the two honest `Then` steps
- `genesis/data/timeline/backlog/doorway-worker-pool-optimistic-auth-storm.md` — the other half: `Authenticated` logged without an ack read
- `genesis/data/timeline/backlog/blob-forward-confirmation-status-only-no-body-check.md` — same shape, blob surface
- `genesis/data/timeline/backlog/health-conductor-struct-missing-camelcase.md` — same `/health` struct, if the cure touches it
