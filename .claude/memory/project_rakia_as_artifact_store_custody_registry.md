---
name: project_rakia_as_artifact_store_custody_registry
title: rakia = package manifest over the dataplane store
description: "2026-09-25: the p2p dataplane IS the store; rakia adds none — it is the package manifest giving blobs meaning over the storage SDK; latest = channel head."
metadata:
  node_type: memory
  type: project
  originSessionId: 184fb351-36f9-402f-b933-974a3cf7b37c
  modified: 2026-09-25T15:10:32.013Z
---

**Operator steering 2026-09-25 (compute-agreements sprint), corrected the same day:** rakia should fill the repository / registry (Nexus / Harbor) role — but *the store itself is not unbuilt*: the p2p dataplane supplies it (elohim-storage `BlobStore`, blob protocols over iroh/libp2p, `peer_blob_inventory`, `replicates-content` / `custody-blob` commitments, ≤1 MiB CIDv1 chunks). Rakia is the **installation / package manifest** that supplies meaning and relationships to particular blobs (this CID is `compute-executor` from step X, depends on Y, supersedes Z, pinned by channel `dev`), as a composition of the storage SDK. Seam: what you add is a manifest → **SDK seam** (seam map §3.5). Rakia = רָקִיעַ, the firmament: peers are stars, the build graph is their constellation, a blob at its CID is a star, a channel head is the course a dependency follows.

**Ruling R12 (plan `/projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md`):** wired = chunked, leased, content-addressed transfer between peers + inventory + custody schemas; composition to write (SDK-side, small dataplane additions at most) = (a) a manifest artifact entry naming a blob CID with relationships, (b) persistent custody minted for build outputs (not task-scoped leases), (c) `brit build put` with the artifact CID as `output_cid` (today a run-record CID), (d) multi-source chunk fetch (`materialize` takes one base), (e) channel-head resolution — "latest" is a declared head, never recency. lvi Track B stays deferred and is this composition.

**Why:** the executor binary vanished from the cargo pool on the 2026-09-24 pin move and rung H had to rebuild it from a stash; as a rakia manifest entry over the dataplane, any household peer would serve it. **How to apply:** never propose a new artifact plane, registry service or "store"; name built artifacts by CID and record sha256/CID in receipts now; design the registry as a rakia manifest composition over the storage SDK (missing node `artifact-manifest-entry`, chain build-step → artifact-fetched-by-peer). Related: [[project_rea_compute_commitment_primitive]], [[feedback_upgrade_propagation_north_star_wall_clock]], [[project_lvi_devspace_peer_runtime]], [[project_head_reach_freshness_semantics]], [[project_inventory_exchange_not_byte_replication]].
