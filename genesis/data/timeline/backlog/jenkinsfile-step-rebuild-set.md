---
id: "backlog-jenkinsfile-step-rebuild-set"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Pipeline Jenkinsfiles ignore the orchestrator step-level rebuild set (holochain rebuilds all stages)"
slug: "jenkinsfile-step-rebuild-set"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "CI/orchestrator"
recurrence: 1
source_shifts:
  - "2026-05-05"
domain: "code"
relatedNodeIds:
  - "memory:project_orchestrator_predictive_vision"
  - "memory:feedback_understand_orchestrator_substrate_before_changes"
  - "memory:feedback_orchestrator_build_manifest_required"
tags: [ci, orchestrator, jenkinsfile, rebuild-set, code-domain]
shift_objective: |
  The orchestrator computes a step-level rebuild set (which stages a changeset actually
  requires) and passes it downstream, but the per-pipeline Jenkinsfiles ignore it and rebuild
  every stage anyway — most visibly the holochain pipeline, which re-runs all stages on any
  trigger regardless of what changed (observed 2026-05-05). The orchestrator's selectivity is
  wasted because the consuming pipeline doesn't honor it.
  Resolve it by having the pipeline read the orchestrator's step-level rebuild set and gate its
  stages on it (skip stages not in the set). This is code-domain in INTENT, but the gating
  lives in Jenkinsfile stage `when{}` blocks — and per the safety rule, do NOT edit any
  Jenkinsfile in this backlog item's authoring; the design must keep the gating logic in a
  helper above `pipeline {}` (CPS method-size cap), never inline in stages. The implementing
  shift wires the rebuild-set consumption via a helper. Done when a holochain change touching
  one stage's inputs rebuilds only that stage's dependents, not all stages.
---

# Pipelines honor the orchestrator step-level rebuild set

## Why this matters

Code-domain. The orchestrator already does the expensive part (computing which stages a
changeset needs); the savings are forfeited because the downstream pipeline ignores the set
and rebuilds everything. Honoring it turns the orchestrator's predictive build-graph
(`project_orchestrator_predictive_vision`) into real wall-time savings.

## The failure shape

- Orchestrator computes a step-level rebuild set and passes it downstream.
- Per-pipeline Jenkinsfiles (notably holochain) ignore it → rebuild all stages.
- A one-stage change pays for a full pipeline rebuild.

## Shape of the fix (code-domain intent; helper-scoped implementation)

The pipeline reads the orchestrator's step-level rebuild set and gates its stages on it. The
gating decision MUST live in a helper above `pipeline {}` (CPS method-size cap — never inline
in stage `when{}`). Read `strategy.mjs` + the orchestrator README and confirm the
build-manifest covers the stage inputs first
(`feedback_understand_orchestrator_substrate_before_changes`,
`feedback_orchestrator_build_manifest_required`). Authoring this backlog item does not touch
any Jenkinsfile.

## Acceptance

A holochain change touching one stage's inputs rebuilds only that stage's dependents, not all
stages.
