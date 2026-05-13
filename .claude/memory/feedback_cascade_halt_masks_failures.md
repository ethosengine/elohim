---
name: Cascade-halt orchestrator masks downstream failures
description: When a CI pipeline is red, the orchestrator's halt-on-failure cascade hides errors in every downstream pipeline; driving toward green surfaces buried failures one layer at a time
type: feedback
originSessionId: c423684a-b162-42c6-b5cf-177683da9ed0
---
When the Elohim orchestrator (`genesis/orchestrator/Jenkinsfile`) sees an upstream pipeline FAIL, it aborts the cascade — downstream pipelines are never attempted. This means: while a critical pipeline (holochain, edge, etc.) is red, every other pipeline's potential failures are invisible. They could be broken for days and you wouldn't know.

**Why:** Shift `2026-05-03T18-19-orchestrator-805-pipelines-unstable` drove holochain back to green (4 sweettest fixes + Jenkinsfile quarantines), then immediately surfaced TWO previously-hidden failures: (1) a storage compile error from commit `333fa635` 2 days prior (Dockerfile missing COPY for `elohim/sdk`); (2) a K8s deploy timeout when the new image rolled out. Both had been latent since 2026-05-01 but invisible because edge never ran while holochain was red.

**How to apply:** When taking on a shift to drive a CI cascade green, expect to discover downstream issues in waves rather than all at once. Budget for at least one extra iteration per cascade layer below the immediate failure. Resist the temptation to declare done as soon as the immediately-failing stage clears — fresh-trigger verification is essential because each new layer's failure mode is invisible until you can run it.
