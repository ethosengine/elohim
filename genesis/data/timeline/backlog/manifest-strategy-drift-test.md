---
id: "backlog-manifest-strategy-drift-test"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Add a build-manifest ⊆ orchestrator-strategy changePatterns drift test"
slug: "manifest-strategy-drift-test"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI/orchestrator"
recurrence: 2
source_shifts:
  - "2026-05-04"
  - "2026-05-22"
domain: "code"
relatedNodeIds:
  - "memory:feedback_orchestrator_build_manifest_required"
  - "memory:feedback_understand_orchestrator_substrate_before_changes"
  - "memory:project_orchestrator_predictive_vision"
tags: [ci, orchestrator, manifest, drift-test, code-domain, recurring]
shift_objective: |
  Per-project build-manifest.json files declare the source paths that should trigger a
  pipeline, and the orchestrator strategy declares its own changePatterns for which paths
  dispatch which pipeline. These two drift apart silently: a path family covered by a
  manifest may not be in the strategy's changePatterns (the `data/**` gap was one of several),
  so a change that SHOULD trigger a build doesn't, and nobody notices until something
  downstream is stale (observed 2026-05-04, 05-22).
  Resolve it with a drift test that asserts every path a build-manifest claims is also covered
  by the orchestrator strategy's changePatterns (manifest ⊆ strategy), failing at PR time on
  drift. This is code-domain: a test under the orchestrator's test surface reading the
  manifests via the existing graph-walker / pipeline-registry (per
  feedback_orchestrator_build_manifest_required), NOT a Jenkinsfile edit. Done when a
  manifest path not covered by strategy changePatterns fails a committed drift test.
---

# build-manifest ⊆ orchestrator-strategy changePatterns drift test

## Why this matters

Code-domain. Silent trigger-drift is a "build that should have run but didn't" class — the
worst kind, because it leaves a green board over stale artifacts. A drift test converts the
silent failure into a PR-time failure.

## The failure shape

- A build-manifest declares it cares about a path family (e.g. `data/**`).
- The orchestrator strategy's changePatterns don't include that path.
- A change under that path doesn't dispatch the pipeline; the gap is invisible until staleness
  surfaces downstream.

## Shape of the fix (code-domain)

A test asserting **manifest paths ⊆ strategy changePatterns** for every pipeline, reading the
manifests through the existing `graph-walker.mjs` / `pipeline-registry.mjs`
(`feedback_orchestrator_build_manifest_required`). It lives in the orchestrator test surface,
not inline in any Jenkinsfile. Read `strategy.mjs` + the orchestrator README first
(`feedback_understand_orchestrator_substrate_before_changes`).

## Acceptance

A manifest path not covered by the strategy's changePatterns fails a committed drift test.
