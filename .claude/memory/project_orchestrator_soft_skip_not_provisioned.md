---
name: orchestrator-soft-skip-not-provisioned
description: "orchestrator's triggerPipeline now classifies \"No item named\" Jenkins errors as NOT_PROVISIONED soft-skip → UNSTABLE (not ERROR → FAILURE); lets strategy.mjs registration land ahead of Jenkins job provisioning without breaking delivery"
metadata: 
  node_type: memory
  type: project
  originSessionId: d5ebc70b-b1ff-43c0-9172-9d14847a28ec
---

`triggerPipeline` in `genesis/orchestrator/Jenkinsfile:681` (landed `09d39c2f4`, 2026-05-17) now classifies `hudson.AbortException` carrying `"No item named"` as a soft-skip:

```groovy
if (e.message?.contains('No item named')) {
    return [success: true, skipped: true, result: 'NOT_PROVISIONED', error: e.message]
}
```

The result-handler (`recordPipelineResult`, extracted above `pipeline {` to stay under the 11000-byte CPS method-size HARD limit) echoes `⏭️ <name>: SKIPPED — Jenkins job not provisioned (…)` and calls `unstable()`. The level-failed guard already treats `success: true` results as non-failures, so the cascade continues for whatever else is dispatched.

**Why:** Before this, registering a pipeline in `PIPELINES` / `orchestrator-strategy.mjs` AHEAD of provisioning its Jenkins multibranch job — a normal interim state during multi-step landings — caused the orchestrator to hard-FAIL the entire cascade at "Trigger Downstream" (build 963: `Error: No item named elohim-epr/dev found`). Other pipelines that were provisioned never ran. The shift's RCA confirmed this is a topology-incompleteness, not a delivery failure: UNSTABLE-with-loud-warning is the right semantic.

**How to apply:** When you see `result: 'NOT_PROVISIONED'` in BUILD_RESULTS or a `⏭️ SKIPPED — Jenkins job not provisioned` line in an orchestrator log, the missing job is operator-side. The strategy.mjs entry, Jenkinsfile, and build-manifest are all present; the Jenkins multibranch item just needs to be created. Until provisioned, the orchestrator stays UNSTABLE on every build whose changeset matches that pipeline's `changePatterns`. Validated end-to-end on orchestrator/dev #964: log line 646 has the SKIPPED message, line 648 fires unstable(), cascade ran elohim/storybook/edge to SUCCESS, orchestrator landed UNSTABLE (measure=1).

See also: [[doorway-single-target-no-fanout]] for how downstream dispatch ties to the doorway projection layer; [[orchestrator-predictive-vision]] for the broader strategy.mjs design.
