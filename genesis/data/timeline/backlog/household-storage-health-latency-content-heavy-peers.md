---
id: "backlog-household-storage-health-latency-content-heavy-peers"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "On the household, matthew's and jessica's storage /health answers in ~1.2 s at steady state (james 0.1 s) and past 2 s right after a restart, so a step that bounds one health read at 2 s after a scripted restart reads the latency as 'peer did not come back'"
slug: "household-storage-health-latency-content-heavy-peers"
written: "2026-10-09"
author: "claude-fable-5-1 (shift 2026-10-09T06-30-land-gradient-sprint-on-dev)"
status: "open"
priority: "low"
tags: [household, elohim-storage, health, latency, a2o, epr-app-deliverability, serving-receipt, flake-class, idle-is-free]
relatedNodeIds:
  - habit:idle-is-free
cites:
  - genesis/a2o/features/dataplane/epr-app-deliverability.feature
  - genesis/a2o/steps/dataplane/epr-app-deliverability.steps.ts
  - genesis/a2o/src/framework/dataplane/surfaces.ts
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# Household storage /health is slow on the two content-heavy peers

## Measured (2026-10-09, Dowell household, storage from main 1291fc0d3, conductor hc-fork-c916eddbed02)

| peer | `/health` steady (3 reads) | right after `storage-restart` |
|---|---|---|
| matthew :8090 | 1.31 s, 1.14 s, 1.12 s (12:30Z); 1.24 s (12:35Z); 1.10 s (12:56Z) | not timed |
| jessica :8091 | 1.33 s, 1.38 s, 2.07 s (12:30Z); 1.16 s (12:35Z); 1.12 s (12:56Z) | > 2 s twice (receipt runs 12:14Z and 12:25Z: `RequestBoundExceeded: GET http://localhost:8091/health — no response within its 2000ms bound`) |
| james :8092 | 0.19 s, 0.10 s, 0.14 s; 0.09 s; 0.10 s | — |

The latency did not follow conductor CPU: the three conductors were at 200–283 % each at 12:30Z and at 40 % after `just mesh conductors-restart` at 12:33Z, and matthew's and jessica's `/health` stayed at ~1.2 s either way. James, with the smallest content store, answers in a tenth of that.

## Where it bites

`epr-app-deliverability.feature` "a doorway that restarts while its peer is down catches up on its own" → step `peer "jessica" comes back` (`epr-app-deliverability.steps.ts` ~1329): after the maintained arm's `storage-restart` returns ready, the step takes ONE `getRaw(.../health, { timeoutMs: 2000 })`. On jessica that read sits inside the latency's noise, so the serving receipt the pre-push requires reads 4/5 and the push is refused. Four runs on 2026-10-09: 11:48Z pass, 12:14Z and 12:25Z fail on this step, 12:39Z fail on post-recycle convergence instead, 13:03Z pass once the household had settled.

## The question (not an answer)

What does `/health` do on a content-heavy peer that costs a second — a conductor round-trip, a content-table count, a blob-store stat? If it is work on the health path, that is an idle-is-free concern (a liveness probe should not price the store); if it is a conductor round-trip, the probe should report the conductor leg separately and answer the HTTP leg at once. Do not widen the step's bound to cover it; the bound is the story's claim.
