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
mechanical 0.7 batch. Note (17:00Z): the 10:20Z pod swap that ended run 2 was a k8s-dqlite crash-loop artefact (stale secretless ReplicaSet), not the mesh's writes — see the guide's corrected F2 disk note. Baseline caveat: there is NO full-lane 0.6 household report since 2026-08-29 — the
tevah/ark session is running one (with a per-role `/health` probe) so each item can be tagged
`0.7-delta` or `standing`.

## 1. Stale `Arc<HcClient>` after a conductor restart (storage) — STANDING (0.6 calibration 17:35Z: same shape — jessica/james lamad + node_registry `dead` after the lane's restarts, matthew live; tevah/ark session's per-role probe)

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

## 4. Conductor `database is locked` (SQLITE_BUSY 5/517) in `integrate_dht_ops_consumer` — NOT a 0.7 delta (0.6 calibration 17:35Z under the same lane churn: matthew 5, jessica 452, james 543 since 16:32Z — an upper bound far above 0.7's 7–15 per 15 min at rest)

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

## 6. Storage: an EMPTY `AGENT_PUBKEY` is taken as a key — standing, hash-neutral

`main.rs:~5684` builds the release-adoption `SoakAttestor` with `args.agent_pubkey.unwrap_or_else(|| hc.agent_key_uhcak())`;
an env of `AGENT_PUBKEY=""` is `Some("")`, so the fallback never fires and every soak attestation fails
`soak_context_incomplete: deviceId is required` (station 3 red on 2026-09-03 until the mesh script stopped exporting the
empty string). Treat empty as absent at the clap/env boundary (`value_parser` rejecting "" or `.filter(|s| !s.is_empty())`)
so a peer whose launcher could not resolve the key still attests under its admin-derived key.

## 7. Edge bakes the FLOATING `elohim-happ:dev-latest` — make the fetch commit-pinned or guarded (CI)

