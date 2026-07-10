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
recurrence: 4
source_shifts:
  - "2026-04-27"
  - "2026-05-04"
  - "2026-05-26"
  - "2026-07-06"
domain: "operator"
relatedNodeIds:
  - "memory:feedback_no_kubectl_from_dev_env"
  - "memory:project_ci_storage_topology"
tags: [ci, rbac, k8s, operator-domain, recurring, deploy, statefulsets-scale, zombie-conductors, storm-amplifier]
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

## 4th recurrence — 2026-07-06 (edge #1160): the mask now hides ACTIVE HARM, not a no-op

Live, evidenced instance during the P2P inventory-snapshot storm-heal deploy (edge #1160,
dev `f79ab5a4`). `cleanupOrphanedHumans` (`elohim/holochain/Jenkinsfile:472–491`) correctly
detected all seven personas suspended by the 2026-07-02 coordination-ladder cast
(`deployments.json` commits `a6e2e2e58` + `5666ebe02`) and emitted, verbatim:

```
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-caleb-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-daniel-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-emma-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-frank-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-nancy-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-pete-alpha
Scaling orphaned StatefulSet to 0: elohim-alpha/elohim-terrance-alpha
```

Every `kubectl scale statefulset/<name>-alpha -n elohim-alpha --replicas=0` returned
`Error from server (Forbidden)` — jenkins-deployer lacks **`statefulsets/scale`** patch (note:
the 2026-06-02 write named `deployments/scale`; the conductor workloads are StatefulSets, so the
`statefulsets` subresource is the specific missing grant). The `|| true` (`+ true`) masked every
failure → edge #1160 reported **SUCCESS**.

**What's new vs the prior three recurrences — the mask hides compounding harm, not a no-op.**
The un-scaled orphans are not inert. These 7 conductors have run continuously since 2026-07-02 on
the OLD pre-fix `elohim-storage` image and re-apply every received inventory snapshot
non-idempotently: **12.7–61.9K applies/30min each (~6× a fixed peer), crash-looping
(`reason=Error`, restart counts climbing)**. They (a) DEFEAT the directive's own intent — shem
capacity is never released and the `cluster-to-shem-p2p-request-starvation-11-peer-blackout`
falsifier is contaminated (the "removed" peers are still consuming) — and (b) churn the DHT so
the correctly-healed 7 peers cannot converge (`caughtUp=false`, `divergentAnchor=1533` in #1160's
own dataplane-validation stage), which keeps `elohim-genesis` / `elohim-app` CI stages UNSTABLE.
So here the fail-open converts a silent no-op into a silent, self-compounding mesh-pollution
incident that masquerades as a green deploy.

**Immediate operator action (stops the live storm at its source):**
```
kubectl scale statefulset/elohim-{caleb,daniel,emma,frank,nancy,pete,terrance}-alpha \
  -n elohim-alpha --replicas=0
```
This does exactly what the 2026-07-02 directive always intended (these pods were meant to be 0).

**Resolved (this instance), 2026-07-07:** the operator cleared the live storm by **deleting** the
7 StatefulSets — notably *not* by scaling them (a `scale` would itself have been Forbidden — the
deletion path was the available lever, which is itself evidence of the gap). Storm stopped, mesh
reconverging. The underlying RBAC drift PERSISTS (jenkins-deployer still cannot scale), so the
durable fix below stays open and the concern stays `proposed`.

**Durable fix + sequencing constraint.** Grant jenkins-deployer `statefulsets/scale` (alongside
`deployments/scale` + the PVC/Deployment/Service verbs named above) in
`elohim-alpha`/`staging`/`prod`, committed in-tree per the fix shape above. THEN remove the
`|| true` mask on the scale step. **Order matters:** removing the mask BEFORE the grant lands
turns every edge deploy into a hard FAILURE (the orphan-scale is always attempted while
suspended personas exist). Grant first, unmask second. A safe interim (repo-side, no RBAC
dependency) is to keep the deploy non-blocking but make the drift loud — count Forbidden scales
and set the stage UNSTABLE with an explicit `RBAC DRIFT` banner — so the phantom-green stops
without breaking deploys before the grant is in place.
