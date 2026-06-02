---
id: "backlog-elohim-epr-job-provision"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Provision the elohim-epr multibranch Jenkins job (currently 404 / not provisioned)"
slug: "elohim-epr-job-provision"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "low"
area: "CI"
recurrence: 2
source_shifts:
  - "2026-05-17"
  - "2026-05-22"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_orchestrator_build_manifest_required"
  - "memory:project_orchestrator_predictive_vision"
tags: [ci, jenkins, orchestrator, epr, operator-domain]
shift_objective: |
  The orchestrator's dependency graph dispatches an `elohim-epr` pipeline, but the
  corresponding Jenkins multibranch job is not provisioned — the dispatch 404s. The
  orchestrator already soft-skips it (NOT_PROVISIONED is handled gracefully, so it doesn't
  fail the parent), but the EPR crate therefore has no CI of its own; it rides only on
  whatever downstream pulls it in. Recurred 2026-05-17 and 05-22.
  Resolve it by provisioning the multibranch item pointed at `elohim/epr/Jenkinsfile` so the
  EPR crate gets first-class CI. This is operator-domain (creating the Jenkins job is an
  operator action); the repo side is confirming `elohim/epr/Jenkinsfile` + its
  build-manifest.json are present and graph-walkable (per feedback_orchestrator_build_manifest_required).
  Done when the elohim-epr job exists, the orchestrator dispatch resolves instead of 404ing,
  and the EPR crate builds/tests on its own changes.
---

# Provision the elohim-epr multibranch job

## Why this matters

Operator-domain (Jenkins job creation). Low priority because the orchestrator already
soft-skips the missing job (NOT_PROVISIONED), so it isn't blocking — but the EPR crate
currently has no dedicated CI, which is a coverage gap waiting to bite.

## The failure shape

- Orchestrator graph includes `elohim-epr`; the dispatch targets a job that doesn't exist.
- Jenkins returns 404 / NOT_PROVISIONED; orchestrator soft-skips (no parent failure).
- Net: EPR crate changes get no first-class build/test until a downstream consumer pulls them.

## Shape of the fix

1. Operator provisions the multibranch item pointed at `elohim/epr/Jenkinsfile`.
2. Repo side: confirm `elohim/epr/Jenkinsfile` + `build-manifest.json` exist and are
   graph-walkable (`feedback_orchestrator_build_manifest_required`).

## Acceptance

The elohim-epr job exists; the orchestrator dispatch resolves rather than 404ing; the EPR
crate builds and tests on its own changes.
