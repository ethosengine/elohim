---
id: "backlog-ci-genesis-conductor-adminws-unreachable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis conductor admin-WS unreachable from CI — household-formation seeding never runs (the dominant genesis-UNSTABLE cause)"
slug: "ci-genesis-conductor-adminws-unreachable"
written: "2026-06-08"
author: "agentic-developer (overnight shift)"
status: "wip"
priority: "high"
ci_status: blocked
jobs: [elohim-genesis]
tags: [ci, genesis, seeding, conductor, admin-ws, substrate, escalation, operator-owned]
cites:
  - genesis/Jenkinsfile
  - genesis/manifests/humans/matthew-manager.yaml
  - genesis/seeder/src/seed-conductor-identities.ts
---

# Genesis conductor admin-WS unreachable — the dominant genesis-UNSTABLE cause

## Symptom (consistent across builds #1104–#1106)
The genesis pipeline's three conductor-mediated seed stages — **Seed Conductor
Identities**, **Seed Agent Peer Bindings**, **Seed Household Formation** — each
emit `unstable()` with:
```
[probe] ❌ elohim-matthew-alpha…:4444 unreachable (admin WS not accepting TCP)
[probe] ❌ elohim-adam-alpha…:4444 unreachable
[probe] ❌ elohim-jessica-alpha…:4444 unreachable
WARNING: All 3 conductors unreachable — skipping seed (see conductor-readiness.json)
```
**These three stages have NOT successfully run in any of the last 4 builds.**
Because household formation never runs, the resilience a2o scenarios fail (5×,
~33 failed steps): `"content-alpha" is stewarded by 0 households; expected ≥2`
(`genesis/a2o/steps/resilience.steps.ts:199`) — a direct cascade.

## Root cause: admin-WS path refused while storage works (runtime, NOT a repo bug)
- The same pod's **storage (8090) IS reachable** (`✅ elohim-matthew-alpha storage is ready`),
  so the pod is up — only the **admin WebSocket** path is refused.
- The repo manifest ports are **coherent**: `genesis/manifests/humans/matthew-manager.yaml`
  headless Service maps `admin-ws port:4444 → targetPort:8444`, and the pod runs a socat
  sidecar `socat TCP-LISTEN:8444 … TCP:127.0.0.1:4444` (the conductor binds admin on
  localhost:4444; socat bridges 8444→localhost:4444; the Service re-labels that as 4444).
  The Jenkinsfile probe (`probeConductorReadiness`, adminPort = appPort−1 = 4444) and the
  seeder (`seed-conductor-identities.ts:123` "4445 app → 4444 admin") both target 4444 —
  **matching the Service.** So this is NOT a port-number mismatch in the repo.
- Therefore the refusal is **runtime/operator**: the socat sidecar is not running, OR the
  conductor's admin interface (localhost:4444) is not up (vs the app interface), OR a
  NetworkPolicy blocks the jenkins-ns CI pod from the elohim-alpha ns on the admin port.
  Storage (8090) crossing fine narrows it to the admin-WS path specifically.

## Verdict / disposition
**ESCALATION — operator-owned** (never kubectl from dev; the repo manifest is coherent so
there is no repo cleanup surface to reconcile here). This is the **highest-leverage** lever
for flipping genesis seeding UNSTABLE→SUCCESS: it removes 3 stage-UNSTABLEs AND the 33-step
content-alpha resilience cascade. `ci_status: blocked` on operator substrate action.

**Operator action:** on the alpha conductor pods (matthew/jessica; adam is shem-suspended),
verify the socat admin-WS sidecar is running and the conductor admin interface is listening,
and that the jenkins-ns CI pod can reach the admin-WS port on the elohim-alpha ns (vs the
working 8090 storage path). Then re-run genesis; the three conductor stages + the resilience
content-alpha scenarios should clear.

## Diagnosis provenance
Overnight agentic-developer shift 2026-06-08 (ci-investigator on builds #1104–#1106 +
manifest/Jenkinsfile/seeder read). No code landed (operator-owned runtime issue).
