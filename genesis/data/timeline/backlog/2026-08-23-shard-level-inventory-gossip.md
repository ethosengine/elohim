---
id: "backlog-shard-level-inventory-gossip"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shard-level inventory: a peer advertises the shard hashes it holds, so the swarm can source shard N from a peer that holds ONLY shard N"
slug: "shard-level-inventory-gossip"
written: "2026-08-23"
author: "fable-5 fork 2026-08-23 (operator-requested Codex queue — sharded blob distribution)"
status: "refined"
priority: "high"
area: "dataplane/blob-swarm"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:blob-durability"
  - "habit:dataplane-convergence"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
tags: [dataplane, blob-swarm, inventory, gossip, bounded-feature, codex-claimable, agent-agnostic]
---

# Shard-level inventory gossip

**Why.** `blob_swarm.rs:46-49` says it: per-shard inventory "is not universally populated — the
common case", so `plan_shard_holders` falls back to rotating the COMPOSITE's holder set. A peer
that salvaged or was pushed only shard 3 (custody push is shard-granular: `p2p/mod.rs:1852
push_shard`) is invisible to the swarm as a source for shard 3. That is the difference between
"availability rises with holders" and "bandwidth compounds with holders".

## Scope (elohim-storage only)
1. Inventory snapshot (`inventory.rs`, the `T22` publisher) gains the shard hashes the peer holds
   for sharded manifests — additive, bounded: publish shard hashes only for manifests whose
   encoding != "none", and cap the snapshot delta so it stays under the gossip frame limit
   (the log shows `MessageTooLarge` on `elohim/inventory/blob` on the household mesh today — if the
   full snapshot already exceeds the limit, this item must chunk the snapshot rather than add to
   an oversized frame; say so in the commit if you hit it).
2. Receive side (`gossip_dispatch` → `db/peer_blob_inventory`): record `(peer, shard_hash)` rows so
   `lookup_hosts(shard_hash)` answers for shards — the function `fetch_shards_via_swarm` already
   calls (`per_shard_hosts`). No new table if `peer_blob_inventory` can key a shard hash like a
   blob hash (it is content-addressed either way); prefer that.
3. Must work on BOTH gossip legs: `DualGossipPublisher` + `p2p_iroh/gossip_receive.rs` dispatch
   through the same `handle_gossip`, so a transport-neutral payload change covers both.

## DoD / verification
- Unit/integration test: peer B holds only shard 2 of a 3-shard chunked manifest and publishes
  inventory; on peer A, `plan_shard_holders` puts B first for shard 2 and not for shards 0/1.
- `cargo test --test sync_libp2p_convergence` still green; `just gate elohim-storage` →
  `GATE_EXIT=0` echoed; `just test-iroh` green (gossip parity tests).
- Household-lane proof is the orchestrator's: `just test mesh features/dataplane/content-sync.feature`
  and `features/resilience/app-blob-heal-on-read.feature` show no new red.
- Commit path-limited (`git commit -m "…" -- <paths>`); never `--amend`.

## Disjointness
`inventory.rs`, `gossip_dispatch.rs`, `db/peer_blob_inventory.rs`, tests. Do not touch
`blob_swarm.rs` (sibling item `swarm-parity-aware-completion` owns it) or `p2p_iroh/` transport code.
