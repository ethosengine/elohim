---
id: "backlog-placement-keeps-data-shards-diverse"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Diversity-aware placement distinguishes data shards from parity shards (derived from the manifest — no registry or DNA field) and keeps the data shards household-diverse first"
slug: "placement-keeps-data-shards-diverse"
written: "2026-08-23"
author: "fable-5 fork 2026-08-23 (operator-requested Codex queue — sharded blob distribution)"
status: "wip"
priority: "medium"
area: "dataplane/custody-placement"
domain: "protocol"
jobs: [elohim]
relatedNodeIds:
  - "habit:blob-durability"
cites:
  - genesis/docs/superpowers/plans/2026-08-23-doorway-federated-continuity-roadmap.md
  - genesis/docs/superpowers/specs/2026-06-24-blob-custody-phase3-xor-salvage-placement-design.md
tags: [dataplane, custody, placement, erasure-coding, bounded-code-fix, codex-claimable, agent-agnostic]
---

# Data-shard-first diversity placement

**Why.** `ShardAssignment` (`elohim-storage/src/node_registry_api.rs:28-38`) carries `shard_index`
and `strategy` but no data-vs-parity role, and `DiversityAwarePlacementStrategy`
(`reconcile/placement.rs:170+`, live default via `SALVAGE_DIVERSITY_PLACEMENT`) treats every shard
alike. For `rs-4-7` the 4 data shards are what a fast read needs (parity is insurance); losing a
household that held 2 data shards hurts more than one that held 2 parity shards.

## Scope (elohim-storage only; NO new registry field, NO DNA change)
1. Role is DERIVED: for an erasure-coded manifest, `shard_index < data_shards` ⇒ data, else parity
   (`ShardManifest` already carries `data_shards`/`total_shards`). Add a pure helper
   `shard_role(manifest, index) -> ShardRole` in `sharding.rs` with a unit test; do not add a field
   to `ShardAssignment` (that is a Node Registry DNA-facing shape — out of bounds here).
2. `DiversityAwarePlacementStrategy::choose` (or the salvage caller in `reconcile/custody.rs:452`)
   places data shards across distinct households FIRST, then parity shards, so the data set is the
   most diverse subset; XOR remains the tiebreak. Chunked/none manifests unchanged.
3. One structured log line per salvage pass: `data_shards_diverse_households`, `parity_shards`.

## DoD / verification
- Unit tests: rs-4-7 manifest, 3 households × 2 peers → the 4 data shards land on ≥3 distinct
  households before any parity shard is placed; chunked manifest → behavior byte-identical to today
  (existing placement tests stay green).
- `just gate elohim-storage` → `GATE_EXIT=0` echoed on its own line.
- Commit path-limited (`git commit -m "…" -- <paths>`); never `--amend`.

## Disjointness
`sharding.rs` (helper + test only), `reconcile/placement.rs`, `reconcile/custody.rs` salvage call
site. Do not touch `http.rs`, `blob_swarm.rs`, `inventory.rs`, `node_registry_api.rs`, or the DNAs.

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
