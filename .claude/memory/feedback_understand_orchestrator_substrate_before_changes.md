---
name: Understand orchestrator substrate before changing build dispatch
description: Don't naively wrap parallel/stages — orchestrator dispatch has reliability invariants (level guards, baselines, cascade rules)
type: feedback
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
Before editing the orchestrator's Execute Builds stage or any dispatch logic, read all four pieces of the substrate:
- `genesis/orchestrator/orchestrator-strategy.mjs` (pure JS algorithm)
- `genesis/orchestrator/preview.mjs` (`just ci-preview`)
- `genesis/orchestrator/Jenkinsfile` `groupByDependencyLevel` and `triggerPipeline` (the Groovy that mirrors strategy.mjs)
- `orchestrator-strategy.test.mjs` (drift detector)

**Why:** The user pushed back when I proposed wrapping `parallel` calls in stages without first studying graph-walker / dependency-level computation. The orchestrator's reliability comes from invariants that are easy to break with naive edits:
- Level-failed guard (`levelFailed = level.findAll { !results[it]?.success }`) must abort downstream
- Per-pipeline baselines tracked in `pipeline-baselines.json` must be advanced only after success
- `cascades: false` on a pipeline (e.g. elohim-sophia, elohim-epr) means downstream auto-include doesn't apply
- Genesis is intentionally outside the levels loop — runs last after all builds succeed

**How to apply:**
- Presentation-only changes (nested `stage()` for Blue Ocean) are safe IF they don't move the `levelFailed` guard, the baselines update, or the genesis trigger out of their current control flow.
- Any edit to `orderByDependencies`, `groupByDependencyLevel`, `propagateDependencies`, or the PIPELINES map MUST be mirrored in `orchestrator-strategy.mjs` — `orchestrator-strategy.test.mjs` is the drift detector and will catch missed mirroring.
- Do not assume "parallel is fine" — the user has explicit reliability standards for build dispatch. Surface the invariants you intend to preserve before editing.
