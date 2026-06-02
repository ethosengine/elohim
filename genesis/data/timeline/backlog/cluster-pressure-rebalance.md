---
id: "backlog-cluster-pressure-rebalance"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Cluster pressure rebalance — intel-nuc 135% CPU, jessica edgenode OOM-flap, nodeAffinity rebalance"
slug: "cluster-pressure-rebalance"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "recovery"
recurrence: 1
source_shifts:
  - "2026-05-05"
domain: "operator"
relatedNodeIds:
  - "memory:project_ci_storage_topology"
  - "memory:project_household_horizontal_scaling"
  - "memory:feedback_multi_agent_pvc_pacing"
tags: [recovery, cluster, pressure, nodeaffinity, operator-domain]
shift_objective: |
  The alpha cluster runs hot: the intel-nuc node reported ~135% CPU and the jessica edgenode
  is OOM-flapping, which trips evictions and rollout instability elsewhere (observed
  2026-05-05). Workloads are not balanced across nodes by capacity — heavy pods land on
  nodes that can't carry them.
  Resolve it with a nodeAffinity / resource-request rebalance: set realistic requests/limits
  so the scheduler spreads load by capacity, pin memory-heavy workloads off the OOM-flapping
  edgenode, and review whether the intel-nuc is carrying work that belongs on a blade
  (project_household_horizontal_scaling — more blades = more node instances with different
  roles). This is operator-domain cluster work (node labels, affinity, resource requests are
  cluster applies). Done when no node sits pegged at >100% CPU under normal load and the
  jessica edgenode stops OOM-flapping.
---

# Cluster pressure rebalance

## Why this matters

Operator-domain (nodeAffinity, resource requests, and node labels are cluster applies — see
`feedback_no_kubectl_from_dev_env`). A pegged node and an OOM-flapping edgenode cause
evictions that masquerade as unrelated CI flakes (e.g. checkout SIGTERMs), so the pressure is
a hidden root cause.

## The failure shape

- intel-nuc at ~135% CPU — oversubscribed; work that should spread is concentrated.
- jessica edgenode OOM-flapping — memory-heavy pods scheduled where they don't fit.
- Downstream: evictions, rollout instability, mid-clone SIGTERMs on pressured nodes.

## Shape of the fix (operator-owned)

1. Set realistic resource requests/limits so the scheduler spreads by capacity.
2. nodeAffinity memory-heavy workloads off the OOM-flapping edgenode.
3. Review intel-nuc load against `project_household_horizontal_scaling` (push work onto
   blades with appropriate roles rather than concentrating it).

## Acceptance

No node pegged >100% CPU under normal load; jessica edgenode stops OOM-flapping.
