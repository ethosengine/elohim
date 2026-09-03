---
id: "backlog-conductor-split-left-doorway-admin-url-on-the-storage-service"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Conductor split left the doorway CONDUCTOR_ADMIN_URL on the storage Service — every doorway restart since 2026-08-31 died minting its auth token (apex 503, edge #1420 hung 2h); hand-written Service names in doorway manifests must be rendered or linted against the conductor template"
slug: "conductor-split-left-doorway-admin-url-on-the-storage-service"
written: "2026-09-03"
author: "shift 2026-09-02T02-20-land-rung5-batch"
status: "fix-landed-unverified"
priority: "high"
domain: "D-runtime-operations"
roadmap_rung: "delivery floor — fleet wiring coherence after the conductor split (9c9f9fc65); feeds death-witness (probe kills leave no ERROR) and the doorway-failover habit"
relatedNodeIds: []
tags: [doorway, conductor-split, manifests, service-wiring, auth-token, crashloop, apex-outage, death-witness, doorway-failover, edge-pipeline]
cites:
  - genesis/orchestrator/manifests/doorway/alpha.yaml
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
  - genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml
  - elohim/holochain/Jenkinsfile
  - doorway/doorway-service/src/main.rs
  - genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md
  - genesis/data/timeline/backlog/ci-orchestrator-supersede-aborts-in-flight-edge-rolls.md
---

## What happened (2026-08-31 → 2026-09-03, alpha)

The conductor split (9c9f9fc65) moved the conductor into `<prefix>-conductor-0` with its own
Services (`<prefix>-conductor`, ports 4444→8444 / 4445→8445 via the socat sidecar that moved with
it). The edge Jenkinsfile (`computeConductorUrls`) and the genesis Jenkinsfile were updated to render
`CONDUCTOR_URLS` against `-conductor`. The **hand-written** `CONDUCTOR_ADMIN_URL` in the five doorway
manifests (alpha, alpha-b, prod, staging, staging-read) still named the storage Service
(`ws://elohim-<human>-<env>:4444` → targetPort 8444 on a pod that no longer listens).

Effect: from the first post-split storage roll, every doorway container start ran
`mint_app_auth_token` → `Admin client connect failed while minting auth token … Connection timed out
(os error 110)` (one WARN, attempt 1) and was killed by the probe before attempt 2 — the mint loop's
5×(≤5 s) backoff never gets past one TCP connect timeout (~130 s). doorway-alpha survived only on the
pre-split pod (0 restarts, token minted on 2026-08-31); doorway-B (apex `doorway.elohim.host`) lost
its last Ready pod at its first restart → nginx 503; every edge deploy since hung on the doorway
rollout (edge #1420: 2 h, UNSTABLE). The pre-split doorway image failed identically → fleet wiring,
not doorway code. Diagnosed entirely from Prometheus (`kube_pod_container_status_waiting_reason`,
`kube_service_info`, `kube_pod_container_info`) and Loki — no kubectl.

## Fix landed

2d356dbc2 — all five manifests → `ws://<prefix>-conductor:4444`. Verify: doorway Deployments reach
0 unavailable on the next edge roll; apex `/db/content/elohim-host-landing` 200; genesis preflight
"Doorway not ready" no longer fires.

## What must outlive the fix

1. **Render, don't hand-write, cross-workload Service names.** `CONDUCTOR_ADMIN_URL` should be a
   placeholder the edge Jenkinsfile fills from the same `computeConductorUrls` family
   (`<prefix>-conductor`), or a manifest lint in `scripts/ci/` must refuse any `ws://elohim-*:444[45]`
   that does not end in `-conductor` (the storage Service no longer carries those ports at all —
   consider deleting the 4444/4445 ports from the storage Service so a stale name fails at DNS/port
   level instead of a 130 s timeout).
2. **A probe kill must be a witnessed death.** The doorway's "last words" were one WARN; nothing at
   ERROR. The death-witness runtime must capture *why the probe killed it* (no listener yet ↔ the
   mint loop blocking bind) — the generalized lesson from the conductor crash-loop.
3. **Habit `doorway-failover`** is declared green while both doorways failed on restart for 3 days —
   the register did not see it because no fleet lane restarts a doorway and reads it. Bind the habit's
   check to a rollout-completion read (Prometheus `kube_deployment_status_replicas_unavailable == 0`
   for both doorway Deployments after each edge roll).
4. **Startup should not block bind on the mint.** Bind the HTTP listener first (readiness = listening,
   conductor auth = degraded flag), so a slow conductor never turns into a probe crash-loop.
