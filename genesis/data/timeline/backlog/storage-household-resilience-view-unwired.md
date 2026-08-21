---
id: "backlog-storage-household-resilience-view-unwired"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "HouseholdResilienceView is a dead view type: schema + codegen + contract test exist, but no HTTP handler ever constructs it — the household resilience route serves ResilienceSnapshotView"
slug: "storage-household-resilience-view-unwired"
written: "2026-08-21"
author: "surface-census"
status: "backlog"
priority: "low"
severity: low
---

## What was measured

Closing the census gap "`onlinePeerCount` missing from `GET /api/v1/resilience/<id>/household`"
(commit 6ac384d71) showed the route is served by `handle_get_household_resilience`
(`elohim/elohim-storage/src/api/resilience.rs`) → `household_resilience::snapshot_with_staleness_secs`,
which returns a **`ResilienceSnapshotView`** (`details.onlinePeers.{live,known}` — the declared, honest
shape). The flat `onlinePeerCount` the scenario expected lives only on **`HouseholdResilienceView`**
(`elohim/sdk/schemas/v1/views/household-resilience-view.schema.json`,
`elohim-views/src/infrastructure.rs` ≈ L1390), and a grep shows that type is never constructed by any
handler — it exists for codegen and its own schema-contract test. A view that nothing serves is a
contract with no counterparty: the scenario author read it as the route's shape and drifted.

## Decision needed (one of)

- **Retire** `HouseholdResilienceView` (schema, Rust struct, contract test, generated TS) — if
  `ResilienceSnapshotView` is the household resilience wire shape for good; or
- **Wire** it: if a distinct household-level rollup is wanted, a handler must construct it and a
  scenario must sense it — until then the type is dead weight that misleads authors.

Either way, the census tool (`genesis/a2o/scripts/surface-census.ts`) should flag a view schema with no
constructing handler as its own class — it is exactly the "contract with no reader" the 2026-08-06
nomenclature review named.

## Cites

- genesis/a2o/features/resilience/resilience-dimensions.feature (D2 scenario, now reads `details.onlinePeers.live`)
- genesis/a2o/layering/surface-census.md
