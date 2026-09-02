---
id: "backlog-ci-orchestrator-supersede-aborts-in-flight-edge-rolls"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A new orchestrator run aborts the previous wave's in-flight elohim-edge build mid-rollout — two consecutive edge rolls (#1412, #1413) died ABORTED after their storage pods had already restarted, so the fleet ran a half-verified wave and the next roll was gated another hour behind the DNA build"
slug: "ci-orchestrator-supersede-aborts-in-flight-edge-rolls"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch"
status: "open"
priority: "medium"
ci_status: open
jobs: [elohim-orchestrator, elohim-edge]
relatedNodeIds: []
tags: [ci, orchestrator, supersede, abort, elohim-edge, rollout, fleet-churn, not-built-is-lossy]
cites:
  - genesis/orchestrator/Jenkinsfile
  - elohim/holochain/Jenkinsfile
  - genesis/docs/content/elohim-protocol/history/2026-06-02-ci-orchestrator-recurring-anti-patterns-museum.md
  - genesis/data/timeline/backlog/ci-edge-depends-on-holochain-rebuilds-dna-without-dna-change.md
---

## Measured

2026-09-02: `elohim-edge/dev` #1412 (started 06:48Z from orchestrator #1784) had restarted all
seven `elohim-<human>-alpha-0` pods by ~07:40Z (Prometheus `kube_pod_start_time`) and was still
running its validation legs when orchestrator #1785 (wave 4, pushed 07:5xZ) superseded it:
`result: ABORTED` while `building: true`. #1413 (from #1785, started 09:09Z) was aborted the
same way by #1787 (wave 5, 09:40Z). Neither build's console could be read through the Jenkins
MCP (timeouts on large logs), so the abort-cause line is unquoted; the pod ages and the
building/ABORTED pair are the evidence.

The museum already records "superseded ≠ regression" as a *reading* trap. This is the *cost*
side: an abort after the storage roll but before the dataplane validation leaves the fleet on
the new image with no `✓ canonical head propagated` measure, and the App deploy that ran into
the roll (#1684) left `alpha.elohim.host` serving an index whose hashed assets 404.

## The change

Supersede should not abort an edge build past its point of no return (image pushed, rollout
begun): let it finish validation, and queue the newer plan behind it — or abort only builds
still in build stages. A `[edge:validate-only]` re-measure of the live fleet is the cheap
recovery when a roll was cut.

## Done when

A wave pushed while the prior wave's edge build is in its rollout/validation stages produces
one completed edge build with its validation legs, then the newer one — no ABORTED-after-roll.
