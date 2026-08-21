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

## Fixed (2026-08-21) — conductor role-map discovery was boot-once

Same shape as the route-registry fix, one layer down. `hc-mesh.sh start_all`
starts doorways at step 1 and conductors at step 2, so
`spawn_discovery_task_with_signal` (`services/discovery.rs`) woke at boot+2s,
found no admin interface, logged `Discovery completed with issues … readiness
NOT signaled` — and the task **exited**. `zome_configs` then stayed empty for
the life of the process, so `get_zome_config_by_role`
(`routes/zome_helpers.rs:169`) answered `No zome config found for role
'imagodei'. Available: []` and every hosted registration returned
`HTTP 500 {"error":"Failed to get agent identity","code":"AGENT_KEY_ERROR"}`
until an operator restarted the doorway. Retry-only-while-the-connect-fails,
return-on-first-success is exactly the storage-side shape fixed in 77dd6b7b6.

Cured by a bounded-exponential retry (1s→30s cap) that runs until the role map
is non-empty and RETURNS on success (`discovery_verdict`, registered in
`seam-registry.yaml`). Reproduced and verified on a throwaway doorway (:8898,
conductor port closed at boot, opened later):

| | boot-once (pre-fix) | retry (post-fix) |
|---|---|---|
| discovery attempts | **1**, then `readiness NOT signaled` | 16 retries, each WARN'd with attempt + cause + next delay |
| role map, 45s after conductor reachable | **empty forever** (`discoveryComplete: false`) | `rolesDiscovered: 5` — **14s** after the conductor came up (attempt 17) |
| `conductor.connected` | `true` (hardcoded dev-mode constant) | `false` while blind, `true` once roles resolve |
| `/health/serving` | `200` | `503` while blind → `200` on discovery |
| after success | — | retry count stays 16: the self-heal does not become its own flap |

`/health` also stopped lying: `conductor.connected` was a hardcoded `true` in
dev mode (`routes/health.rs`), so the one field the struct's own docs tell the
seeder to check before seeding could not report the single condition that makes
seeding fail. It is now gated on the role map, a new `conductor.rolesDiscovered`
carries the count, and `/health/serving` 503s while the map is empty. Read
replicas (`--projection-writer=false`) never run discovery and are never judged
conductor-blind.

## Open sibling 4 — worker app-socket reconnect flap (~10s cycle, never quiets)

Distinct from the discovery gap above and NOT fixed by it. After discovery
succeeded on the throwaway, the four pool workers still cycled
**18× in 40s** on `ws://localhost:4445`:

```
Connecting to conductor at ws://localhost:4445
Connected to conductor
Conductor closed connection: None      <-- stream end, no close frame
Reconnecting to conductor in 100ms...
```

Measured session lifetimes: 2.4s and 7.5s — both under
`STABLE_SESSION_THRESHOLD` (10s, `worker/conductor.rs:52`), so the ladder never
settles into a quiet cadence, and the observed redial stays at the 100ms floor.
The doorway sends nothing on the app socket between zome calls (no ping /
keepalive), so the leading hypothesis is a conductor-side idle reap.
Root cause on the conductor side NOT established — the cure (a keepalive on the
app socket, or explicit idle-close tolerance) needs the conductor's idle-timeout
value and touches `handle_messages`, which risks regressing the auth-reject
detection path (`AUTH_ACK_WINDOW`, 500ms). Falsifier: with a keepalive the
`Conductor closed connection: None` line disappears; if it persists, the close
is not idleness.

Rate on the live mesh for scale: ~97 reconnects/2min (pre-fix throwaway),
~62/2min (post-fix throwaway — unchanged by the discovery fix, as expected).

## Open sibling 5 — the app-interface port is fixed, not per-conductor

`derive_app_url(conductor_url, args.app_port_min)` (`main.rs:1850`, called at
`main.rs:253`) replaces the port in the admin URL with the **fixed**
`--app-port-min` (default 4445, `config.rs:81`) regardless of which conductor
the admin URL names. Doorway B on the local mesh has admin `ws://localhost:4454`
(conductor 1) and therefore dials **conductor 0's** app interface at 4445,
authenticating with a token minted by conductor 1's admin. Live evidence
(doorway-b.log, one boot): 44× `Failed to authenticate with conductor: Holochain
error: Conductor closed connection during authentication`, 12× `Re-minted app
auth token after unstable conductor session`, backoff climbing 100ms→30s, and
`/health` `connected_workers: 0` with `pools_healthy: 1`.

Two candidate cures: pass `--app-port-min` per doorway in `hc-mesh.sh` (B needs
4455), or derive the app port from the admin port rather than a global default.
The second is the durable one — a fixed app port is only correct for a
single-conductor host.

## Verification hooks

- Mesh restart with doorway deliberately started first: routes self-register
  (proven), SSR materializes within one retry window (open), no breaker flap
  (open).
- `GET /admin/self-healing` upstreams show `circuit: closed`, `lastGood` set,
  within 60s of a full-mesh cold start.
