---
id: "backlog-edge-deploy-2539797f-conductor-selfheal-device-rollout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Edge deploy of 2539797f (first deploy of the conductor self-heal) left ≥2 device StatefulSets not-Ready — peerCount 13→11, elohim-emma-alpha named — bootstrap pair stayed healthy"
slug: "edge-deploy-2539797f-conductor-selfheal-device-rollout-2026-06-23"
written: "2026-06-23"
author: "overnight deployment-shakeout shift — live alpha probe + ci-observer (edge #1107)"
status: "open"
priority: "high"
tags: [incident, alpha, edge, deploy, conductor, self-heal, CellWithoutGenesis, statefulset, rollout, operator, kubectl]
relatedNodeIds:
  - backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag
cites:
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/main.rs
---

# Edge deploy of 2539797f — conductor self-heal first-deploy device rollout failure

## Observation (evidence)

The dev push `2539797f` triggered **edge #1107**, which came back **UNSTABLE on the deploy
stage**: `elohim-emma-alpha` (archetype `device-recycled-laptop`) **did not reach Ready**
within the kubectl rollout-status timeout (ci-observer, edge #1107). Live `/health` over the
deploy window: **peerCount 13 → 12 → 11** (≥2 devices dropped from the mesh), while
`caughtUp=true`, `conductor.connected=true`, `connected_workers=4/4` held the whole time.

**The matthew/adam bootstrap pair stayed healthy — NO partition.** The user-facing alpha
endpoints (served by matthew's pods) deployed successfully and the two edge code fixes in this
push verified live (`/identity`→SPA 200, `custodians/metrics/recommendations` 503→404).

## Hypothesis (operator to confirm — needs kubectl, which dev cannot run)

This edge build is the **first deploy of the conductor self-heal** (`d8f903007`,
`e2db8beb9`, `4ef67011d` — landed on dev by the integrator; the previously-deployed build
`922a11a` predates them, per the 2026-06-21 visual-verification map). The pushed *code* in
2539797f (an HTTP route narrowing + a panic-guard) **cannot** cause a StatefulSet
Ready-probe failure, so the device rollout failure correlates with the self-heal's
first-deploy behavior on low-resource fixture devices — the exact class the pre-push advisor
flagged, manifesting on a **fixture device, not the bootstrap pair**.

Likely sub-causes to check (operator, `kubectl` — see [[backlog-alpha-conductor-cellwithoutgenesis-floating-happ-tag]]):
- `kubectl describe pod -l app.kubernetes.io/name=elohim-emma-alpha` + logs — CrashLoopBackOff vs OOMKilled vs PVC-Pending.
- Whether `CONDUCTOR_GENESIS_HEAL_ON_BOOT_FAIL` is set for the device archetype and whether emma's cell hit `CellWithoutGenesis` on boot (the destructive re-key path); if so, did it re-genesis or get stuck.
- Recycled-laptop resource limits vs the new storage binary footprint.

## Not in scope to fix from dev

Cluster ops are operator-owned (CLAUDE.md — never `kubectl` from dev; repo manifests are the
cleanup surface). This is surfaced, not chased. If the fix is repo-side (device manifest
resource bump, or gating the genesis-heal flag per archetype), it is a supervised change on
the conductor self-heal work, not part of this shakeout.
