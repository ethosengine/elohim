---
name: project_household_space_partition_blocks_and_round_deadline
title: Household space partition blocks
description: "One rejected op makes holochain 0.7 block the author's cell forever (no unblock) → storageArc null on every peer, no authorities, held views local-only; bites when ONE space reds"
metadata: 
  node_type: memory
  title: "Household space partition: 0.7 blocks + 15 s round deadline"
  type: project
  originSessionId: bf90213f-876c-4014-807d-504fb20fefd3
  modified: 2026-09-05T12:20:40.965Z
---

**Symptom shape (2026-09-05, household mesh, node-registry space only):** `dumpNetworkMetrics`
for the space shows `completed_rounds: null` and `peer_timeouts` rising on every pair, every
peer's `storage_arc` null while the other spaces are full-arc; a courier's
`export_held_records` comes back `view: "local-only"` (stale) because `get_agent_activity`
with `GetOptions::network()` returns the empty NoPeersForLocation shape; `addAgentInfo` of
fresh signed infos is accepted but silently dropped (kitsune2 `mem_peer_store.insert` drops only
blocked / expired / older).

**PROVEN 2026-09-05 (hc-dbtool on the conductor dbs):** the rejected ops were `CapGrant` Creates written AFTER Station 8's `seal_close` on the household's real v1 chains ("No more actions are allowed after a chain close") — the grant every client mints with `authorizeSigningCredentials` (storage `hc_client.rs:293-327` on every reconnect; a2o `connectRoleConductor`). Close is not self-enforcing on the author, invalid on every neighbour → permanent block. Unblocking re-earns it on the next write; the only mesh cure is fresh cells (rebuild). Fixture rule: never close a reusable cell (Task 31); storage rule: never re-authorize on a closed cell (Task 32).

**Two causes, layered:**
1. **Blocks (the one that stands):** holochain 0.7 `integrate_dht_ops_workflow.rs:101-118`
   blocks the AUTHOR'S CELL from now to `Timestamp::max` when one op it authored is integrated
   as invalid (`CellBlockReason::InvalidOp`). 0.7 exposes NO unblock (no admin call, no HDK host
   fn); `BlockSpan` rows live in the passphrase-encrypted `conductor.db` (mesh passphrase `test`,
   key via `holochain_data::DbKey::load`). One rejected op per author partitions a household
   forever. Cure = epic Task 30 `hc-dbtool` (`blocks` / `rejected --dna` / `unblock` with the
   conductor stopped) + `hc-mesh.sh blocks <peer>`.
2. **Round deadline:** `hc-mesh.sh patch_mesh_gossip_config` set 1 s initiate intervals but left
   kitsune2 `roundTimeoutMs` at the 15 s default (a whole-round wall clock); a ~10k-op space
   cannot finish its first Accept in 15 s. Fixed 86acd1926: `roundTimeoutMs 60000` +
   `maxConcurrentAcceptedRounds 4` (same as the edge household profile). Alone it did NOT unstick
   the space (5 h, still 0 completed rounds) — it was second-order.

**How to apply:** when a space reds this way, read `dumpNetworkMetrics` per space FIRST
(elohim-storage already fetches it: `hc_client.rs:654`), then `hc-mesh.sh blocks <peer>`; never
force full arcs, never delete `p2p-peer-meta`, never re-key. A conductor restart does not clear a
block. `agentInfo` admin returns the peer store WITHOUT the blocked agents, so "self only" in one
space is the tell. See [[project_holochain_evolution_epic]] (Task 29 label, Task 30 tool),
[[project_conductor_arc_resources]], [[project_local_mesh_binary_slot_and_restart]] (SIGINT → ark
respawn ≈ 50 s; storage-restart after, tokens re-mint).
