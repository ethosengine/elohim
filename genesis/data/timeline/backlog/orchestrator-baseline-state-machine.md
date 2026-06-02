---
id: "backlog-orchestrator-baseline-state-machine"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Baseline advances only on confirmed downstream success — kill phantom-success-on-FAILURE and the lossy FAILURE-count grep"
slug: "orchestrator-baseline-state-machine"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI/orchestrator"
recurrence: 2
source_shifts:
  - "2026-05-16"
  - "2026-05-24"
domain: "code"
relatedNodeIds:
  - "memory:project_pre_dispatch_hard_fail_post_dispatch_unstable"
  - "memory:feedback_cascade_halt_masks_failures"
  - "memory:project_orchestrator_predictive_vision"
tags: [ci, orchestrator, baseline, state-machine, code-domain, recurring]
shift_objective: |
  The orchestrator's per-pipeline baseline advances too eagerly. When a child FAILs or ABORTs
  during dispatch, the baseline-tracking logic still records a phantom success and advances —
  and the failure-detection itself is a lossy `grep`-for-FAILURE-count over the result stream
  (NOT_BUILT / ABORTED / UNSTABLE all collapse to 0, so a real failure can be undercounted).
  When the per-pipeline baseline is then invalidated, it falls back to a global baseline →
  full cascade rebuild, and `lastSuccessful()` can pin an ancient green (observed 2026-05-16,
  05-24).
  Resolve it by modeling baseline advance as an explicit state machine: a baseline advances
  ONLY on a confirmed-terminal downstream success for the dispatched commit (reuse the
  tightened measure), never on a FAILURE/ABORT during dispatch, and the failure detection
  must read explicit per-pipeline results, not a lossy count grep. This is code-domain
  orchestrator helper logic (NOT a Jenkinsfile body edit — root Jenkinsfile is near the CPS
  cap). Done when a FAILURE during dispatch can never advance the baseline and a test covers
  the FAILURE/NOT_BUILT/ABORTED-don't-advance cases.
---

# Baseline advances only on confirmed downstream success

## Why this matters

Code-domain. This is the downstream half of the measure-tightening item: even a correct
measure is wasted if the baseline state machine advances on the wrong signal. Phantom-success
advance is what lets a red pipeline's baseline drift forward, masking the failure and
eventually triggering the global-fallback cascade.

## The failure shape

- A child FAILs/ABORTs during dispatch; the baseline logic records success and advances.
- Failure detection is a lossy FAILURE-count grep — NOT_BUILT/ABORTED/UNSTABLE all read as 0,
  so a real failure is undercounted.
- Per-pipeline baseline gets invalidated → falls back to global → full cascade rebuild;
  `lastSuccessful()` pins an ancient green.

## Shape of the fix (code-domain)

Model baseline advance as an explicit state machine: advance ONLY on a confirmed-terminal
success for the dispatched commit (reuse the tightened measure); never advance on FAILURE/
ABORT during dispatch; read explicit per-pipeline results, not a count grep. Logic lives in
the orchestrator helper layer, not inline in a Jenkinsfile (CPS cap). Honor the post-dispatch
UNSTABLE contract (`project_pre_dispatch_hard_fail_post_dispatch_unstable`) so observational
stages don't blank downstream truth.

## Acceptance

A FAILURE during dispatch can never advance the baseline; a test covers the
FAILURE/NOT_BUILT/ABORTED-don't-advance cases.
