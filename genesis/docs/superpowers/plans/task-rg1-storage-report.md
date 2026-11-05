---
title: "Re-genesis 1 T2 report: elohim-storage on the byte profile"
id: task-rg1-storage-report
status: claimed-not-verified
landed_commit: "regenesis/storage on regenesis/kernel e6adb9b43: 0a082ba74 B1, ae90f9531 B2, cce464f94 B3, efe21e7f3 B4, 4720931e1 + 2f495757f fix-ups (A: 8e1f7a8d7, ef6afcd21; B step 0/T5 adoption: 4693e4a1c, fb2386513)"
verified_by:
actor:
  - agent:storage-a@claude-opus-5-5 (A1 blob store, A2 sharding, A3 inventory, A4 trust/release verify)
  - agent:storage-b@claude-opus-5-5 (B1 migration+db, B2 http, B3 fetch+wiring, B4 tests)
cites: []
concerns:
  - eprfs_tier BlobStoreTier: 3 tests (leaf contract, dag-cbor tree batch, corrupt-bytes batch) need T3's eprfs-core (BlobCid::verifies over the byte profile); on this branch eprfs-core verifies raw CIDs as sha2-256, so the batch path refuses byte-CID leaves. Expected green after the T2+T3 merge; not verified here
  - element-registry-view.schema.json (T5) still pins cid to ^sha256- while T5's Rust doc says byte CID; schema_contract keeps the sha256 fixture for that view and flags it
  - fixture CIDs come from real bytes (the label's bytes) or kernel vectors; record-CID fixtures (mishpat ratios, trust memo digests, attestation/membership cids) were left in their record spelling on purpose
  - no epr session or actor claim could be opened from the worktree; every commit carries UNMEASURED attribution
  - intermediate commits B1-B3 do not compile alone; the series compiles at B4
---

# T2 report (elohim-storage)

Every byte address in the crate is a byte CID (`bafkr4i…`, raw 0x55 + BLAKE3-256 0x1e), admitted by
full decode (`ByteCid::parse`). There is no second spelling: no `sha256-<hex>`, no `bafkrei…`, no
dual read, no prefix validator, no persistent iroh copy. dag-cbor records keep `0x71 + sha2-256`
(`store_record`/`get_record`).

## Interfaces

### HTTP byte routes

| Route | Request | Success | Refusals |
|---|---|---|---|
| `PUT /blob/{cid}` | body = payload bytes; `Content-Type` = mime (default `application/octet-stream`) | `201 Created` (new) / `200 OK` (already held), JSON `PutBlobResponseView`, header `X-Content-Address: <cid>` | `400 {"error":"invalid-byte-cid","check","detail"}` (path); `400 {"error":"cid-mismatch","expected","actual"}` (body hashes elsewhere; nothing stored) |
| `GET /blob/{cid}` | headers `X-Agent-Cid` (or legacy `x-agent-id`) for reach | `200` bytes, `ETag: "<cid>"`, `X-Content-Address: <cid>`, `Cache-Control: public, max-age=31536000, immutable`, `Content-Type` = manifest mime | `400 invalid-byte-cid`; `403` reach refusal; `404 "Blob not found"`; `503 {"status":"syncing","blobCid","hint"}` + `Retry-After: 5` while a peer heal runs |
| `PUT /shard/{cid}` | body = unit bytes | `201`/`200`, `{"cid","byteLen","alreadyExisted"}`, `X-Content-Address` | `400 invalid-byte-cid`; `400 cid-mismatch` |
| `GET /shard/{cid}` | — | `200` verified bytes, `ETag`, `X-Content-Address`, immutable cache | `400 invalid-byte-cid`; `404 "Shard not found"` (absent or failing its CID) |
| `HEAD /shard/{cid}` | — | `200` + `Content-Length`, `ETag`, `X-Content-Address` | `400 invalid-byte-cid`; `404`; a storage failure answers 500, not 404 (enforced: src/http.rs shard_head_distinguishes_absence_from_storage_failure) |
| `GET /manifest/{cid}` | — | `200` JSON `ShardManifest` (DNA text shape) | `400 invalid-byte-cid`; `404 "Manifest not found"` |

`X-Blob-Hash` is gone everywhere (blob, shard and `/apps/{id}/_capability`, which now answers only
`X-Content-Address`). `/apps/{identifier}` treats an identifier as a content address only when it
decodes as a byte CID; anything else is a slug. `/ipfs/{cid}` and `/dag/{cid}` read dag-cbor
records from the record lane and byte CIDs from the payload store; other CIDs are 404.
`POST /db/content` and bulk refuse a `blobCid` that is not a byte CID (400). The :4273 iroh
dual-write and `http_blob_router.rs` are deleted.

### PutBlobResponseView (crates/elohim-views, byte-identical to T5 010a9e859)

```rust
pub struct PutBlobResponseView {
    pub blob_cid: String,
    pub byte_len: u64,
    pub mime_type: String,
    pub profile: BlobProfileView,
    pub chunks: Vec<BlobChunkRefView>,
    pub stripes: Vec<BlobStripeView>,
    pub reach: String,
    pub author_id: Option<String>,
    pub created_at: String,
    pub verified_at: Option<String>,
}
pub struct BlobProfileView {
    pub epoch: u16,
    pub chunker: BlobChunkerView,
    pub erasure: BlobErasureView,
}
pub struct BlobChunkerView {
    pub id: String,
    pub min: u32,
    pub avg: u32,
    pub max: u32,
    pub level: u8,
}
pub struct BlobErasureView {
    pub id: String,
    pub data: u8,
    pub parity: u8,
}
pub struct BlobChunkRefView {
    pub cid: String,
    pub len: u32,
}
pub struct BlobStripeView {
    pub index: u32,
    pub byte_len: u64,
    pub shards: Vec<BlobShardRefView>,
}
pub struct BlobShardRefView {
    pub cid: String,
    pub len: u32,
    pub index: u8,
}
```

### Wire protocols

- libp2p blob fetch: `/elohim/blob/2.0.0`, `BlobFetchRequest { cid: ByteCid }` (string serde, full decode on read); `BlobFetchResponse::{Found, NotFound, Error, Manifest(ShardManifest)}`.
- libp2p shard (and the iroh shard ALPN, same messages): `/elohim/shard/2.0.0`, `ShardRequest::{Get, Have, Push, GetManifest} { cid: ByteCid, .. }`.
- Inventory: gossip topic `elohim/inventory/blob/2` (signed delta envelopes from `inventory_broadcaster::build_delta_pages`, signed with the node's libp2p ed25519 transport key); `elohim/inventory/blob` subscribed on libp2p and iroh only to report its publishers unavailable.
- Inventory reconciliation: `/elohim/inventory-recon/1.0.0` (negentropy-v1 `ReconRequest`/`ReconResponse`, 4-byte big-endian length + dag-cbor, frame cap `RECON_PAGE_LIMIT + 256`); the periodic tick reconciles with up to 8 connected peers; a gap the reorder window cannot close calls `request_recon`; a finished session re-bases that peer's cursor (`rebase_after_recon`).
- Custody announcements: `CustodyAnnouncement { shard_cid: ByteCid, .. }` (no signature field; A3's topic).

### Migration `2026-11-06-000000_address_crossing`

Clean break; old addresses are dropped, not rewritten. `content` drops `blob_hash`, renames
`server_blob_hash` → `server_blob_cid`, NULLs `blob_cid`/`server_blob_cid`. Dropped and recreated
under `*_cid` names: `shard_locations`, `shard_manifests` (`blob_cid, byte_len, mime_type,
profile_json, chunks_json, stripes_json, reach, author_id, created_at, verified_at`),
`placement_gaps`, `peer_blob_inventory` (`peer_id, blob_cid, last_seen_at, source ∈ {gossip-delta,
recon, fetch-success, self-custody}, sequence`; no blake3/affinity), `peer_inventory_cursor`,
`custodian_shares`, `blob_arrivals`, `blob_unnamed_watch`, `projection_events`. New:
`held_envelopes(id, topic, reason, bytes, received_at)` bounded by `HELD_ENVELOPE_CAP`.

## Gate EXIT lines

All logs under `genesis/local-dev/regenesis-1/` in the rg1-storage worktree.

- `gate-T2-elohim-storage.log` — `CARGO_BUILD_JOBS=1 just gate elohim-storage` at 2f495757f: **EXIT=1**. migration-hygiene, image-includes, fmt-check, clippy (default) and clippy-iroh (`p2p p2p-iroh --all-targets`, -D warnings) pass; `cargo test` lib: `4912 passed; 3 failed; 3 ignored` — the three failures are the eprfs_tier BlobStoreTier tests named under Unresolved (cross-track with T3). Because the lib binary failed, cargo did not run the integration targets in this gate.
- `gate-T2-storage-integration.log` — `cargo test --no-fail-fast --test '*'` (default features): **EXIT=0** — api 181, blob 36, db 25, epr 54, graph 59, observation 30, p2p 67, rea 64, reach 4, render_capability_view 7, schema_contract 247, schema_contract_diesel_epr 4, search 30, views 141 passed, 0 failed (ignores pre-existing; bench_perf/iroh_planes run 0 without their features).
- iroh-feature integration (`--features "p2p p2p-iroh" --test iroh_planes`): 81 passed, 0 failed (fork log `forkB-6.log`, EXIT=0, before the kernel rebase).
- `gate-T2-rust-style.log` — `just gate rust-style`: **EXIT=0**, `elohim/elohim-storage: debt 128 -> 116` (paid down 12, touched), PASS.
- Earlier gate runs on this branch failed on infrastructure, not code: one on a missing Dockerfile COPY for the kernel tree vectors (fixed, 4720931e1), one on a fingerprint written away under it by the concurrent rg1-kernel `_gate-epr-storage` build in the same cargo slot.

## Deviations (with reasons)

- **T5 adoption.** crates/elohim-views, sdk view schemas, create-content-input schema, storage-client-ts generated, sdk domain types, the a2o release-manifest 2.0 fixtures and the rakia gitlink (f9787c9e5, object fetched from the rg1-schemas submodule store) are taken byte-identical from regenesis/schemas 010a9e859 so schema_contract and the verify fixtures run here; L2b meets identical content.
- **Transport affinity deleted** with http_blob_router.rs (column, db functions, admin route): it keyed a per-object iroh/libp2p preference on the dual-write that no longer exists.
- **Pure-iroh /blob keeps a byte path.** A4 asked for `pure_iroh_byte_answer -> NoBytePath (503)`; the iroh fetch leg reads the same BlobStore over the shard ALPN, so a pure-iroh node does have bytes to serve and heal. NoBytePath is not wired.
- **Recon responder empty reply** duplicates inventory_recon's private VERSION byte as `recon_wire::RECON_EMPTY_REPLY = [0x61]` (A's constant is private).
- **Recon is libp2p-only.** An iroh-only author's gap counts `unavailable` in the iroh bridge; no iroh recon transport exists yet.
- **FetchBudget** is applied in blob_fetch (artifact pulls use the default budget plus their own deadline); it is not plumbed through SwarmFetchParams.
- **CRDT doc key `blobHash` dropped**; the projector writes and heals `blobCid` only. "Half rows" are now rows with no blob_cid whose doc carries one.
- **Witness ingest refuses non-canonical bytes** explicitly (the old guard was the sha256 equality the crossing removed).
- **Named hunks in worker A's files:** blob_store.rs/sharding.rs `chunk_bytes(&ValidatedProfile::EPOCH_0)` (step 0); gossip_dispatch.rs custody dedup moved after the author-binding check (an unbound relay of the same bytes consumed the dedup key and shadowed the bound author's claim; A's own test caught it); inventory_broadcaster.rs tests (`plan_shard_holders` takes ByteCid; unused duplicate MockInventory and clone-on-Copy for clippy); release_adoption/mod.rs test fixtures (2.0 `profile`, real CIDs).
- **Codemod receipt** does not record that 11 `fixture-sha256`/`fixture-bafkrei` header swaps were reverted and reminted from real bytes instead; `projection_events` and `ShardAssignment.content_hash -> content_cid` were renamed too (coordinate with T1 if it reads them).

## Unresolved

- `eprfs_tier::blob_store_tier` — `the_blob_store_tier_passes_the_leaf_contract`, `a_dag_cbor_tree_node_round_trips_under_its_own_codec`, `bytes_that_fail_their_address_are_refused_and_never_returned` fail on this branch: the eprfs batch path (`TierChain::answer`, eprfs-core `BlobCid::verifies`) recomputes a raw CID as sha2-256, so it refuses every byte-CID leaf the tier returns. T3 (regenesis/eprfs, rg1-eprfs) adds `BlobCid::compute_bytes` and verifies the byte profile; these are expected to pass once T2 and T3 meet. Not verified in a merged tree.
- `tests/` integration binaries were swept by two forks under this worker (blob/bench_perf/api/reach; epr/iroh_planes/p2p/db/graph/views); their runs are in the scratchpad logs `forkA-*.log` / `forkB-*.log`. Two src defects they found are fixed in B3 (heal response keeps the manifest mime for an absent unstriped payload; the shard plane's Get consults compute payloads).
- Doorway/T6 must accept `X-Content-Address` (not `X-Blob-Hash`) from `/apps/{id}/_capability`, `/blob`, `/shard`.
- `elohim-storage/CLAUDE.md` still describes the old addressing canon (`bafkrei…` blobs, `sha256-<hex>` legacy); a managed-surface edit, left for the integrator.
