---
name: project_cascade_deadlock_live_target_gate
description: A live-target E2E gate on the only waited-on Level-0 pipeline deadlocks the edge deploy that fixes that target; fixed via catchError→UNSTABLE.
metadata: 
  node_type: memory
  type: project
  originSessionId: c6f7bba6-ed57-449a-a6d9-29bc56445abe
---

The orchestrator cascade deadlocked on 2026-06-13 (orchestrator #1240): a down/flapping alpha blocked the deploy that would FIX alpha. Mechanism — `elohim-app` is the ONLY non-`longRunning` (waited-on) Level-0 pipeline; its `E2E Testing - Alpha Validation` stage opens with `timeout 60s curl alpha.elohim.host`, so a down alpha → app build FAILURE → orchestrator level-fail abort (`error "Build(s) failed: elohim - Aborting"`, Execute Builds ~1807) → `elohim-edge` (Level 1+, the ONLY `kubectl apply` pipeline) **never dispatched** → self-healing/arc code never reached the cluster. The DNA/holochain pipeline has no deploy stage; edge deploys. `longRunning` pipelines (storybook, holochain) dispatch fire-and-forget so their failures are invisible to the abort — only the waited-on app could trip it.

**Fixed 2026-06-14 (51d16c4d4, feat/frontend-eyes-sprint):** wrap `runE2ETests('alpha')` in `catchError(buildResult:'UNSTABLE')`. `triggerPipeline` treats UNSTABLE as success (`result in [SUCCESS, UNSTABLE]`) → no abort → edge deploys. App build/compile/Sonar still hard-gate.

Gate-class facts: edge deploy = `kubectl apply`, never gates on target health (it's the fix carrier). Genesis `Verify Target Health` (`timeout 120s curl`, exit 124, genesis/Jenkinsfile:1435) is a DISTINCT downstream gate — legitimately gates seeding, runs after edge, left as-is (killed genesis #1141/#1142). Orchestrator's own post-flight/P2P/fed-smoke gates are already `catchError→UNSTABLE`. Full record: `.claude/data/doorway-freeze-incident-2026-06-13/CASCADE-DEADLOCK-AND-FIX.md`. Related: [[project_sprint_branch_not_orchestrator_indexed]], [[feedback_concurrent_push_mutual_abort]], [[project_self_healing_control_plane_vision]].
