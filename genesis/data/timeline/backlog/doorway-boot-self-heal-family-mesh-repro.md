---
id: "backlog-doorway-boot-self-heal-family-mesh-repro"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Doorway boot-order self-heal family — three gaps reproduced on the local mesh (SSR materialization boot-once; seed-blob forward silent loss; upstream breaker flap on hostname resolution)"
slug: "doorway-boot-self-heal-family-mesh-repro"
written: "2026-08-16"
author: "claude (local-mesh-saga-delivery shift, desk reproduction)"
status: "open"
priority: "medium"
tags: [doorway, self-heal, boot-order, ssr, seed-blob, breaker, local-mesh, dataplane]
cites:
  - doorway/doorway-service/src/render/registry.rs
  - doorway/doorway-service/src/routes/seed.rs
  - doorway/doorway-service/src/routes/upstream_health.rs
  - app/elohim-app/scripts/hc-mesh.sh
---

# Doorway boot-order self-heal family (local-mesh reproduction, 2026-08-16)

Doorway and storage restart independently (pod restarts, deploys, the local
hc-mesh). One member of this family is FIXED; three siblings remain. All were
reproduced live on the 3-peer local mesh in one evening — each is invisible on
alpha only because k8s readiness ordering usually masks it.

## Fixed (same session)

**Steward-peer route registration** was boot-once: doorway up before storage →
`registered:0 failed:3`, `totalRoutes: 0`, every `/db/*` 404 until an operator
restarted doorway (the `restart-doorway-epr.sh` crutch). Fixed by idempotent
`install_steward_routes` (replace-by-source) + a background retry task
(15s cadence until every peer registers). Commit b50c3a641.

## Open sibling 1 — SSR materialization is boot-once

`render/registry.rs:417`: when the DEFAULT slug's server-bundle
materialization fails at boot (storage not yet answering), the whole registry
returns `Self::empty()` and never retries — the doorway stays CSR-only until
restart. The reconcile task (`decide_reconcile`) reconciles *loaded* apps; an
empty registry has none. Cure direction: the reconcile loop should attempt
materialization for slugs listed in `SSR_BUNDLE_SLUGS` that have no loaded
renderer (same retry-until-healthy shape as the route-registry fix).

## Open sibling 2 — seed-blob forward claimed success on a blob that never became servable

`PUT /admin/seed/blob` forwarded an 18MB SPA bundle to storage, reported
`forwarded_to_storage: true`, and the blob was absent from storage's blob
store minutes later (three sibling blobs 2.7-14MB from the same run
survived; storage accepts an 18MB direct `PUT /blob/{hash}` fine — 201 +
readback 200). Root cause NOT established (forward timeout is 30s; suspects:
mid-transfer connection drop misread as success, or a post-write eviction).
A read-back verification now gates the `forwarded_to_storage` flag (truthful
reporting — this session), but the loss mechanism needs RCA before the seed
path is trusted for large bundles.

## Open sibling 3 — upstream breaker flap on `localhost` resolution

The doorway's upstream breaker to a HEALTHY storage repeatedly opened
(errorStreak 11, lastGood null) while `curl` against the same URL succeeded.
This container resolves `localhost` → `::1` first and elohim-storage binds
`0.0.0.0` (v4-only). Hypothesis: some doorway client paths fail on the v6
attempt without v4 fallback. The mesh now passes explicit `http://127.0.0.1`
storage URLs (hc-mesh.sh); if the flap disappears, the durable fix is either
v6 binding on storage's HTTP listener or a documented convention: storage
URLs in doorway config are IP-literal. Falsifier: breaker reopens on
127.0.0.1 URLs → hypothesis wrong, RCA continues.

## Verification hooks

- Mesh restart with doorway deliberately started first: routes self-register
  (proven), SSR materializes within one retry window (open), no breaker flap
  (open).
- `GET /admin/self-healing` upstreams show `circuit: closed`, `lastGood` set,
  within 60s of a full-mesh cold start.
