---
id: "backlog-household-cannot-stage-third-doorway-gamma"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The household mesh cannot stage a third doorway — story 3.1's scenario needs one asker and two holders, and the prologue's gamma slot declares itself absent"
slug: "household-cannot-stage-third-doorway-gamma"
written: "2026-09-20"
author: "serving-edge failover-balance-stream campaign, 2026-09-20 review"
status: "open"
priority: "medium"
jobs: [elohim, elohim-genesis]
cluster: "mesh-prologue-cast-and-env-gaps"
relatedNodeIds:
  - "habit:served-under-standing"
tags: [household-mesh, prologue, doorway, gamma, name-routing]
---

**The fact.** `genesis/a2o/features/federation/name-routing.feature` scenario 5 (`a07134562`, `@wip`) is
the acceptance scenario for `Weight` (story 3.1): one non-holder asking doorway relaying to two holders,
one of which declares a shed window that outlives a federation refresh. Staging it needs a third
household doorway. `app/elohim-app/scripts/hc-mesh-prologue.sh:580` declares a `gamma` slot whose only
content is `"absentReason": "Act I mesh stages two doorways (A/alpha and B/beta=apex); a scenario naming
a third must say which act it needs"`. `genesis/a2o/src/framework/fixtures/household-mesh.ts:200` iterates
`['alpha', 'beta', 'apex', 'gamma']` when applying doorway environment overrides, and `:356`
(`requireFixtureDoorwayUrl`) throws a named-absence error rather than silently returning nothing when a
requested doorway id has no URL.

**Evidence.** `app/elohim-app/scripts/hc-mesh-prologue.sh:580` (the `"gamma"` fixture entry, verbatim
`absentReason` string); `genesis/a2o/src/framework/fixtures/household-mesh.ts:200`
(`applyDoorwayEnvironment`'s doorway-id loop), `:356` (`requireFixtureDoorwayUrl`'s named-error path).
Design: `genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md` §9.4-9.5 — an exhaustive
2026-09-20 search of the a2o harness, `doorway-service` and `elohim-storage` found no way to make an
existing doorway declare a shed window longer than the 60s federation refresh cycle (real windows are
2s/20s/30s), which is a second, independent blocker on the same scenario.

**Why it matters.** Until gamma exists, scenario 5 stays `@wip`, and `Weight`'s cross-refresh persistence
(the story's whole content per §9.3 — `note_shed` already reorders a shedding holder for one cycle; the
new work is surviving the next `replace_all`) ships unit-proven only, with the habit ledger honestly `red`
on the household reading. This is one of two named prerequisites for closing that story on the household
mesh — the other (a dev-gated `PUT /admin/dev/shed`) is tracked separately.

**Smallest next step.** Stage a third doorway process in `hc-mesh.sh`: it needs no third conductor and no
third storage peer (a holder of a name is a projection contract, not a substrate role — it can share
beta's pool), just a port, membership, and the prologue filling `E2E_DOORWAY_GAMMA`. Note the RAM/process
cost of the extra doorway when sizing the household mesh.

**Links.** Story: `genesis/a2o/reports/recovery/serving-edge-20260919/story-3.1-design.md` §9.4-9.5. Plan:
`genesis/docs/superpowers/plans/2026-09-19-serving-edge-failover-balance-stream-campaign-plan.md` story
3.1. Habit: `doorway/doorway-service/.epr-meta/served-under-standing.habit.md`. Scenario:
`genesis/a2o/features/federation/name-routing.feature` scenario 5 (`a07134562`, `@wip`).

**2026-09-21 — landed, awaiting first household run.** `app/elohim-app/scripts/hc-mesh.sh` gained the
`gamma` doorway (`MESH_DOORWAY_GAMMA`, default on; `DOORWAY_C_PORT`/`DOORWAY_C_HEALTH_PORT`), riding
james's already-running conductor+storage — no third conductor, no third storage peer, confirmed by
`just mesh preflight` enumerating its ports as `free`. `hc-mesh-prologue.sh` fills the fixture's
`doorways.gamma` slot from a soft health check (or keeps the honest `absentReason` when gamma is off),
and exports `E2E_DOORWAY_GAMMA` only when reachable; `just test mesh` (justfile) mirrors that export.
The shed fixture (`PUT /admin/dev/shed`, `bd1446d88`) was already built and is unchanged here; the a2o
glue (`name-routing.steps.ts`) already called it correctly and now also names a `403 FIXTURE_ONLY`
refusal explicitly rather than folding it into a generic assertion. `household-mesh.ts` needed no
change — its `alpha|beta|apex|gamma` loop and absence-throwing were already generic and are covered by
the existing `__tests__/household-mesh.test.ts` gamma cases. Status stays `open`: nobody has yet run
the scenario against a live gamma doorway. Next: an operator runs the sequence in the session's final
report (source `environment.sh`, re-export `STORAGE_BIN`/`DOORWAY_BIN` to HEAD binaries, `just mesh
start`, `just mesh wait`, `just mesh prologue`, then the scoped `@wip` cucumber run) and records the
first result here.
