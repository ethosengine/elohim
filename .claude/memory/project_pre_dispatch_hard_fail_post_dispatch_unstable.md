---
name: pre-dispatch-hard-fail-post-dispatch-unstable
description: "Jenkins orchestrator pipeline pattern — stages BEFORE dispatch may hard-fail; stages AFTER dispatch (observational/reporting) must use catchError → UNSTABLE so they don't blank the downstream truth"
metadata: 
  node_type: memory
  type: project
  originSessionId: 4b9a03ba-e6e3-4770-9ee2-00e1721d108f
---

In `genesis/orchestrator/Jenkinsfile`, two failure regimes live side-by-side and the discipline-line between them is load-bearing for pipeline-quality:

**Pre-dispatch stages CAN hard-fail.** Examples: `Post Predicted Build Graph` (runs after Determine Build Plan but before Execute Builds). The stage's comment says it explicitly: "Failing here is loud: a red stage in Blue Ocean before Execute Builds runs." If predicted-build-graph.json can't be archived, the orchestrator's setup is broken and downstream dispatch shouldn't proceed.

**Post-dispatch stages MUST NOT hard-fail.** Examples: `Post Actual Build Graph`, `Verify Deployment`, `Post-flight Health Check`, `Reconcile Build Graph`. By the time these run, the dispatched downstream pipelines have already completed — the world is what it is. Hard-failing because of a parse/archive/HTTP hiccup in the reporting layer blanks the truth those stages were supposed to surface (drift detection, deployment-version verification, health probes).

**The pattern:** wrap post-dispatch stage script bodies in `catchError(buildResult: 'UNSTABLE', stageResult: 'UNSTABLE')`. The existing `P2P Validation (Advisory)` stage demonstrates the canonical shape. Internal `unstable()` calls (policy decisions: "any unhealthy → UNSTABLE") stay; catchError is the exception fallback.

**Why this matters for the agentic-developer loop:** A FAILURE'd orchestrator hides the downstream truth from ci-observer dispatches, which forces the iteration loop to retry blind. UNSTABLE preserves the truth + still surfaces the signal that something was off.

**Historical instance:** orchestrator build #970 (2026-05-18) — Post Actual Build Graph NPE'd dereferencing `actual.results[name]` when `actual.results` was null (Execute Builds wrote the file after an aborted-pre-start dispatch with executionOrder populated but results map empty). The whole orchestrator dropped to FAILURE; downstream truth was preserved only by separately querying each dispatched job. Fixed in commit `1bcafb01e` by adding `?: [:]` null-coalesce on `actual.results` AND wrapping in the catchError pattern.

Related: [[feedback_cascade_halt_masks_failures]] (same principle one rung up — green-driving up the call stack masks downstream truth).
