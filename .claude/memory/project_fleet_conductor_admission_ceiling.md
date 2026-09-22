---
index: false
name: project-fleet-conductor-admission-ceiling
title: "Fleet ceiling — pool conductors shed interactive writes"
id: project-fleet-conductor-admission-ceiling
description: "On alpha the pool conductors (matthew/adam) shed interactive content_store writes and time out heal reads; the quiesce gate and the app head PATCH both fail on that, not on storage code"
metadata: 
  node_type: memory
  title: Fleet ceiling — pool conductors shed interactive writes
  type: project
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T09:55:28.769Z
cites:
  - elohim/elohim-storage/src/conductor_admission.rs
  - scripts/ci/fleet-quiesce-gate.sh
---

Read on 2026-09-13 after rolling the storage cures (shard serialization, lifecycle-ordered classifier, heal guard) to the fleet: the quiesce gate's `divergent_actionable` on storage A plateaus at 14–25 because the heal leg's reads of matthew-alpha's conductor time out (three attempts per sweep before the HealCircuit opens), and the app pipeline's notarized head PATCH on both alpha and apex is answered `{"status":"catching-up","retryAfter":30,"cause":"upstream"}` because `elohim_conductor_admission_shed_total{class="interactive",zome="content_store"}` rises on adam and matthew. `state_divergent` did fall (matthew 40 → 12), so the classifier is not the residual.

**Why:** the pool conductors host the cast (hosted humans) and carry the known arc/CPU load; both CI verdicts are honest readings of that load, and no storage-side change hides it.

**How to apply:** when the quiesce gate or the app head declaration reds after a roll, read `elohim_conductor_admission_shed_total` and `elohim_projection_heal_outcomes_total{outcome="timeout_exhausted"}` on the pool pods before touching reconcile code; the operator-owned levers are pool-conductor capacity (or hosted-cast size) and the gate's window; the pipeline-side lever is honouring the doorway's `retryAfter` in `scripts/ci/stage-spa-blob.sh` (fixed 2026-09-13). Related: [[project_conductor_arc_resources]], [[project_pipeline_dispatch_ordering]].
