---
id: "backlog-shard-level-inventory-gossip"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Shard bitfield inventory: a peer advertises each composite ONCE with a bitfield of the shard indices it holds (not N shard addresses), so the swarm sources shard N from a shard-only holder at composite gossip cost"
slug: "shard-level-inventory-gossip"
written: "2026-08-23"
author: "fable-5 fork 2026-08-23 (operator-requested Codex queue — sharded blob distribution)"
status: "wip"
priority: "high"
area: "dataplane/blob-swarm"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:blob-durability"
  - "habit:dataplane-convergence"
cites:
  - "doorway-federated-continuity-roadmap | Doorway-federated continuity | sha256:4c661dbbb6927763 | path: genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md"
  - "swarm-curve-and-blind-custody-design | The swarm curve and blind custody | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
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

**Red-team gate (2026-08-23, spec §3.1 / §12).** This row does NOT ship first. Prerequisites:
S0-a (reassembled composite re-hashed against `blob_hash`), S0-b (push wire carries
`manifest_cid` + `shard_index`; receiver persists a `shard_locations` membership row — without it a
shard-only holder has nothing to fold), and the minimum gossip-auth fix (bind the applied
`peer_id` to the propagation source, `gossip_dispatch.rs:327-352` — today the body-claimed id is
trusted, so hints would be forgeable under a victim). The fold is a SUBSTITUTION (one composite
entry replaces the shard addresses; the snapshot set stays stable — the composite is never on disk
under its own name and a full-replace snapshot would otherwise erase it), the folded entry is
non-droppable under the 3.5 KB page budget, the bitfield is merged by bit-OR `UPDATE` (the three
`replace_into` writers in `db/peer_blob_inventory.rs` must not touch it), `content_fingerprint`
covers it, and rows born from a bitfield claim carry `source='gossip-bitfield'` which custody /
salvage exclude from honored-replica counts.

## Scope (elohim-storage only) — frozen wire shape, see spec §3.1
1. `BlobHint` (`p2p/inventory_gossip.rs`) gains `shards_held: Option<Vec<u8>>` (LE bit i = shard
   index i of the manifest at `address`) and `encoding: Option<String>` — additive,
   `#[serde(default, skip_serializing_if = Option::is_none)]`; old peers ignore it.
2. Broadcaster (`inventory_broadcaster.rs` `build_snapshot` / `build_delta` + `gather_hints`):
   a `LocalInventory` layer folds shard files under their manifest (via the S0-b membership rows
   and `shard_manifests`) and emits ONE composite entry IN PLACE of them. A holder with the
   composite bytes sets all bits. `build_bounded_inventory_publications` pages a folded entry,
   never strips its hint.
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

## Live evidence (2026-08-23, orchestrator on the owned 3-peer mesh)

Codex landed the implementation as `4009362f0` (gate green, 2,968 lib tests) without a
live mesh. Re-verified here: storage rebuilt at `4009362f0` with `p2p-iroh`, all three
peers re-exec'd in `dual`; `features/resilience/app-blob-heal-on-read.feature` 2/2
(first-request heal races peers for locally-missing bytes and serves; the >64 MiB RS
artifact ingests and serves whole) and `features/dataplane/doorway-failover.feature`
10/10 as the regression guard. Not yet measured: the curve itself (roadmap S3 — fetch
one RS blob with 1/2/3 holders and assert wall-clock falls) and a parity-shard-missing
heal through the doorway; those are the evidence that flips blob-durability, so this
row moves to `wip`-verified, not `done`.
