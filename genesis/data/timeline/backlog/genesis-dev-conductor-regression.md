---
id: "backlog-genesis-dev-conductor-regression"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis baseline UNSTABLE on dev — CellDisabled seed 503s, commit-hash drift, missing conductor-readiness.json"
slug: "genesis-dev-conductor-regression"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "CI"
recurrence: 1
source_shifts:
  - "2026-05-18"
domain: "operator"
relatedNodeIds:
  - "memory:project_seed_whoever_is_ready"
  - "memory:project_ci_storage_topology"
  - "memory:feedback_sweettest_cross_agent_consistency"
tags: [ci, conductor, seed, genesis, operator-domain]
shift_objective: |
  The genesis baseline is UNSTABLE on dev with a cluster of conductor deploy/init symptoms:
  seeding 503s with CellDisabled (the conductor cell isn't enabled when the seeder hits it),
  commit-hash drift between the deployed image and HEAD, and a missing
  conductor-readiness.json that the verify step expects. Together these read as a conductor
  deploy/init regression rather than a content problem (observed 2026-05-18).
  Resolve it by making the seed wait for conductor readiness deterministically: ensure the
  cell is enabled before the seeder runs (CellDisabled 503 = ordering bug), emit and check
  conductor-readiness.json as the gate, and pin/verify the deployed commit-hash so the
  seeder isn't racing a stale image. This is operator-domain (conductor deploy ordering +
  readiness gating live in the cluster pipeline). Done when the genesis baseline is stable on
  dev — no CellDisabled 503s, readiness gated on conductor-readiness.json, and no commit-hash
  drift between image and HEAD.
---

# Genesis dev conductor deploy/init regression

## Why this matters

Operator-domain (conductor deploy ordering + readiness gating are cluster pipeline concerns).
An UNSTABLE baseline on dev poisons every downstream measure — the orchestrator's per-pipeline
baseline can't advance, so this has cascade reach beyond genesis itself.

## The failure shape (one regression, three symptoms)

- **CellDisabled 503 on seed** — the seeder hits the conductor before the cell is enabled
  (an ordering bug, not a content bug).
- **Commit-hash drift** — the deployed image lags HEAD, so the seeder races a stale conductor.
- **Missing conductor-readiness.json** — the verify gate has nothing to check against.

## Shape of the fix (operator-owned)

1. Gate the seed on conductor readiness: enable the cell before the seeder runs; emit and
   require `conductor-readiness.json`.
2. Pin/verify the deployed commit-hash so the seeder isn't racing a stale image.
3. Cross-reference `project_seed_whoever_is_ready` — partial readiness is fine, but a
   *disabled* cell is a hard error, not a partial-cluster steady state.

## Acceptance

Genesis baseline stable on dev: no CellDisabled 503s, readiness gated on
conductor-readiness.json, no commit-hash drift between image and HEAD.
