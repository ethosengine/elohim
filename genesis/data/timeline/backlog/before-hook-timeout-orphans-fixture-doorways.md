---
id: "backlog-before-hook-timeout-orphans-fixture-doorways"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A timed-out deliverability Before hook orphans the fixture's mongod and doorways — cleanup registers only after an await it may never return from"
slug: "before-hook-timeout-orphans-fixture-doorways"
written: "2026-09-21"
author: "serving-edge household-acceptance campaign, 2026-09-20/21 review"
status: "open"
priority: "medium"
jobs: [elohim-genesis]
cluster: "mesh-prologue-cast-and-env-gaps"
relatedNodeIds:
  - "habit:served-under-standing"
tags: [a2o, harness, fixture-leak, mesh, doorway, cucumber-hook]
---

**The fact.** `genesis/a2o/steps/dataplane/epr-app-deliverability.steps.ts:143`'s `Before` hook boots
the scenario's own doorway pair via `OwnedDoorwayPair.start` (`genesis/a2o/src/framework/fixtures/owned-doorway-pair.ts:300`)
and registers cleanup (`this.onCleanup(...)`, `:156`) only on the line immediately AFTER
`const pair = await startOwnedDoorwayPair(...)` (`:150`) resolves. On 2026-09-20 nine processes
(3 mongod + 6 doorway, ~1.15 GB RSS combined) from three `epr-deliverability-doorways-*` scenario
roots (`AXOTKC`, `xmOR2r`, `j8GoM8`) outlived their cucumber runs and kept writing into the app
dist. Commit `4dce6642f` (2026-09-20) raised the hook's own timeout from cucumber's 31 s default to
an explicit 180 s — necessary because the boot it was timing out on was itself pathological (see the
server-bundle-feedback-loop finding) — but it does not touch the ordering that causes the leak.

**Evidence.** `J1-orphan-fixture-cleanup.log` (nine PIDs, RSS 1,182,428 KB total, ages 2320–5328 s at
`mesh stop`). `owned-doorway-pair.ts:300-435` (`OwnedDoorwayPair.start`): its own `acquired`/catch
block (`:308`, `:416-434`) fires only when `start()` itself throws — an assertion failure, a spawn
error, a `waitFor` rejection surfaced *to* `start()`. It has no interaction with an external caller
abandoning the await. `epr-app-deliverability.steps.ts:150,156`: the cleanup registration is the
statement immediately after the await, not before it.

**Why it matters.** A `Before`-hook timeout is cucumber's own bookkeeping marking the hook failed and
moving on — it does not, and cannot, cancel the in-flight `start()` promise (Node has no promise
cancellation). If `start()` is still running when cucumber gives up, the scenario's `After` hooks
(which call `world.runCleanup()` over whatever is in `cleanupCallbacks` *at that moment*) run before
the abandoned promise ever reaches the `onCleanup(...)` line — so the callback registers, if at all,
into a list nothing will drain again this scenario. Any doorway pair that finishes booting (fully or
partially) after the timeout fires is orphaned by construction, not by bad luck.

**Smallest next step.** Register cleanup before the await that can outlive the hook — reserve a
mutable cleanup slot on `this` (or close over a not-yet-assigned `pair` variable) before calling
`startOwnedDoorwayPair`, so a late-resolving promise still has a live cleanup hook to feed.
Complementary: have the fixture sweep its own process group or a pidfile at the next `start()`, the
way `hc-mesh.sh` already detects an orphaned-live conductor (`task-mesh-orphaned-conductor-guard.md`),
so a leaked prior run's mongod/doorway triplet is reaped on the next scenario boot regardless of hook
timing.

**Links.** Evidence: `genesis/a2o/reports/recovery/serving-edge-20260920/J1-orphan-fixture-cleanup.log`.
Commit: `4dce6642f` (raises the Before-hook budget; does not close this leak). Habit:
`doorway/doorway-service/.epr-meta/served-under-standing.habit.md` (this leg gates the pre-push
serving receipt). Cluster: `genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md`.
