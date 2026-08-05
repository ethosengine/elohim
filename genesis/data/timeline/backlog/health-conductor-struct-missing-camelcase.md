---
id: "backlog-health-conductor-struct-missing-camelcase"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway /health ConductorHealth struct lacks #[serde(rename_all=camelCase)] — conductor.* sub-fields wire as snake_case while sibling p2p/projection are camelCase"
slug: "health-conductor-struct-missing-camelcase"
written: "2026-06-29"
author: "dataplane validation-suite Task 2 review (surface-mapping surfaced the inconsistency)"
status: "backlog"
priority: "low"
jobs: [elohim]
---

## What
`ConductorHealth` in `doorway/doorway-service/src/routes/health.rs:~101` has `#[derive(Serialize)]` with **no** `#[serde(rename_all = "camelCase")]`, so `/health.conductor` sub-fields wire as `connected_workers`/`total_workers`/`pool_size`/`pools_healthy`/`pools_total` — while its sibling structs (`P2PHealth`, `ProjectionRole`) DO rename to camelCase. Inconsistent /health JSON casing.

## Coupling (do these together)
The dataplane validation suite's `HealthConductor` TS interface (`genesis/a2o/src/framework/dataplane/surfaces.ts`) was set to the REAL wire (snake_case) to match. **If you add `rename_all="camelCase"` to the Rust struct, flip the TS interface back to camelCase in the same change** (it's a live /health wire change — also check any other consumer of `/health.conductor.*`).

## Why low
`conductor.connected` (the only field the suite asserts today) is single-word, unaffected. This is a latent consistency cleanup, not a live break. Domain D8 (doorway projection).
