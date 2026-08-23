---
id: "backlog-shard-level-inventory-gossip"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shard bitfield inventory: a peer advertises each composite ONCE with a bitfield of the shard indices it holds (not N shard addresses), so the swarm sources shard N from a shard-only holder at composite gossip cost"
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
  - genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md
tags: [dataplane, blob-swarm, inventory, gossip, bounded-feature, codex-claimable, agent-agnostic]
---

# Shard-level inventory gossip

**Correction (design pass 2026-08-23, spec §1).** The premise below was half wrong: shards ARE
already gossiped — every shard is stored as its own blob (`http.rs` "Store each shard") and the
broadcaster's `current_hashes` walks the blob dir, and `fetch_shards_via_swarm` already does one
`lookup_hosts` per shard (`blob_swarm.rs:148-158`). The gap is SHAPE, not population: N flat
`sha256-` addresses per blob (64 for a 64 MiB `chunked` blob), receivers cannot tell a shard from
a composite, and oversized snapshots hit the gossip frame limit and silently stop advertising
(`MessageTooLarge` on `elohim/inventory/blob` on the household mesh). The design decision is in the
spec, §3.1 — this row is now its implementation row (S1′).

**Why.** Bandwidth compounds with holders only if a requester can find the peer that holds *just*
shard 3 and know it is shard 3 of manifest M — at a gossip cost that does not grow with shard count.

## Scope (elohim-storage only) — frozen wire shape, see spec §3.1
1. `BlobHint` (`p2p/inventory_gossip.rs`) gains `shards_held: Option<Vec<u8>>` (LE bit i = shard
   index i of the manifest at `address`) and `encoding: Option<String>` — additive,
   `#[serde(default, skip_serializing_if = Option::is_none)]`; old peers ignore it.
2. Broadcaster (`inventory_broadcaster.rs` `build_snapshot` / `build_delta` + `gather_hints`):
   fold shard files under their manifest (via `shard_manifests` rows) before emitting — the
   composite address carries the hint; shard addresses stop being emitted individually once the
   fold is in. A holder with the composite bytes sets all bits.
3. Receive side: `peer_blob_inventory` gains `shard_bitfield BLOB NULL` (one migration; source of
   truth: local/operational — rebuilt from the next snapshot). Both gossip legs dispatch through
   the same `handle_gossip`, so a payload change covers libp2p and iroh.
4. `fetch_shards_via_swarm` derives `per_shard_hosts` from bitfields first, per-shard rows as the
   transitional fallback.

## DoD / verification
- Unit/integration test: peer B holds only shard 2 of a 3-shard manifest and publishes a snapshot
  with ONE composite entry whose `shards_held` has only bit 2 set; on peer A, `plan_shard_holders`
  puts B first for shard 2 and not for shards 0/1. A snapshot for a 64-shard blob is one hint, not
  64 addresses (assert the frame size).
- Mixed-version test: a snapshot without hints (old peer) still populates per-shard rows; a
  receiver without the column ignores the hint.
- `cargo test --test sync_libp2p_convergence` still green; `just gate elohim-storage` →
  `GATE_EXIT=0` echoed; `just test-iroh` green (gossip parity tests).
- Household-lane proof is the orchestrator's: `just test mesh features/dataplane/content-sync.feature`
  and `features/resilience/app-blob-heal-on-read.feature` show no new red.
- Commit path-limited (`git commit -m "…" -- <paths>`); never `--amend`.

## Disjointness
`inventory.rs`, `gossip_dispatch.rs`, `db/peer_blob_inventory.rs`, tests. Do not touch
`blob_swarm.rs` (sibling item `swarm-parity-aware-completion` owns it) or `p2p_iroh/` transport code.
