---
id: "backlog-elohim-app-ng-build-oom-cascade-blocks-dispatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim/dev ng-build OOMKill (recurring, no pod memory limit) cascade-aborts the orchestrator graph — edge/genesis never dispatch"
slug: "elohim-app-ng-build-oom-cascade-blocks-dispatch"
written: "2026-08-02"
author: "pipeline-landing shift (integrator)"
status: "backlog"
priority: "high"
tags: [ci, orchestrator, elohim-app, oom, pod-memory, cascade-abort, dispatch, infra]
cites:
  - Jenkinsfile
  - genesis/orchestrator/Jenkinsfile
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
---

# elohim/dev `ng build` OOMKill cascade-aborts the orchestrator graph

## Evidence (2026-08-02, ci-investigator, quoted from build logs)

Two identical signatures on `elohim/dev` (job display name "elohim-app"):
- **#1654** (2026-08-02T21:57Z, orchestrator #1602's wave): `pnpm exec ng build
  --configuration=alpha` → `Container [builder] terminated [OOMKilled]` →
  `- builder -- terminated (137)` → `AgentOfflineException … Pod failed because
  container terminated` → `InterruptedBuildAction: "Agent was removed"`.
- **#1652** (2026-08-01T00:29Z): byte-identical sequence, only the pod suffix differs.

2 of the last 6 `elohim/dev` builds. The builder pod spec declares **no memory
limit or request at all** — only ephemeral-storage (5Gi/2Gi) — so the OOM is
node-level. Hypothesis (inferred, NOT confirmed against Prometheus node metrics):
memory contention from 5 concurrent Level-0 builder pods on `node-type: edge`.

## The graph consequence (the part that bit this shift)

`elohim` is a **wait-for-result** Level-0 branch in the orchestrator's Execute
Builds parallel step. Its ABORTED result hard-failed orchestrator #1602 and
**skipped Levels 1–3: elohim-edge and elohim-genesis never dispatched** — for a
push whose critical path (dataplane deploy) had nothing to do with the app
build. One optional pipeline's infra abort blocked the whole graph. Recovery
required tag-forced retriggers.

## Fix directions (needs an owner; deliberately NOT hot-fixed mid-shift — the
orchestrator was load-bearing for the shift that found this)

1. **Pod provisioning**: add a memory request/limit to the app pipeline's
   builder pod template (root `Jenkinsfile` pod YAML) sized for `ng build
   --configuration=alpha`; a request alone would let the scheduler avoid the
   contended node.
2. **Graph resilience**: an ABORTED/infra-killed wait-for-result Level-0 branch
   should degrade the orchestrator to UNSTABLE and continue dispatching
   independent downstream levels, not hard-fail the graph — precedent: the
   cascade-deadlock cure (catchError→UNSTABLE) on the live-target E2E gate
   (memory: project_cascade_deadlock_live_target_gate). An app-image build is
   not a dependency of edge/genesis; the graph should encode that independence.
3. Optional: serialize the heaviest Level-0 builds or anti-affinity the builder
   pods if node contention is confirmed (verify first against node memory
   metrics around 2026-08-02T21:58Z and 2026-08-01T00:29Z).

Status: open, unowned.
