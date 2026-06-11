---
id: "backlog-ci-genesis-conductor-adminws-unreachable"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis conductor admin-WS unreachable from CI — household-formation seeding never runs (the dominant genesis-UNSTABLE cause)"
slug: "ci-genesis-conductor-adminws-unreachable"
written: "2026-06-08"
author: "agentic-developer (overnight shift)"
status: "done"
priority: "high"
ci_status: verified
jobs: [elohim-genesis]
tags: [ci, genesis, seeding, conductor, admin-ws, substrate, escalation, operator-owned]
cites:
  - genesis/Jenkinsfile
  - genesis/manifests/humans/matthew-manager.yaml
  - genesis/seeder/src/seed-conductor-identities.ts
  - genesis/orchestrator/manifests/network-policies.yaml
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

## Root cause CONFIRMED 2026-06-10: NetworkPolicy, by design
Of the three candidates above, it is the **NetworkPolicy** — and it is deliberate, not
drift. `genesis/orchestrator/manifests/network-policies.yaml` (`default-deny-cross-env`,
elohim-alpha) admits jenkins-ns traffic **only on pod ports 8090 (storage) and 8080
(doorway)**; its own comment said "conductor admin/app ports (4444/4445) remain blocked"
(introduced `1ba3c2aa2` 2026-04-28, *before* the seed stages were wired `04565b24a`
2026-05-07). The other two candidates are RULED OUT: the doorway connects to the same
conductors through the same Service:4444→pod:8444 socat bridge (live 14/14 pools healthy),
so the bridge runs and the admin interface listens — the path is broken only
cross-namespace from jenkins. Jenkins archaeology (conductor-readiness.json across all
retained builds #1099–#1118) shows `readyCount: 0, unreadyCount: 3` in **every** build:
the probes have NEVER passed; seeding has never run from CI.

NetworkPolicy gotcha encoded here for posterity: policy `ports` match the **post-DNAT pod
port** — allowing 4444/4445 would be a no-op; the rule must open **8444/8445**.

## Verdict / disposition
**Repo fix landed 2026-06-10**: `network-policies.yaml` jenkins rule now includes pod
ports 8444 (admin-WS bridge) + 8445 (app-WS bridge) — rationale in the rule comment.
The file is `managed-by: manual`, so the live cluster does NOT reconcile from a push;
**operator action: `kubectl apply -f genesis/orchestrator/manifests/network-policies.yaml`**
(same flow as `1ba3c2aa2`, which was live-patched first and committed after).
The port-open is **scoped bootstrap debt** (operator-ratified 2026-06-10): network-position
auth on conductor admin cuts against the delegates-compute grant direction; the consistent
authorization end-state (and the condition for re-closing 8444/8445) is tracked in
`security-ci-substrate-authorization-grant-coherence.md`.
`ci_status: blocked` until that apply happens — then the next genesis build self-verifies:
`probeConductorReadiness` flips to ready, the three seed stages run, and
`conductor-readiness.json` records `allReady: true`. This remains the highest-leverage
lever: it removes 3 stage-UNSTABLEs, the 33-step content-alpha resilience cascade, AND
unblocks the conductor-seeded signal chain (`PeerStatusRecorded` → `/api/v1/peer-statuses`
→ `/api/v1/network/posture`) that the substrate-validation stages assert on.

**UPDATE 2026-06-11: operator confirmed the netpol IS applied.** Build #1118
(2026-06-10T20:47Z, all probes still refused) predates the apply, and no genesis
build has run since (latest = #1118, nothing queued) — so the apply is real but
unexercised. `ci_status` flipped blocked → pending-verification: the next genesis
trigger is the self-verification (expect `[probe] ✅` ×3, the three seed stages
actually seeding for the first time from CI, and — once the substrate-validation
suite commits land — `CONDUCTOR_SEEDING_READY=true` un-gating Verify Resilience
Signals and the Workstream-D junction fill).

**VERIFIED 2026-06-11, genesis #1119 (first run after the apply): RESOLVED.**
All three conductors probed `[probe] ✅ …:4444 reachable` in all three seed
stages; the stages genuinely seeded for the first time from CI (identities:
5 existing/idempotent; peer bindings: **9 bindings written, 6 humans, 0
failed**; household formation reached its ceremony logic). The concern's
scope ends here — the next rung of the seeding chain surfaced its own
distinct bug (founder binding FATAL, tracked in
`ci-genesis-household-founder-binding.md`).

## Diagnosis provenance
Overnight agentic-developer shift 2026-06-08 (ci-investigator on builds #1104–#1106 +
manifest/Jenkinsfile/seeder read). Root cause confirmed 2026-06-10 by parallel
investigation (Jenkins artifact archaeology #1099–#1118 + netpol/manifest/doorway-config
correlation); netpol repo fix landed same day (this session).
