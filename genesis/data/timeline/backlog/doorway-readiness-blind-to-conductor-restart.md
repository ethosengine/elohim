---
id: "backlog-doorway-readiness-blind-to-conductor-restart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway readiness is blind to a conductor restart"
slug: "doorway-readiness-blind-to-conductor-restart"
written: "2026-11-08"
author: "claude (re-genesis 1 overnight shift, operator-directed)"
status: "refined"
priority: "high"
relatedNodeIds:
  - "genesis/a2o/features/dataplane/app-delivery-refuses-fast.feature"
  - "genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/regression-classes.md"
tags: [doorway, readiness, health, conductor, boot-snapshot, push-delivers-within-budget]
shift_objective: |
  Make doorway /health/serving report a conductor that is restarting or gone as not ready,
  so the fleet write-readiness probe sees the window a conductor restart opens. Today the
  conductor-presence test is `rolesDiscovered = state.zome_configs.len()`
  (doorway/doorway-service/src/routes/health.rs ~596), a count filled at first discovery that a
  conductor restart does not empty. Done when station 2 of
  genesis/a2o/features/dataplane/app-delivery-refuses-fast.feature ("restarting every conductor
  opens a window the probe names, and the window closes on its own") passes on the household,
  with the probe answering cell-not-running or catching-up at least once during the restart.
---

# Doorway readiness is blind to a conductor restart

## What happens

The fleet write-readiness probe (`scripts/ci/fleet-write-readiness.sh`) asks each doorway's
`/health/serving` whether it can take a write. The doorway's answer counts the conductor as
present when `rolesDiscovered > 0`, and `rolesDiscovered` is the size of `state.zome_configs`, the
role map discovery fills when the doorway first reaches its conductor. Nothing visible empties that
map when the conductor restarts. So once a doorway has discovered its roles, `/health/serving` keeps
answering ready while its conductor is down and coming back.

## How it was found

Re-genesis 1, overnight 2026-11-08. The late-bind storage fix needed a household serving receipt
from `app-delivery-refuses-fast.feature`. Stations 1, 3 and 4 passed twice; station 2 failed both
times identically: the sampler asked every 3 s and got 3 answers, all ready, across a restart the
household finished in about 10 s. Shem pointed out that a 10 s window sampled every 3 s should yield
at least two not-ready answers if the probe looked at the conductor at all; the source confirms the
count above. On alpha after matthew's storage restart, `/health/serving` reported `rolesDiscovered`
5 as before.

The story is right and the signal is wrong. It is the third instance this sprint of one class —
a dependency's state captured once and treated as live — after storage's release adoption and
binding mint (fixed by lazy acquisition through the client registry). See
`regression-classes.md` class 7.

## Why it matters

A deploy that consults the probe during a conductor restart is told the fleet is ready and writes
into a window where it is not, which is exactly what the probe exists to prevent. The station-2
receipt stays red until this is fixed, so any change to a refuses-fast serving path has to be
admitted by an operator bypass.

## Readiness notes

- **Shape of the fix:** derive conductor presence from live state rather than a discovery-time
  count — the worker pool's connection state, or the app-socket liveness the doorway already
  tracks for `/health` (`conductor_is_connected(workers_live, roles_discovered)` in the same file
  takes `workers_live`; `/health/serving` does not consult it). Keep a read replica
  (`--projection-writer=false`) reporting "not observed here".
- **Proof:** station 2 on the household (`just test mesh features/dataplane/app-delivery-refuses-fast.feature`),
  then a doorway unit test that a dropped worker pool flips `/health/serving` to not ready.
- **Area knowledge:** `doorway/doorway-service/src/routes/health.rs` (`serving_check`, ~913-928;
  the `conductor_blind` branch was added 2026-08-21 for the boot-before-conductor case and covers
  only the empty map).
