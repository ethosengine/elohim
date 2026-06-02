---
id: "backlog-genesis-cross-ns-netpol"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Cross-namespace NetworkPolicy blocks Verify-Target-Health storage check (jenkins ns → elohim-alpha ns)"
slug: "genesis-cross-ns-netpol"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "medium"
area: "k8s networking"
recurrence: 1
source_shifts:
  - "2026-04-28"
domain: "operator"
relatedNodeIds:
  - "memory:project_doorway_single_target_no_fanout"
  - "memory:feedback_no_kubectl_from_dev_env"
tags: [ci, k8s, networkpolicy, operator-domain, doorway]
shift_objective: |
  The genesis pipeline's Verify-Target-Health stage runs in the `jenkins` namespace and
  tries to reach an elohim-storage pod in the `elohim-alpha` namespace to confirm the seed
  landed. A cross-namespace NetworkPolicy blocks that traffic, so the health check fails (or
  is skipped) even when storage is actually healthy — a false-negative on delivery
  verification.
  Resolve it with one of three routes, operator's choice: (1) route the check through the
  doorway proxy that is already allowed to reach storage (preferred — it exercises the real
  user path and respects single-target dispatch), (2) add a scoped NetworkPolicy exception
  for the jenkins-ns verify probe, or (3) co-locate the verify pod in the namespace it
  probes. This is operator-domain k8s networking; surface the options and let the operator
  apply. Done when Verify-Target-Health can confirm storage health without a NetworkPolicy
  false-negative, ideally via the doorway path so it mirrors how a real client reaches
  storage.
---

# Cross-namespace NetworkPolicy blocks the storage health check

## Why this matters

Operator-domain (NetworkPolicy is a cluster apply). Surfaced because a blocked
Verify-Target-Health probe undermines the whole point of the verify stage — it can't
distinguish "storage is down" from "I'm not allowed to talk to storage."

## The failure shape

- Verify-Target-Health runs in `jenkins`; the storage pod lives in `elohim-alpha`.
- A cross-namespace NetworkPolicy denies jenkins-ns → elohim-alpha-ns traffic.
- The probe fails or is force-skipped, producing a false-negative even on a healthy seed.

## Shape of the fix (operator picks)

1. **Preferred:** route the verify probe through the doorway proxy (already permitted to
   reach storage; exercises the real client path; respects single-target dispatch per
   `project_doorway_single_target_no_fanout`).
2. A scoped NetworkPolicy exception for the verify probe only.
3. Co-locate the verify pod in `elohim-alpha`.

## Acceptance

Verify-Target-Health confirms real storage health without a NetworkPolicy false-negative,
ideally via the doorway path.
