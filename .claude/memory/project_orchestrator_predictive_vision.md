---
name: Orchestrator predictive build-graph vision
description: Long-term vision for orchestrator — predict before push, reconcile after run, every disconnect = investigation
type: project
originSessionId: 91882765-aece-476c-a49a-85b618774d32
---
The user's stated vision for the elohim-orchestrator pipeline: **a real build-graph pipeline that helps visualize how each run flows, gives great visibility to investigators (human + AI), and lets us know before we push what we expect to run, then reconciles against what actually ran. Every disconnect elevates to an investigation, iterating toward predictive compiler-validated builds every run.**

**Why:** Three-hour build runs hide cascading failures (DNA cold-compile, conductor outage, Jenkins controller restart) inside a single opaque "Execute Builds" stage. Without visible structure, drift between expected and actual goes unnoticed; investigations are reactive instead of preventive.

**How to apply:**
- Existing substrate already supports this — don't rebuild it:
  - `genesis/orchestrator/orchestrator-strategy.mjs` is a pure-function port of the Jenkinsfile's decision logic (`simulate`, `analyzePipelineRequirements`, `propagateDependencies`, `orderByDependencies`).
  - `genesis/orchestrator/preview.mjs` (`just ci-preview`) runs `simulate()` locally pre-push and prints predicted dispatch.
  - `genesis/orchestrator/orchestrator-strategy.test.mjs` is the drift detector between Groovy and JS — keep it green.
  - `groupByDependencyLevel` in Jenkinsfile:478 computes execution levels.
- Iterations toward the vision:
  1. **Visibility**: nest stages inside Execute Builds so Blue Ocean shows the level structure with per-pipeline timing. Pure presentation, no behaviour change.
  2. **Reconciliation artifact**: emit `predicted-build-graph.json` (from `simulate()`) and `actual-build-graph.json` (from Execute Builds results), then a Reconcile stage that diffs them.
  3. **Drift escalation**: any predicted-vs-actual disconnect produces a UNSTABLE result with an investigation pointer.
- The user accepts solid incremental iterations — don't over-build, but every iteration should leave the substrate cleaner toward the predictive goal.
- Distinguish two graph-walkers in this repo: `graph-walker.mjs` is per-pipeline manifest-step gating (lint/test); `groupByDependencyLevel` in the orchestrator Jenkinsfile is for orchestrator-level dispatch ordering. Different layer, different concern.
