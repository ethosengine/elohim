---
id: "backlog-ci-rbac-jenkins-deployer"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Commit jenkins-deployer / ee-jenkins ServiceAccount RBAC so cluster-permission drift is reviewable from code"
slug: "ci-rbac-jenkins-deployer"
written: "2026-06-02"
author: "cartographer"
status: "proposed"
priority: "high"
area: "recovery/k8s"
recurrence: 3
source_shifts:
  - "2026-04-27"
  - "2026-05-04"
  - "2026-05-26"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_no_kubectl_from_dev_env"
  - "memory:project_ci_storage_topology"
tags: [ci, rbac, k8s, operator-domain, recurring, deploy]
shift_objective: |
  The jenkins-deployer / ee-jenkins ServiceAccount repeatedly loses (or never had) the
  cluster permissions a deploy stage needs — PVC, Deployment, Service, and scale operations
  return Forbidden mid-rollout. The failure is masked because the deploy step swallows the
  kubectl exit with a trailing `|| true` (`+ true` in the build log), so the pipeline goes
  green while nothing actually rolled. The drift recurred across three shifts (2026-04-27,
  2026-05-04, 2026-05-26) and each time cost a manual operator RBAC patch that left no
  reviewable trace.
  Resolve it by committing the deployer SA's Role/RoleBinding manifests into the repo so
  the grant is code-reviewable and drift is diffable — e.g. `rbac/jenkins-deployer-{ns}.yaml`
  per target namespace — and remove the `|| true` mask from the deploy step so a Forbidden
  fails loudly instead of phantom-passing. Operator owns the apply; the repo owns the
  source-of-truth manifest. Done when the deployer's permissions live in-tree, a missing
  grant fails the deploy stage visibly, and an operator can reconcile the cluster from the
  committed RBAC rather than from memory.
---

# Commit jenkins-deployer RBAC so permission drift is reviewable from code

## Why this matters

This is an operator-domain item (the agent cannot apply cluster RBAC — see
`feedback_no_kubectl_from_dev_env`), surfaced for the operator and seeded into the repo so
the *next* drift is caught at review time, not at 2am mid-rollout. The recurrence across
three separate shifts is the signal that the permission set is not pinned anywhere durable.

## The failure shape

- The jenkins-deployer / ee-jenkins ServiceAccount lacks one or more of: PVC create/patch,
  Deployment create/patch, Service create/patch, `deployments/scale` update — in the target
  namespace.
- The deploy step swallows the error (`kubectl ... || true`), so the log shows `+ true` and
  the stage reports SUCCESS while the cluster state is unchanged.
- The mask means the regression is only noticed downstream (a Verify-Target-Health probe or
  a human looking at the running pods), well after the green build.

## Shape of the fix

1. Commit the deployer SA's Role + RoleBinding (per namespace) into the repo as the
   source-of-truth RBAC, so drift is a reviewable diff.
2. Remove the `|| true` from the deploy/scale step so a Forbidden fails the stage.
3. Operator reconciles the cluster from the committed manifests (apply is operator-owned).

## Acceptance

The deployer's required permissions live in-tree; a missing grant fails the deploy stage
visibly (no phantom-green); the operator can reconcile RBAC from code rather than memory.
