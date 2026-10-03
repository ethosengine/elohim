---
id: "backlog-quiesce-gate-measurement-availability"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fleet-quiesce gate DID-NOT-MEASURE x3 in one evening — A-side oscillation is measurement-availability debt"
slug: "quiesce-gate-measurement-availability"
written: "2026-08-19"
author: "claude (shift 2026-08-19T03-37-operator-positive-path-green)"
status: "backlog"
priority: "high"
cites:
  - scripts/ci/run-dataplane-validation.sh
  - elohim/elohim-storage/src/reconcile/controller.rs
relatedNodeIds:
  - backlog-bounds-validator-lexicographic-timestamp-compare
tags: [ci, measurement, quiesce, reconcile, oscillation, dataplane, performance, perf-telemetry, perf-convergence, friction-blind, friction-wait, phase-transition, lane-operator]
shift_objective: |
  Make fleet measurement available while A-side reconcile oscillates. Evidence
  (2026-08-19 evening): edge #1367, #1368, #1369 (validate-only) ALL ended
  DID-NOT-MEASURE — the fleet-quiesce gate (deadline 2700s, sustain 330s,
  tol=2) never held a sustained window because storage-A's projectionReconcile
  plateaued (~1000-1769 divergentAnchor, pending ~2400-2800, healedTotal
  +23/45min, caughtUp flapping true/false, actionable oscillating 0-1 with
  count bursts to 9-14). #1366 (deploy run) caught a quiet window and measured
  (that is how operator-runtime-surface went green), but three consecutive
  validate-only attempts could not — so the entire byConcern measurement
  surface (all habits + sagas) was UNMEASURABLE for an evening. Candidates:
  (a) treat the plateau as the real bug — the reconcile sweep re-derives the
  same ~1000 divergent anchors each pass without converging (relate to
  saga-06-heads-converge and reconcile-inventory reds, and the
  ghost-declaration class); (b) gate policy — a sustained-window predicate
  that tolerates a bounded steady-state oscillation (actionable<=tol
  time-weighted, not instant), or a separate measure-under-churn mode that
  runs the suite and LABELS the churn state instead of refusing to measure.
  Fix (a) is the substrate cure; (b) is measurement honesty either way.
---

# Fleet-quiesce gate: three DID-NOT-MEASURE runs in one evening

See `shift_objective` for the full evidence trail. The one-line shape: the
gate's instant predicate + 330s sustain never coincided with the A-side
reconcile plateau's quiet windows during validate-only runs #1367-#1369
(2026-08-19 16:00-19:10Z), while deploy-run #1366 measured minutes after its
restart. Measurement availability should not depend on winning a race against
a non-converging sweep.

**RECONCILED 2026-10-02** (shem, fork 2b334df7973d, superproject 4a80267f3, code read only, nothing measured): PARTLY cured. A tolerance leg is in code: validation exports QUIESCE_ACTIONABLE_TOLERANCE=2 (scripts/ci/run-dataplane-validation.sh:85-86; gate :32-47,332-343). No measure-under-churn mode exists: deadline still ends DID NOT MEASURE (run-dataplane-validation.sh:97-100). The plateau substrate cause is not determinable from this code. Same mechanism as: ci-edge-inventory-convergence-caughtup-sawtooth-flap. Confirming measurement: fleet only; locally, seconds to sustained PASS from run-mesh-quiesce-stage.sh.
