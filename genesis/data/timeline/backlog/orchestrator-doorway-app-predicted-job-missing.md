---
title: "Orchestrator predicts elohim-doorway-app pipeline but no such Jenkins job exists"
created: 2026-06-04
domain: "ci"
tags: [orchestrator, graph-walker, dispatch-drift, jenkins, ci-domain]
shift_objective: |
  predicted-build-graph.json for orchestrator/dev#1153 listed elohim-doorway-app in the
  Level-1 parallel set, but Jenkins has no elohim-doorway-app job (observer: "job does not
  exist"). Either a build-manifest.json declares a pipeline that was never stood up, or the
  job name drifted. Dispatch verdict was 'mixed' (under-built by one). Reconcile: stand up
  the job, or correct the manifest/graph-walker mapping so prediction matches dispatchable
  reality. Done when validate-mode dispatch_drift returns 'expected' on a doorway-app-touching
  push.
---

Discovered during shift 2026-06-04T14-52-post-merge-shakeout-e2e-greenup iteration 1
(principle-7 change-detection validation). Out of scope for the shift (orchestrator config
not in objective.scope.paths).
