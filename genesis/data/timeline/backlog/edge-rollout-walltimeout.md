---
id: "backlog-edge-rollout-walltimeout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge 1-hour wall-timeout flakes on cold-start statefulset rollout — bump timeout / pre-pull / parallelize the three rollouts"
slug: "edge-rollout-walltimeout"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "CI"
recurrence: 1
source_shifts:
  - "2026-05-16"
domain: "operator"
relatedNodeIds:
  - "memory:project_ci_storage_topology"
  - "memory:project_seed_whoever_is_ready"
tags: [ci, edge, rollout, timeout, operator-domain]
shift_objective: |
  The edge pipeline hits a 1-hour wall-timeout on cold-start statefulset rollout — observed
  ~43% flake (3 of 7 builds) on 2026-05-16. The three sequential statefulset rollouts plus
  cold image pulls don't fit inside the wall budget when the nodes start cold, so the build
  aborts on timeout even though the rollout would have succeeded given more wall-time.
  Resolve it with one or more of: bump the edge wall-timeout to cover a cold-start worst case,
  pre-pull the runtime images so the rollout isn't paying image-pull latency inside the
  timed window, and/or parallelize the three rollouts (per project_seed_whoever_is_ready —
  partial-cluster readiness is steady state, so the rollouts need not be strictly serial).
  This is operator-domain CI tuning (timeout values + pre-pull + rollout scheduling). Done
  when a cold-start edge rollout fits comfortably inside the wall budget and the ~43% flake
  is gone.
---

# Edge cold-start rollout wall-timeout flake

## Why this matters

Operator-domain (wall-timeout + pre-pull + rollout scheduling are pipeline/cluster tuning).
A ~43% flake rate on a deploy-adjacent stage is high enough to erode trust in edge greenness.

## The failure shape

- Three statefulset rollouts run within a 1-hour wall budget.
- On a cold start (cold image pulls, cold nodes) the combined time exceeds the budget.
- The build aborts on the wall-timeout — a false failure; the rollout would have completed.

## Shape of the fix (operator-owned)

1. Bump the wall-timeout to cover a cold-start worst case.
2. Pre-pull runtime images so image-pull latency is outside the timed window.
3. Parallelize the three rollouts where safe (`project_seed_whoever_is_ready` — partial
   readiness is acceptable steady state, so strict serial rollout isn't required).

## Acceptance

A cold-start edge rollout fits inside the wall budget; the ~43% flake is eliminated.