`elohim/holochain/Jenkinsfile` `fetchHappFromHarbor` pulls `elohim-happ:dev-latest`; when edge and the DNA pipeline are
dispatched in the same wave (or edge is forced with `[build:edge]` while the DNA build is still running), edge bakes
the PREVIOUS hApp — 2026-09-03: edge #1424 started 40 min before DNA #1425 could publish the 0.7 hApp, so the first
0.7 roll carried the 0.6 bundle and had to be re-dispatched. Fix: pull `elohim-happ:1.0.0-dev-<short-sha>` for the
edge build's own commit first and fall back to `dev-latest` only with a loud WARN; then verify the fetched bundle's
`hc dna hash` per role against `elohim/holochain/dna/dna-hashes.baseline` and FAIL on mismatch unless the tip carries
`[dna:migrate]` (the same rule the DNA pipeline's guard enforces). The memory trap "a same-wave dispatch bakes the
PREVIOUS happ" becomes a build failure instead of a silent fleet roll. Orchestrator half of the same defect
(run #1801): the plan printed `[elohim-holochain + elohim] → elohim-edge`, but elohim-holochain is triggered
fire-and-forget (longRunning) so edge started the moment the app build ended, and edge's failure then CASCADED an
abort into the DNA build's sweettest shards (#1425 ABORTED, hApp never published). Edge must wait for a
holochain build that is in its own plan, and a downstream failure must not cancel an upstream long-running build.

## 8. CI: sweettest shard pods evicted for ephemeral storage on the 0.7 line — fixed (6ce1fff69), watch it

Every DNA build since the line moved (#1424, #1425, #1426) lost shards 1-3 to kubelet eviction ("Pod ephemeral local
storage usage exceeds the total limit of containers 8Gi") mid-run, so only shard 4's tests ever counted and #1424's
"12 tests run" was one shard, not the matrix. Each shard pod materialises the holonix main-0.7 closure (505 store
paths) in its writable layer on top of the 945 MB nextest archive and its extraction. Shard limits raised to
12Gi/24Gi. Related: `elohim/holochain/dna/elohim/flake.lock` still pins holonix `main-0.6` (2fec8bf) while
`flake.nix` says `main-0.7`; nix re-resolves it at runtime in every CI pod ("updating lock file … → ffcc7c6
2026-07-30") — the operator regenerates the lock with nix and commits it (Lane E7's stated path); until then the
resolution is repeated per pod and can drift.

## 9. CI: storage quality gate's `cargo test --lib` panics on missing fixtures in the Docker context — STANDING

`elohim/elohim-storage/src/services/release_adoption/verify.rs` tests read
`../../genesis/a2o/scripts/__tests__/fixtures/*.json` and `../rakia/schemas/v1/release-manifest.schema.json` by
relative path; the storage Dockerfile's build context copies neither, so the `check` stage's `cargo test --lib`
panics ×20 ("fixture … unreadable") in every edge build — #1423 (0.6, before the cutover) and #1426 alike. It is
"non-blocking while stabilizing", so it only marks edge UNSTABLE, and the gate is measuring nothing for that
module. Fix: COPY the two fixture roots into the `check` stage (the path-dep COPY trap), or embed the fixtures with
`include_str!`.

## 10. Doorway worker pool: one auth token for many conductors — STANDING, exposed on the fresh 0.7 fleet

doorway-alpha-b's pool dials every peer in `CONDUCTOR_URLS` (the seven conductor pods' 8445) but mints its app auth
token on `CONDUCTOR_ADMIN_URL` (adam's conductor only); a token minted by one conductor is invalid on another, so each
foreign session is accepted then closed ("Conductor closed connection: None" ×N/min, workers flapping 0↔2). Tokens
are per-conductor: the pool must mint on each target's admin (`conductorWsHost(...):8444`) or restrict its workers to
the primary. Separately, both doorways' `--conductor-url` still named the pre-split storage host (fixed in the
manifests, 2026-09-04).

## 11. First re-seed of a fresh fleet (genesis #1553, 2026-09-04 03:xxZ) — what the pipeline still assumes

- `Verify Target Health` gated on the site root, which is 503 until the seed exists (fixed: `/health`, 83ae73bda).
- `Seed Conductor Identities` reported `[C] Conflict` for matthew: his conductor already embodied a UUID human id
  (`5f27bc9b-…`) before the seeder reached it — something creates a Human on first contact with an empty conductor
  (doorway A's zome caller or storage's identity fill). One agent = one Human, so on a fresh genesis the seeder must
  run before any self-heal, or the self-heal must use `SELF_HUMAN_ID`. Operator attention per the seeder's own doc.
- `stakes seed adam: HTTP 403` — adam's storage runs with `ALLOW_SEED_NETWORK_STAKES` off (posture, not a bug; the
  manifest seeded on matthew + jessica).
- The landing and lamad-spa rows seed with `blobHash: null` / `serverBlobHash: null`: the SPA bundles are staged by
  the APP pipeline's "Upload SPA Blob" stage, so `/` stays 503 and `/lamad/` 404 until an app run follows the seed —
  the app run that ran before the roll (#1688) declared against the old fleet and is gone with the wipe.

## 12. Apparatus, fixed on the branch (for the record)

- `epr-release-package.ts` stats `elohim/holochain/dna/elohim/workdir/elohim.happ` under the REPO ROOT the
  a2o run executes from — a worktree that installs via `MESH_HAPP_PATH` must also stage `workdir/` (done).
- hc 0.7 removed `hc sandbox call`; the mesh script's agent-key probe now uses `hc client call --port <admin> list-apps`
  with the 0.6 form as fallback, and the storage restart overlay re-resolves the key (fcb81456d).
- Every storage peer starts with `ELOHIM_RUNTIME_CONFIG_PATH=<mesh>/<peer>/runtime-config.toml` so the rung-4 watcher the
  ceremony writes to is armed (bb5353321); the rung-5 baseline pair (bundle N + content_store WasmHash) is repinned to the
  0.7 build and env-overridable (236efbb4c).
- Seeder post-flight raced asynchronous anchoring (fixed: bounded wait, 7df163202); `@holochain/client`
  override moved to `pnpm-workspace.yaml` (49568f756); release-ceremony driver failures print stdout+stderr
  (c7c458de2); the conductor-spin detector reads `.sandbox_run_log`, which ark mode never writes
  (apparatus: point it at `<peer>/ark/logs/conductor.stderr.log`).
