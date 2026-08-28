---
id: "backlog-a2o-reconnect-storm-needs-process-control-pending"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "a2o reconnect-storm scenario fails on every fleet run by design — it needs process control; make it pending, not failed, when the fixture has none"
slug: "a2o-reconnect-storm-needs-process-control-pending"
written: "2026-08-28"
author: "shift 2026-08-28T03-25-shakeout-landing-perf-trust-hybrid"
status: "proposed"
priority: "medium"
jobs: [elohim-genesis]
---

## The failure (persistent, byte-identical across genesis #1510 and #1511)

`features/doorway/peer-conductor-connection-resilience.feature:82` "Conductor fleet survives a doorway
reconnect storm" (`@requires:shem @act:iii`) fails with `E2E_SHEM_HOST not set — cannot resolve shem peer`.
Injecting the variable would NOT fix it: the second step (`steps/mesh/peer-conductor-resilience.steps.ts:1177`)
asserts `fixture.processControl === true` and its own message says a deployed multi-tenant fleet cannot be
severed by the harness — "the sever has to come from the operator (a fleet roll) with the run attached".
So on the fleet lane the scenario is unrunnable by design and counts as a FAILURE on every run, which
keeps the genesis measure red for a reason no repo change can cure.

## Proposal (test code — decided by the operator, not a shift)

1. In the `Given every conductor session is severed…` step, return `'pending'` (the a2o convention for
   "not exercisable here") when `mesh().processControl` is false, instead of asserting — the scenario
   then counts as pending, matching how `@act:i` scenarios are HELD on the fleet lane.
2. Optionally inject `E2E_SHEM_HOST=https://doorway-alpha.elohim.host` in `genesis/Jenkinsfile`'s
   `householdMeshEnv` entries (the step only needs a doorway whose `/health` reports `pools_total > 1`,
   which doorway-alpha does: pool_size 7) so the FIRST step is exercised on the fleet and only the sever
   goes pending.
3. The operator-attached fleet-roll run (a2o attached to an edge deploy) is the only place this scenario
   can go green — name that lane in the feature comment.
