---
id: "backlog-ci-orchestrator-stale-baseline-supersedes-inflight-downstream"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A stale-baseline dispatch re-runs a downstream pipeline that is still building the same commit, aborting it mid-delivery"
slug: "ci-orchestrator-stale-baseline-supersedes-inflight-downstream"
written: "2026-09-23"
author: "shift 2026-09-23T04-10-land-serving-edge-batch"
status: "open"
priority: "high"
jobs: [elohim-orchestrator, elohim]
cluster: "ci-orchestrator-backlog"
relatedNodeIds:
  - "habit:doorway-failover"
tags: [ci, orchestrator, dispatch, baseline, timeout, wall-clock]
---

**The fact.** Orchestrator #1894 (push c919bc8f1) ran DNA #1451 (44 min) then edge #1477
(~3 h), dispatched app `elohim` #1722 at 07:43, and hit its shared 240-minute
`timeout()` at 07:52 (`Execute Builds` FAILED at 239 m). #1722 was not cancelled and kept
running. Because #1894 failed before recording app's green baseline, the next timer run
(#1895, "Started by timer", 09:00, no changeset) found app's baseline stale and dispatched
app #1723 for the same commit — which superseded #1722 while it was 79 minutes into
`Publish and Verify App Delivery`. A delivery that was about to finish was thrown away and
restarted from zero.

**Why it matters.** Every roll-bearing push now costs two app runs, and the first is
always wasted: the shared budget ends the orchestrator mid-app, and the next dispatch
kills the survivor. It compounds the wall-clock problem the per-pipeline-timeout fix
addresses (orchestrator `Jenkinsfile` `timeout(time: 240, unit: 'MINUTES')`).

**Smallest next step.** Before a stale-baseline dispatch, ask Jenkins whether that
downstream job already has a build running for the same commit (its `lastBuild` is
`building` and its SCM revision matches); if so, wait on or adopt that build instead of
starting a new one. Pairs with giving each downstream pipeline its own timeout so the
orchestrator never abandons a downstream it dispatched.

**Evidence.** Jenkins: elohim-orchestrator/dev #1894 (FAILURE, 244 m, Execute Builds 239 m),
#1895 (TimerTrigger, empty changeset); elohim/dev #1722 (ABORTED after 79 m in Publish
and Verify App Delivery), #1723 (started 09:01). Shift journal
`.claude/shifts/2026-09-23T04-10-land-serving-edge-batch.journal.md` iterations 7–9.
