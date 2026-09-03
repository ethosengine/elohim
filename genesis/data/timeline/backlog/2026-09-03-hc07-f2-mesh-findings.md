---
id: "backlog-hc07-f2-mesh-findings"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Holochain 0.7 F2 gate — findings from the first household-mesh runs (storage follow-ups, hash-neutral; land after the cutover)"
slug: "hc07-f2-mesh-findings"
written: "2026-09-03"
author: "holochain 0.7 upgrade, Lane F2 (household mesh on stock 0.7.0 + local iroh-relay 1.0.3)"
status: "open"
priority: "high"
tags: [holochain-0.7, elohim-storage, household-mesh, lane-f, hash-neutral, codex-claimable]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
---

# F2 findings — what the 0.7 household mesh showed that is NOT the conductor line

Measured 2026-09-03 13:47–14:55Z: 3 ark conductors (stock holochain 0.7.0), storage `dual`, local
iroh-relay 1.0.3, prologue + full Act I (`just test mesh`: 278 scenarios — 203 passed, 31 failed,
7 pending, 37 skipped). Connectivity gate PASSED (2 direct connections per conductor through the relay;
cross-conductor DHT heal observed). Each item below is a separate, hash-neutral fix; none belongs in the
mechanical 0.7 batch. Baseline caveat: there is NO full-lane 0.6 household report since 2026-08-29 — the
tevah/ark session is running one (with a per-role `/health` probe) so each item can be tagged
`0.7-delta` or `standing`.

## 1. Stale `Arc<HcClient>` after a conductor restart (storage) — likely standing

`hc_client_registry.rs:~370-395` re-mints a role's handle after "conductor bridge is DEAD" and publishes it
through `RwLock<Option<Arc<HcClient>>>` slots. But the long-running reconcile tasks take `hc: &Arc<HcClient>`
captured at spawn (`services/head_adoption.rs:1375,1680,1726,1886,1950,2380,2738`, spool custody tick,
provide reconcile, projection-reconcile batch resolve) and never re-read the registry. After the lane's
mesh-wide conductor SIGTERMs (14:07, 14:20) james's `lamad` role showed `zomePath: dead` for 30+ min —
`Websocket closed: No connection` on ~8400 election reads / 15 min — while the HTTP zome routes (which
call `registry.client(role)` per request) were live. jessica: same for `node_registry`. Effects seen:
household participants projection stuck at 1/2/2 rows across peers (Act I "All three members are affirmed
participants" red), reanchor_backfill never healing, custody ticks failing.
**Fix:** every sweep resolves its client via `registry.client(role)` at the top of the pass (or the registry
hands out a thin handle that dereferences the slot per call); add a `/health` tripwire: a role with
`consecutiveFailures > N` and `bridgeReconnects` unchanged for 2 min is a stale holder, log the task name.

## 2. `/db/p2p/conductor-diagnostics` `transportStats` cannot serialize on 0.7 — 0.7-delta, trivial

`http.rs:~7090`: `serde_json::to_value(&stats)` → `{"serializeError":"key must be a string"}` because 0.7's
`blocked_message_counts` is a map keyed by DnaHash (non-string key). Read the raw shape via the admin port
(`dump_network_stats`): `transport_stats{backend, peer_urls[], connections[{pub_key, send/recv counts,
is_direct, opened_at_s}]}` + `blocked_message_counts{url → {dna_hash → {incoming, outgoing}}}`. Stringify
the keys; expose `connections.len()` as a first-class field — it is the F2/F5 relay gate.

## 3. node-registry refuses `ShardAssignment shard_index 7..19 exceeds maximum 6 (4+3 RS encoding)` — verify on 0.6

13 refusals per peer at boot ("Failed to register shard assignment with Node Registry"). Storage assigns
shard indices beyond the integrity zome's 4+3 ceiling. Either the storage shard planner or the zome bound
is stale; decide from the design doc, then fix the one that is wrong.

## 4. 0.7 conductor: `database is locked` (SQLITE_BUSY 5/517) in `integrate_dht_ops_consumer` — 0.7 watch item

50–103 per conductor over the hour, 7–15 per 15 min AT REST. `holochain_data` (sqlx, WAL) has no visible
busy handler where 0.6's `holochain_sqlite` had `ACQUIRE_TIMEOUT_MS` 10 s / 30 s pools. Workflows retry, so
nothing is lost, but integration is slower and the log is noisy. Compare the 0.6 count from the baseline
run; if 0.7 is materially higher, it is an upstream issue (holochain_data pool `busy_timeout`) and a
candidate fork patch beside the sys-validation backoff.

## 5. Storage P2P: `blob fetch response too large: 30252712 > 16777216` — standing

T21 outbound blob fetch over the libp2p/iroh request-response codec caps responses at 16 MiB; a 30 MB blob
(lamad-spa/landing bundles) can never be pulled this way and the fetch retries every ~10 s forever. Either
chunk the response or route >16 MiB through the blob stream path. Act I "A pin completes only when verified
bytes land on disk" is probably this.

## 6. Apparatus, fixed on the branch (for the record)

- `epr-release-package.ts` stats `elohim/holochain/dna/elohim/workdir/elohim.happ` under the REPO ROOT the
  a2o run executes from — a worktree that installs via `MESH_HAPP_PATH` must also stage `workdir/` (done).
- Seeder post-flight raced asynchronous anchoring (fixed: bounded wait, 7df163202); `@holochain/client`
  override moved to `pnpm-workspace.yaml` (49568f756); release-ceremony driver failures print stdout+stderr
  (c7c458de2); the conductor-spin detector reads `.sandbox_run_log`, which ark mode never writes
  (apparatus: point it at `<peer>/ark/logs/conductor.stderr.log`).
