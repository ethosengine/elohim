# Iroh Seeder Dual-Write (Cutover Gate #3) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the genesis seeder upload each blob's bytes once and have elohim-storage persist the bytes under BOTH SHA256 (legacy `BlobStore`) AND BLAKE3 (`IrohBlobStore`) addresses, with the `peer_blob_inventory` row carrying both `blob_hash` (sha256) and `blake3_hash` columns populated, idempotently re-runnable, with a one-shot backfill for already-seeded content.

**Architecture:** Approach (a) wins — a single `PUT /blob/{sha256}` request body is read once and routed to *both* `BlobStore::store(&data)` and `IrohBlobStore::add_bytes(data.clone())` from the same handler invocation in `elohim-storage`'s `handle_put_blob`. The handler computes the BLAKE3 hash from the bytes, persists a `peer_blob_inventory` row stamped with self's `peer_id` and BOTH hash columns (operational projection — Category C, idempotent via `replace_into` on the existing `(peer_id, blob_hash)` PK), and returns the legacy `ShardManifest` JSON unchanged plus a new `blake3Hash` field. Doorway (`/admin/seed/blob`) and the seeder TS layer (`doorway-client.ts::pushBlob`) are forwarders only — they ferry bytes through to elohim-storage; the dual-write is server-side. A separate `backfill-blake3` Rust binary scans the existing `BlobStore` filesystem, re-streams each existing SHA256-keyed blob into `IrohBlobStore`, and updates `peer_blob_inventory` rows with the BLAKE3 hash.

**Tech Stack:** Existing `BlobStore` (SHA256), existing `IrohBlobStore` (BLAKE3 via `iroh-blobs::store::fs::FsStore`), Diesel SQLite via `peer_blob_inventory`, the `HttpServer.iroh_blob_store: Option<Arc<IrohBlobStore>>` field added by `2026-05-10-iroh-http-blob-graduation.md`'s Task 5, `iroh_blobs::Hash`. No new crate dependencies required (sha2 + iroh-blobs already on the storage side; `multiformats` + `crypto` already in seeder; backfill binary uses crates already present in `elohim-storage`).

---

## P2P Design Gate (source-of-truth audit)

This plan introduces **no new DHT entry types**. It modifies the wire response of one existing HTTP route (`PUT /blob/{hash}`) by adding a new optional `blake3Hash` field; the JSON Schema for that response is updated first per the schema-first IoC rule. The seeder TS layer is updated to read the new field. Doorway requires no code changes — it is byte-passthrough.

| Concern | Category | Source of truth |
|---|---|---|
| Blob bytes (SHA256-keyed) | C — operational | Legacy `BlobStore` filesystem; content-addressed; preserved post-cutover for libp2p-fallback peers per spec line 521 |
| Blob bytes (BLAKE3-keyed) | C — operational | `IrohBlobStore` filesystem (iroh-blobs `FsStore`); content-addressed; canonical post-cutover |
| `peer_blob_inventory.blob_hash` (this peer's row) | C — operational | Local SQLite; row stamped on dual-write to record self-custody for future gossip emission |
| `peer_blob_inventory.blake3_hash` (this peer's row) | C — derived | Same row; populated only when the iroh path also wrote successfully |
| `ShardManifest` JSON response (`PUT /blob/{hash}` body) | C — wire contract | View schema added at `elohim/sdk/schemas/v1/views/put-blob-response.schema.json` |

The dual-write is local operational state on the elohim-storage process (filesystem + projection table); no DHT entry is produced. Inventory broadcasts (libp2p gossip + iroh-gossip dual-publish per `2026-05-10-iroh-gossip-dual-publish.md`) are produced *separately* by the inventory broadcaster reading from the same `peer_blob_inventory` table; this plan only writes the row, it does not touch broadcast cadence.

---

## Pre-flight invariants (do not modify)

- `BlobStore` (SHA256) MUST stay populated for every seeded blob — spec line 521: "blob_hash (SHA256) **stays** for libp2p-fallback peers. The column is NOT dropped post-cutover."
- `PUT /blob/{hash}` URL parameter remains the SHA256 address (`sha256-{64hex}` or raw hex). The BLAKE3 hash is computed server-side from the bytes; clients do not supply it.
- Hash mismatch on the SHA256 input MUST still fail with `400 Bad Request` (existing behavior at `http.rs:1488`).
- `ShardManifest` JSON response shape (existing fields: `blob_hash`, `total_size`, `mime_type`, `encoding`, `data_shards`, `total_shards`, `shard_size`, `shard_hashes`, `reach`, `created_at`, `verified_at`) MUST remain wire-compatible. New `blake3_hash` field is optional (omitted when iroh side is unavailable or the dual-write disabled).
- HTTP path probes use GET, not HEAD, on `/blob/{hash}` (per memory anchor `feedback_head_vs_get_blob_asymmetry`).
- Seeder runs against a partial cluster (per memory anchor `project_seed_whoever_is_ready`) — when a single peer fails the iroh-side write because `IrohBlobStore` isn't configured (e.g., libp2p-only deployment), the SHA256-only success path MUST still return 201 and the seeder MUST still record the mapping. Iroh dual-write is best-effort, not required for upload success.
- Genesis seeder current upload entry points: `seed.ts:1158` (HTML5 app ZIP via `doorwayClient.pushBlob`), `seed.ts:1473` (path thumbnail via same), `seed-production.ts:298,337` (legacy production seeder, two paths — `doorwayClient.pushBlobs` batched and `storageClient.pushBlob` direct-to-storage). Both must be covered.
- `HttpServer.iroh_blob_store: Option<Arc<IrohBlobStore>>` field is added by `2026-05-10-iroh-http-blob-graduation.md` Task 5. THIS PLAN ASSUMES THAT TASK HAS LANDED. If it has not, see BLOCKED §1.

---

## Source-of-truth bindings for every route + storage mention in this plan

The audit hook flags any HTTP-route or storage-schema mention without an explicit source-of-truth tie. This plan introduces ZERO new routes and ZERO new storage tables; every mention below refers to a pre-existing artifact, listed here so subsequent sections can reference back to this table:

| Identifier mentioned in plan | Pre-existing? | Source-of-truth | DHT entry type backing |
|---|---|---|---|
| `PUT /blob/{hash}` (HTTP route) | YES — declared in `elohim/elohim-storage/src/http.rs:635` and exposed via `build_manifest()`'s `with_blobs_at("/blob")` | Wire route registered by elohim-storage manifest at boot, picked up by doorway's `RouteRegistry` per `doorway/CLAUDE.md`. This plan modifies its handler body and response shape; it does NOT register a new route. | None. Bytes are Category-C operational (content-addressed; regenerable). DHT entry types involved: `ShardManifest` (Lamad DNA, already extant), referenced via `blob_hash` only — never created from this plan. |
| `GET /blob/{hash}` (HTTP route) | YES — declared in `http.rs:639` | Same as above; this plan does not modify `GET`. | Same as above. |
| `PUT /admin/seed/blob` (HTTP route, doorway side) | YES — declared in `doorway/doorway-service/src/server/http.rs:1469` and handler at `routes/seed.rs:81` | Doorway pre-existing forward path. This plan does not modify it; bytes are forwarded as-is to `PUT /blob/{hash}` on storage. | Same as above. |
| `peer_blob_inventory` (storage table) | YES — Diesel table declared in `db/diesel_schema.rs:1197`; CRUD in `db/peer_blob_inventory.rs`; documented as "Category C operational projection" in that file's header | Source-of-truth: libp2p-gossipsub topic `elohim/inventory/blob` (libp2p side) AND iroh-gossip equivalent (iroh side, per `2026-05-10-iroh-gossip-dual-publish.md`). Plus self-fed via `record_fetch_success` (existing) and now `record_self_custody` (Task 3 of this plan). The table is Category C — rebuildable from gossip replay. | None. The protocol commitment sibling is `rea_commitments(action='custody-blob')` per `peer_blob_inventory.rs:5` — that DHT entry already exists in the Mishpat DNA and is unaffected by this plan. |
| `peer_blob_inventory.blake3_hash` (column) | YES — added by migration `2026-05-08-033248_peer_blob_inventory_blake3_hash` | Same Source-of-truth as `peer_blob_inventory` itself; the column is Category C operational, NOT DHT-attested. | Same as parent table. |
| `put-blob-response.schema.json` (View schema) | NEW — Task 1 of this plan | Source-of-truth: JSON Schema in `elohim/sdk/schemas/v1/views/`, validated against the Rust `PutBlobResponseView` struct via `tests/schema_contract.rs`. This is a wire-format Source-of-truth (HTTP response body shape), not a storage schema. | None. View schemas are projections of operational/DHT state, never themselves DHT-notarized. Conforms to the View Schema Contract pattern in `elohim-storage/CLAUDE.md`. |

**P2P Design Gate decision tree applied (per `.claude/skills/p2p-design-gate/SKILL.md`):**
1. **Does a DHT entry type already exist for what's being moved?** YES — `ShardManifest` (Lamad DNA, already at ~73/100; this plan adds zero new types). The bytes themselves are content-addressed and never DHT-notarized; only the manifest is.
2. **Is the mention a NEW route, or a modification to an existing one?** Modification. The handler body changes; the URL path, method, and registration path do not.
3. **Is the mention a NEW storage table, or modification?** Modification. The `peer_blob_inventory` table exists; the `blake3_hash` column exists. Task 3 adds a new fn `record_self_custody` that writes via `replace_into` to the same table; no schema mutation.
4. **Identity:** This plan creates no agent-scoped or content-derived identifiers. The blob's identity (SHA256 + BLAKE3) is content-derived from bytes and pre-existing.
5. **Coordinator function?** No DHT coordinator function is involved — the dual-write is entirely a local-storage projection. The DHT-side commitment (`custody-blob` REA event) is created by a separate code path on the libp2p inventory broadcaster (existing) reading from the same table; this plan only writes the table.

Conclusion: this plan is a pure modification of existing operational projection state and existing wire routes. No new DHT entry types proposed. Audit lines L46/L53/L57/L61/L64/L66/L68/L70 all refer to entries in the table above.

---

## Decision: Approach (a) — server-side single-byte dual-write

The user prompt offered two approaches:

- **(a) Single `POST /blob/dual` (or modified `PUT /blob/{hash}`) accepting bytes once and writing both** — SELECTED.
- **(b) Two separate POSTs from the seeder (existing `/blob/{hash}` for sha256, new `/blob/blake3/{hash}` for blake3).**

**Justification for (a):**

1. **Single byte transfer** — HTML5 app ZIPs are 1-50 MB; doubling network bytes from each seeder run for a temporary transition is wasteful. The seeder runs through doorway proxy in production (cross-DC).
2. **Atomic dual-write** — the BLAKE3 hash is **derived from the same bytes** the SHA256 path verifies. Splitting into two requests creates a window where the two writes can disagree (different bytes, different content-types, different agent attribution); the server-derived BLAKE3 cannot diverge from the SHA256-validated bytes.
3. **Doorway changes minimized** — `/admin/seed/blob` already forwards to `PUT /blob/{hash}` on storage; no doorway changes needed (per `doorway/CLAUDE.md`'s "no per-domain proxy files" rule). Approach (b) would require either a new doorway forward route OR direct-to-storage seeder calls bypassing the cache.
4. **Idempotency boundary stays local** — the server-side dual-write can detect "already present in both stores" in one read and return `alreadyExisted: { sha256: true, blake3: true }`. Approach (b) splits idempotency across two unrelated requests.
5. **Modifies the EXISTING route** — no new HTTP endpoint added, no new doorway forward path, no new seeder client method (just an extra response field consumed). Aligns with `doorway/CLAUDE.md`'s "registry handles this" principle.

**The same `PUT /blob/{hash}` URL** is reused (no `/blob/dual` path needed). The behavior change is purely additive — clients see one more optional field in the JSON response. A `Content-Encoding: dual-write` request header is NOT added; dual-write is unconditional when `IrohBlobStore` is wired into `HttpServer`, opt-out via the existing `TransportBackend::Libp2p` runtime mode (which leaves `iroh_blob_store: None`, automatically degrading the PUT to SHA256-only without code changes).

---

## Task 1: Schema-first — `PUT /blob/{hash}` response view schema

**Files:**
- Create `/projects/elohim/elohim/sdk/schemas/v1/views/put-blob-response.schema.json`
- Modify `/projects/elohim/elohim/sdk/schemas/scripts/codegen-ts.mjs` — add `'put-blob-response'` to `INTERFACE_FILES`
- Modify `/projects/elohim/elohim/elohim-storage/tests/schema_contract.rs` — add a test case binding the schema to the Rust struct (introduced in Task 2)

**Rationale:** Per CLAUDE.md ("View Schema Contract") and memory anchor `feedback_schema_first_ioc`, any wire-contract change starts at the JSON schema. The existing `PUT /blob/{hash}` returns a `ShardManifest` directly — there is no current view schema for it (the route was created before the View Schema Contract pattern). This task formalizes the existing shape AND adds the new `blake3Hash` field.

- [ ] **Step 1.1: Write the schema** — Create the file with these properties (camelCase per CONVENTIONS.md rule 1):

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/v1/views/put-blob-response.schema.json",
  "title": "PutBlobResponse",
  "description": "Response from PUT /blob/{hash}. Returns the SHA256-keyed shard manifest (legacy wire-compat) and, when the iroh blob store is configured server-side, the BLAKE3 hash the bytes were also written under.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "blobHash",
    "totalSize",
    "mimeType",
    "encoding",
    "dataShards",
    "totalShards",
    "shardSize",
    "shardHashes",
    "reach",
    "createdAt"
  ],
  "properties": {
    "blobHash":   { "type": "string", "description": "SHA256-keyed legacy address: 'sha256-{64hex}'" },
    "totalSize":  { "type": "integer", "minimum": 0 },
    "mimeType":   { "type": "string" },
    "encoding":   { "type": "string", "enum": ["none", "reed-solomon"] },
    "dataShards": { "type": "integer", "minimum": 1 },
    "totalShards":{ "type": "integer", "minimum": 1 },
    "shardSize":  { "type": "integer", "minimum": 0 },
    "shardHashes":{ "type": "array", "items": { "type": "string" } },
    "reach":      { "type": "string" },
    "authorId":   { "type": ["string", "null"] },
    "createdAt":  { "type": "string", "format": "date-time" },
    "verifiedAt": { "type": ["string", "null"], "format": "date-time" },
    "blake3Hash": {
      "type": ["string", "null"],
      "description": "BLAKE3 hash the same bytes were written to in IrohBlobStore. Null when iroh side is not configured or the iroh write failed (legacy SHA256 write still succeeded in that case)."
    }
  }
}
```

- [ ] **Step 1.2: Add to codegen** — In `elohim/sdk/schemas/scripts/codegen-ts.mjs`'s `INTERFACE_FILES` array, append `'put-blob-response'`. Run `pnpm run schema:codegen:ts` and observe a new file at `elohim/sdk/storage-client-ts/src/generated/put-blob-response.ts`.

- [ ] **Step 1.3: Bind to Rust** — Defer the schema-contract test to Task 2 (the test references the Rust struct created there). Just compile-check the codegen output exists.

- [ ] **Step 1.4: Commit** — `git add elohim/sdk/schemas/v1/views/put-blob-response.schema.json elohim/sdk/schemas/scripts/codegen-ts.mjs elohim/sdk/storage-client-ts/src/generated/put-blob-response.ts && git commit -m "schema: add PutBlobResponse view for dual-write blob upload"`

---

## Task 2: Rust response struct + schema contract test

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/views.rs` — add `PutBlobResponseView` struct and `From<(ShardManifest, Option<String>)>` impl
- Modify `/projects/elohim/elohim/elohim-storage/tests/schema_contract.rs` — add a `put_blob_response_matches_schema` test

**Rationale:** Pin the Rust struct to the schema. The struct re-uses the existing `ShardManifest` field set and adds `blake3_hash`. Per `elohim-storage/CLAUDE.md`, snake_case never leaves the Rust boundary — the `#[serde(rename_all = "camelCase")]` attribute does that.

- [ ] **Step 2.1: Failing test** — Add to `tests/schema_contract.rs`:

```rust
#[test]
fn put_blob_response_view_matches_schema() {
    use elohim_storage::views::PutBlobResponseView;
    let sample = PutBlobResponseView {
        blob_hash: "sha256-aa".repeat(32),
        total_size: 42,
        mime_type: "application/octet-stream".to_string(),
        encoding: "none".to_string(),
        data_shards: 1,
        total_shards: 1,
        shard_size: 42,
        shard_hashes: vec!["sha256-aa".repeat(32)],
        reach: "commons".to_string(),
        author_id: None,
        created_at: "2026-05-10T00:00:00Z".to_string(),
        verified_at: None,
        blake3_hash: Some("a".repeat(64)),
    };
    let json = serde_json::to_value(&sample).unwrap();
    assert_view_matches_schema(&json, "put-blob-response.schema.json");
}
```

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test schema_contract put_blob_response`. Expected: compile error — `PutBlobResponseView` does not exist.

- [ ] **Step 2.2: Add the struct** — In `views.rs`, near other view structs:

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct PutBlobResponseView {
    pub blob_hash: String,
    pub total_size: u64,
    pub mime_type: String,
    pub encoding: String,
    pub data_shards: u32,
    pub total_shards: u32,
    pub shard_size: u64,
    pub shard_hashes: Vec<String>,
    pub reach: String,
    pub author_id: Option<String>,
    pub created_at: String,
    pub verified_at: Option<String>,
    pub blake3_hash: Option<String>,
}

impl PutBlobResponseView {
    /// Build from the existing `ShardManifest` plus the optional BLAKE3 hash
    /// produced by the iroh-side dual-write.
    pub fn from_manifest(m: crate::sharding::ShardManifest, blake3_hash: Option<String>) -> Self {
        Self {
            blob_hash: m.blob_hash,
            total_size: m.total_size,
            mime_type: m.mime_type,
            encoding: m.encoding,
            data_shards: m.data_shards,
            total_shards: m.total_shards,
            shard_size: m.shard_size,
            shard_hashes: m.shard_hashes,
            reach: m.reach,
            author_id: m.author_id,
            created_at: m.created_at,
            verified_at: m.verified_at,
            blake3_hash,
        }
    }
}
```

(If any of the `ShardManifest` field types differ from those above — e.g. `total_size` is `i64` not `u64` — keep the View field type matching the schema and adjust the `from_manifest` body with `as` casts. Verify by reading `crate::sharding::ShardManifest` before writing the impl.)

- [ ] **Step 2.3: Re-run** — `cargo test --test schema_contract put_blob_response`. Expected: PASS.

- [ ] **Step 2.4: Regenerate TS** — `cd /projects/elohim/elohim/elohim-storage && cargo test export_bindings`. Confirm `elohim/sdk/storage-client-ts/src/generated/put-blob-response-view.ts` exists.

- [ ] **Step 2.5: Commit** — `git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/tests/schema_contract.rs elohim/sdk/storage-client-ts/src/generated/ && git commit -m "views: add PutBlobResponseView with optional blake3Hash"`

---

## Task 3: `peer_blob_inventory` self-stamp helper for dual-write

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/db/peer_blob_inventory.rs` — add `record_self_custody(peer_id, blob_hash, blake3_hash, observed_at)`

**Rationale:** Existing fns (`apply_snapshot`, `apply_delta`, `record_fetch_success`) are designed for evidence arriving FROM other peers. A new fn `record_self_custody` is needed for "we just stored these bytes locally"; it stamps a row with both hash columns. The PK is `(peer_id, blob_hash)` (sha256), so `replace_into` gives idempotency on re-seed.

- [ ] **Step 3.1: Failing test** — Add to the existing `#[cfg(test)] mod tests` block at the bottom of `peer_blob_inventory.rs`:

```rust
#[test]
fn record_self_custody_writes_both_hashes() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    record_self_custody(
        &mut conn,
        "peer_self",
        "sha256-aa00",
        Some("blake3-bb00"),
        "2026-05-10T00:00:00Z",
    )
    .unwrap();

    let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].peer_id, "peer_self");
    assert_eq!(rows[0].source, "self-custody");
    assert_eq!(rows[0].blake3_hash.as_deref(), Some("blake3-bb00"));
}

#[test]
fn record_self_custody_is_idempotent() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    for _ in 0..3 {
        record_self_custody(
            &mut conn,
            "peer_self",
            "sha256-aa00",
            Some("blake3-bb00"),
            "2026-05-10T00:00:00Z",
        )
        .unwrap();
    }
    let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
    assert_eq!(rows.len(), 1, "re-seed must not duplicate inventory rows");
}

#[test]
fn record_self_custody_accepts_null_blake3() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // libp2p-only deployment: iroh side unavailable, blake3 is None.
    record_self_custody(&mut conn, "peer_self", "sha256-aa00", None, "2026-05-10T00:00:00Z").unwrap();

    let rows = lookup_hosts(&mut conn, "sha256-aa00", "2026-05-09T00:00:00Z").unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].blake3_hash.is_none());
}
```

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib peer_blob_inventory::tests::record_self_custody`. Expected: compile error — `record_self_custody` does not exist.

- [ ] **Step 3.2: Implement** — Add to `peer_blob_inventory.rs`, after `record_fetch_success` (around line 156):

```rust
/// Stamp a row recording that *this* peer holds these bytes locally, with
/// both the legacy SHA256 address and (optionally) the BLAKE3 address. Used
/// by the seeder dual-write path. Idempotent via `replace_into` on the
/// existing `(peer_id, blob_hash)` PK. Does NOT touch the cursor —
/// self-custody is local evidence, not a gossip arrival.
pub fn record_self_custody(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_hash: &str,
    blake3_hash: Option<&str>,
    observed_at: &str,
) -> Result<(), StorageError> {
    let row = NewPeerBlobInventoryRow {
        peer_id: peer_id.to_string(),
        blob_hash: blob_hash.to_string(),
        last_seen_at: observed_at.to_string(),
        source: "self-custody".to_string(),
        sequence: 0,
        blake3_hash: blake3_hash.map(|s| s.to_string()),
    };
    diesel::replace_into(peer_blob_inventory::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("record_self_custody: {e}")))
}
```

- [ ] **Step 3.3: Re-run** — same `cargo test` invocation as 3.1. Expected: 3 tests PASS.

- [ ] **Step 3.4: Commit** — `git add elohim/elohim-storage/src/db/peer_blob_inventory.rs && git commit -m "peer_blob_inventory: add record_self_custody for seeder dual-write"`

---

## Task 4: `handle_put_blob` dual-write

**Files:**
- Modify `/projects/elohim/elohim/elohim-storage/src/http.rs` — `handle_put_blob` at lines 1455-1590

**Rationale:** This is the load-bearing change. The handler reads body once, validates SHA256 hash (existing), executes the existing legacy shard write (existing), then if `self.iroh_blob_store.is_some()` ALSO writes to iroh and computes the BLAKE3 hash. The response is wrapped via `PutBlobResponseView::from_manifest(manifest, blake3_hash_string)`. Self peer_id for inventory stamp comes from `self.self_peer_id` which the http-blob-graduation plan added (verify with grep before this task).

The dual-write is best-effort on the iroh side: if `IrohBlobStore::add_bytes` errors, the handler logs a warning but still returns 201 with the SHA256 manifest and `blake3Hash: null`. Per memory `project_seed_whoever_is_ready`, partial readiness is OK.

- [ ] **Step 4.1: Verify the plumbing exists** — Run:

```bash
grep -n "iroh_blob_store\|self_peer_id\b" /projects/elohim/elohim/elohim-storage/src/http.rs | head -20
```

Expected: `iroh_blob_store: Option<Arc<crate::p2p_iroh::IrohBlobStore>>` field on `HttpServer` plus `with_iroh_blob_store` builder, and a `self_peer_id` field. If neither exists, see BLOCKED §1 and stop.

- [ ] **Step 4.2: Failing integration test** — Create `/projects/elohim/elohim/elohim-storage/tests/put_blob_dual_write.rs`:

```rust
//! Cutover gate #3: PUT /blob/{hash} writes to BOTH BlobStore (sha256) AND
//! IrohBlobStore (blake3) when iroh side is wired; stamps peer_blob_inventory
//! with both hashes; idempotent on re-seed.

use elohim_storage::{
    blob_store::BlobStore,
    p2p_iroh::IrohBlobStore,
    http::HttpServer,
};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn put_blob_dual_writes_when_iroh_configured() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());
    let iroh_store = Arc::new(IrohBlobStore::load(&tmp.path().join("iroh")).await.unwrap());

    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_iroh_blob_store(iroh_store.clone())
        .with_self_peer_id("peer_self_test".into()); // verify exact builder name

    let payload = b"dual-write test bytes".to_vec();
    let sha = BlobStore::compute_hash(&payload);

    // Drive handle_put_blob through the in-process HTTP layer (test helper).
    let resp = server.test_put_blob(&sha, &payload, "application/octet-stream").await;
    assert_eq!(resp.status, 201);

    let view: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert_eq!(view["blobHash"].as_str().unwrap(), sha);
    assert!(view["blake3Hash"].as_str().is_some(), "blake3Hash must be populated");

    // Both stores hold the bytes.
    assert!(blob_store.exists(&sha).await.unwrap());
    let blake3_str = view["blake3Hash"].as_str().unwrap();
    let blake3_hash: iroh_blobs::Hash = blake3_str.parse().unwrap();
    assert!(iroh_store.has(blake3_hash).await.unwrap());

    // Inventory row stamped with both.
    let pool = server.db_pool().expect("test db pool");
    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::peer_blob_inventory::lookup_hosts(
        &mut conn, &sha, "2026-01-01T00:00:00Z").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].peer_id, "peer_self_test");
    assert_eq!(rows[0].blake3_hash.as_deref(), Some(blake3_str));
}

#[tokio::test]
async fn put_blob_re_seed_is_idempotent() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());
    let iroh_store = Arc::new(IrohBlobStore::load(&tmp.path().join("iroh")).await.unwrap());

    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_iroh_blob_store(iroh_store.clone())
        .with_self_peer_id("peer_self_test".into());

    let payload = b"idempotent test bytes".to_vec();
    let sha = BlobStore::compute_hash(&payload);

    for _ in 0..3 {
        let resp = server.test_put_blob(&sha, &payload, "application/octet-stream").await;
        assert_eq!(resp.status, 201);
    }

    let pool = server.db_pool().expect("test db pool");
    let mut conn = pool.get().unwrap();
    let rows = elohim_storage::db::peer_blob_inventory::lookup_hosts(
        &mut conn, &sha, "2026-01-01T00:00:00Z").unwrap();
    assert_eq!(rows.len(), 1, "re-seed must not duplicate inventory rows");
}

#[tokio::test]
async fn put_blob_libp2p_only_deployment_succeeds_with_null_blake3() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());

    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    // No .with_iroh_blob_store — libp2p-only mode.
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_self_peer_id("peer_self_test".into());

    let payload = b"sha-only test bytes".to_vec();
    let sha = BlobStore::compute_hash(&payload);

    let resp = server.test_put_blob(&sha, &payload, "application/octet-stream").await;
    assert_eq!(resp.status, 201);

    let view: serde_json::Value = serde_json::from_slice(&resp.body).unwrap();
    assert!(view["blake3Hash"].is_null(), "libp2p-only mode must return null blake3");
    assert!(blob_store.exists(&sha).await.unwrap());
}
```

The test references `server.test_put_blob(...)` and `server.db_pool()`. If those test helpers don't exist on `HttpServer`, add them as `#[cfg(test)]` methods on `HttpServer` in `http.rs` that thinly wrap the existing routing. Drive them through `handle_put_blob` directly with a constructed `Request<Incoming>` if simpler.

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test put_blob_dual_write`. Expected: compile errors / test failures because dual-write logic isn't implemented yet.

- [ ] **Step 4.3: Implement dual-write** — In `handle_put_blob` (around `http.rs:1568` after `self.manifests.write().await.insert(...)`):

```rust
// --- Cutover gate #3: dual-write to IrohBlobStore when wired ---
let blake3_hash_str: Option<String> = match self.iroh_blob_store.as_ref() {
    Some(iroh) => match iroh.add_bytes(data.clone()).await {
        Ok(hash) => {
            tracing::debug!(
                sha256 = %manifest.blob_hash,
                blake3 = %hash,
                "Dual-wrote blob bytes to IrohBlobStore"
            );
            Some(hash.to_string())
        }
        Err(e) => {
            tracing::warn!(
                sha256 = %manifest.blob_hash,
                error = %e,
                "IrohBlobStore dual-write failed; SHA256 store still succeeded"
            );
            None
        }
    },
    None => None,
};

// Stamp self-custody inventory row (best-effort; log on failure).
if let Some(pool) = self.db_pool.as_ref() {
    let now = chrono::Utc::now().to_rfc3339();
    let sha = manifest.blob_hash.clone();
    let blake3_for_row = blake3_hash_str.clone();
    let peer_id = self.self_peer_id.clone();
    let pool_clone = pool.clone();
    tokio::task::spawn_blocking(move || {
        let mut conn = match pool_clone.get() {
            Ok(c) => c,
            Err(e) => { tracing::warn!(error = %e, "self-custody: db pool checkout failed"); return; }
        };
        if let Err(e) = crate::db::peer_blob_inventory::record_self_custody(
            &mut conn, &peer_id, &sha, blake3_for_row.as_deref(), &now,
        ) {
            tracing::warn!(error = %e, sha256 = %sha, "self-custody: inventory stamp failed");
        }
    });
}

let view = crate::views::PutBlobResponseView::from_manifest(manifest, blake3_hash_str);
let body = serde_json::to_string(&view)
    .map_err(|e| StorageError::Internal(e.to_string()))?;
```

Then replace the existing `let body = serde_json::to_string(&manifest)...` line and the response with the `view`-derived body. Keep status code 201 and `Content-Type: application/json`.

- [ ] **Step 4.4: Re-run integration tests** — same `cargo test` invocation as 4.2. Expected: all three tests PASS.

- [ ] **Step 4.5: Re-run unit + clippy** — `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy --features p2p-iroh -- -D warnings && cargo test --lib --bins`. Expected: no warnings, no regressions.

- [ ] **Step 4.6: Commit** — `git add elohim/elohim-storage/src/http.rs elohim/elohim-storage/tests/put_blob_dual_write.rs && git commit -m "http: dual-write blob bytes to IrohBlobStore on PUT /blob/{hash}"`

---

## Task 5: Seeder TS — consume `blake3Hash` and stamp metadata

**Files:**
- Modify `/projects/elohim/genesis/seeder/src/doorway-client.ts` — `pushBlob` reads new `blake3Hash` from response, returns it on `PushResult`
- Modify `/projects/elohim/genesis/seeder/src/storage-client.ts` — `pushBlob` does the same for direct-to-storage path
- Modify `/projects/elohim/genesis/seeder/src/blob-manager.ts` — `BlobMetadata` gains optional `blake3Hash` field
- Modify `/projects/elohim/genesis/seeder/src/seed.ts` — log the BLAKE3 hash in upload result lines (lines 1167, 1179, 1473)
- Modify `/projects/elohim/genesis/seeder/src/seed-production.ts` — same logging for the legacy production seeder paths (lines 298-340)

**Rationale:** TS clients consume the new optional response field. The seeder does NOT compute BLAKE3 client-side — the server does. This keeps the wire protocol "client supplies sha256, server derives blake3" simple and ensures both hashes reflect the SAME bytes the server validated. The `BlobManager`'s SHA256 computation (`blob-manager.ts:326`) stays unchanged.

- [ ] **Step 5.1: Failing test** — Add to `genesis/seeder/src/storage-client.spec.ts` (mock fetch returns response with `blake3Hash`):

```typescript
it('returns blake3Hash from server response when present', async () => {
  const mockManifest = {
    blobHash: 'sha256-aa',
    totalSize: 4,
    mimeType: 'text/plain',
    encoding: 'none',
    dataShards: 1,
    totalShards: 1,
    shardSize: 4,
    shardHashes: ['sha256-aa'],
    reach: 'commons',
    createdAt: new Date().toISOString(),
    blake3Hash: 'b'.repeat(64),
  };
  (global.fetch as any).mockResolvedValueOnce({
    ok: true,
    status: 201,
    json: async () => mockManifest,
  });

  const result = await client.pushBlob(Buffer.from('test'), 'text/plain');
  expect(result.success).toBe(true);
  expect(result.blake3Hash).toBe('b'.repeat(64));
});

it('handles libp2p-only response with null blake3Hash', async () => {
  const mockManifest = {
    blobHash: 'sha256-aa',
    totalSize: 4,
    mimeType: 'text/plain',
    encoding: 'none',
    dataShards: 1,
    totalShards: 1,
    shardSize: 4,
    shardHashes: ['sha256-aa'],
    reach: 'commons',
    createdAt: new Date().toISOString(),
    blake3Hash: null,
  };
  (global.fetch as any).mockResolvedValueOnce({
    ok: true,
    status: 201,
    json: async () => mockManifest,
  });

  const result = await client.pushBlob(Buffer.from('test'), 'text/plain');
  expect(result.success).toBe(true);
  expect(result.blake3Hash).toBeUndefined();
});
```

Run: `cd /projects/elohim/genesis/seeder && pnpm exec vitest run storage-client`. Expected: failures (PushResult lacks `blake3Hash`).

- [ ] **Step 5.2: Add `blake3Hash` to `PushResult`** — In `storage-client.ts`:

```typescript
export interface PushBlobResult {
  success: boolean;
  manifest?: ShardManifest;
  blake3Hash?: string;   // NEW: BLAKE3 hash from server-side dual-write (undefined if libp2p-only)
  error?: string;
}
```

In `pushBlob` (line 198), after `const manifest = await response.json();`:

```typescript
const blake3Hash = (manifest as any).blake3Hash || undefined;
return {
  success: true,
  manifest: { ...manifest, reach },
  blake3Hash,
};
```

Also extend `ShardManifest` interface (line 37-50) with `blake3Hash?: string | null;`.

- [ ] **Step 5.3: Same for `doorway-client.ts`** — `PushResult` interface (find it near line 441) and the response parsing in `pushBlob` (line 478-490). The doorway response is the storage `PutBlobResponseView` JSON forwarded through, so the field is already present.

- [ ] **Step 5.4: BlobManager metadata** — In `blob-manager.ts`, add to `BlobMetadata` (line 31):

```typescript
export interface BlobMetadata {
  hash: string;          // SHA256 (existing)
  blake3Hash?: string;   // NEW: BLAKE3 (populated post-upload by seeder, not by manager)
  sizeBytes: number;
  mimeType: string;
  entryPoint?: string;
  fallbackUrl?: string;
}
```

- [ ] **Step 5.5: Seed.ts logging + capture** — In `seed.ts:1167-1182`, after `if (uploadResult.success) {`, capture the BLAKE3:

```typescript
const blake3Note = uploadResult.blake3Hash ? ` blake3=${uploadResult.blake3Hash.slice(0, 12)}…` : ' (sha256-only)';
console.log(`   ✅ ${concept.id}: ${uploadResult.cached ? 'already cached' : 'uploaded'} (slug: ${slug})${blake3Note}`);
```

Apply same pattern to the path thumbnail upload at `seed.ts:1473`. For `seed-production.ts:298-340`, log BLAKE3 alongside SHA256 in the success line.

- [ ] **Step 5.6: Re-run TS tests** — `pnpm exec vitest run`. Expected: PASS.

- [ ] **Step 5.7: Commit** — `git add genesis/seeder/src/storage-client.ts genesis/seeder/src/doorway-client.ts genesis/seeder/src/blob-manager.ts genesis/seeder/src/seed.ts genesis/seeder/src/seed-production.ts genesis/seeder/src/storage-client.spec.ts && git commit -m "seeder: capture blake3Hash from PUT /blob/{hash} dual-write response"`

---

## Task 6: End-to-end seed -> verify both addresses serve

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/tests/seed_e2e_dual_address.rs`

**Rationale:** Prove the cutover-gate-#3 invariant end-to-end: after a seed, GET on the SHA256 address returns the same bytes as GET on the BLAKE3 address (both via `/blob/{hash}`, exercising the BLAKE3 acceptance added by the http-blob-graduation plan). Per memory anchor `feedback_head_vs_get_blob_asymmetry`, use GET not HEAD.

- [ ] **Step 6.1: Write the test** —

```rust
//! Cutover gate #3 e2e: a seeded blob is fetchable by BOTH its SHA256 and
//! BLAKE3 addresses, returning identical bytes.
//!
//! Memory anchor: feedback_head_vs_get_blob_asymmetry — this test uses GET,
//! never HEAD, on /blob/{hash}.

use elohim_storage::{blob_store::BlobStore, p2p_iroh::IrohBlobStore, http::HttpServer};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn seeded_blob_is_fetchable_by_both_addresses() {
    let tmp = tempdir().unwrap();
    let blob_store = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());
    let iroh_store = Arc::new(IrohBlobStore::load(&tmp.path().join("iroh")).await.unwrap());

    let bind: std::net::SocketAddr = "127.0.0.1:0".parse().unwrap();
    let server = HttpServer::new(blob_store.clone(), bind)
        .with_iroh_blob_store(iroh_store.clone())
        .with_self_peer_id("peer_self_e2e".into());

    let payload: Vec<u8> = (0..1024).map(|i| (i % 251) as u8).collect();
    let sha = BlobStore::compute_hash(&payload);

    // 1. PUT bytes once.
    let put_resp = server.test_put_blob(&sha, &payload, "application/octet-stream").await;
    assert_eq!(put_resp.status, 201);
    let view: serde_json::Value = serde_json::from_slice(&put_resp.body).unwrap();
    let blake3_addr = view["blake3Hash"].as_str().expect("blake3Hash present").to_string();

    // 2. GET by SHA256 returns the bytes.
    let sha_resp = server.test_get_blob(&sha).await;
    assert_eq!(sha_resp.status, 200, "SHA256 GET must return 200");
    assert_eq!(sha_resp.body.as_ref(), payload.as_slice());

    // 3. GET by BLAKE3 returns the same bytes (graduated dispatch from
    //    2026-05-10-iroh-http-blob-graduation.md must accept blake3-prefixed
    //    addresses on the same /blob/{hash} route).
    let blake3_resp = server.test_get_blob(&format!("blake3-{}", blake3_addr)).await;
    assert_eq!(blake3_resp.status, 200, "BLAKE3 GET must return 200");
    assert_eq!(blake3_resp.body.as_ref(), payload.as_slice());

    // 4. Bytes are byte-identical across both addresses.
    assert_eq!(sha_resp.body, blake3_resp.body);
}
```

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test seed_e2e_dual_address`. Expected: PASS (Tasks 4 + http-blob-graduation already landed).

If `test_get_blob` doesn't exist on `HttpServer`, add it as a `#[cfg(test)]` helper alongside `test_put_blob` from Task 4.

- [ ] **Step 6.2: Commit** — `git add elohim/elohim-storage/tests/seed_e2e_dual_address.rs && git commit -m "test(e2e): seeded blob fetchable by both sha256 and blake3 addresses"`

---

## Task 7: One-shot `backfill-blake3` binary

**Files:**
- Create `/projects/elohim/elohim/elohim-storage/src/bin/backfill_blake3.rs`
- Modify `/projects/elohim/elohim/elohim-storage/Cargo.toml` — add `[[bin]] name = "backfill-blake3"`

**Rationale:** Pre-cutover content was uploaded SHA256-only. The backfill binary scans the legacy `BlobStore`, re-streams every blob through `IrohBlobStore::add_bytes`, and updates the `peer_blob_inventory` row's `blake3_hash` column. Idempotent: blobs already present in `IrohBlobStore` are skipped (deduped by `iroh-blobs::Hash`).

- [ ] **Step 7.1: Failing test** — Create `tests/backfill_blake3.rs`:

```rust
//! Cutover gate #3: backfill BLAKE3 for blobs seeded before dual-write
//! shipped. Idempotent — running twice over the same legacy store must not
//! duplicate inventory rows or re-stream bytes already present in
//! IrohBlobStore.

use elohim_storage::{blob_store::BlobStore, p2p_iroh::IrohBlobStore};
use std::sync::Arc;
use tempfile::tempdir;

#[tokio::test]
async fn backfill_populates_blake3_for_legacy_blobs() {
    let tmp = tempdir().unwrap();
    let legacy = Arc::new(BlobStore::new(tmp.path().join("legacy")).await.unwrap());
    let iroh = Arc::new(IrohBlobStore::load(&tmp.path().join("iroh")).await.unwrap());

    // Pre-populate legacy store, NOT iroh.
    let payload_a = b"backfill A".to_vec();
    let payload_b = b"backfill B".to_vec();
    let sha_a = legacy.store(&payload_a).await.unwrap().hash;
    let sha_b = legacy.store(&payload_b).await.unwrap().hash;

    let pool = elohim_storage::test_helpers::in_memory_db_pool();
    // Seed inventory rows lacking blake3_hash (simulates pre-cutover state).
    {
        let mut conn = pool.get().unwrap();
        for sha in [&sha_a, &sha_b] {
            elohim_storage::db::peer_blob_inventory::record_self_custody(
                &mut conn, "peer_self", sha, None, "2026-04-01T00:00:00Z").unwrap();
        }
    }

    let report = elohim_storage::backfill::backfill_blake3(
        legacy.clone(), iroh.clone(), pool.clone(), "peer_self").await.unwrap();
    assert_eq!(report.scanned, 2);
    assert_eq!(report.backfilled, 2);
    assert_eq!(report.skipped_already_present, 0);
    assert_eq!(report.errors, 0);

    // Re-run: idempotent.
    let report2 = elohim_storage::backfill::backfill_blake3(
        legacy.clone(), iroh.clone(), pool.clone(), "peer_self").await.unwrap();
    assert_eq!(report2.scanned, 2);
    assert_eq!(report2.backfilled, 0);
    assert_eq!(report2.skipped_already_present, 2);

    // Inventory rows now carry blake3.
    let mut conn = pool.get().unwrap();
    for sha in [&sha_a, &sha_b] {
        let rows = elohim_storage::db::peer_blob_inventory::lookup_hosts(
            &mut conn, sha, "2026-01-01T00:00:00Z").unwrap();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].blake3_hash.is_some());
    }
}
```

If `test_helpers::in_memory_db_pool` doesn't exist, expose the existing `peer_blob_inventory.rs` test pool helper publicly under `#[cfg(any(test, feature = "test-helpers"))]`.

Run: `cd /projects/elohim/elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --features p2p-iroh --test backfill_blake3`. Expected: compile error — `backfill::backfill_blake3` does not exist.

- [ ] **Step 7.2: Implement the library function** — Create `/projects/elohim/elohim/elohim-storage/src/backfill.rs`:

```rust
//! Cutover gate #3: BLAKE3 backfill for legacy SHA256-keyed content.
//!
//! Streams every blob in the legacy `BlobStore` through `IrohBlobStore`, then
//! updates the corresponding `peer_blob_inventory` row's `blake3_hash`
//! column. Idempotent — already-present blobs in iroh and already-stamped
//! inventory rows are skipped. Safe to re-run.

use std::sync::Arc;
use crate::{blob_store::BlobStore, p2p_iroh::IrohBlobStore, db::DbPool};
use anyhow::Result;

#[derive(Debug, Default)]
pub struct BackfillReport {
    pub scanned: usize,
    pub backfilled: usize,
    pub skipped_already_present: usize,
    pub errors: usize,
}

pub async fn backfill_blake3(
    legacy: Arc<BlobStore>,
    iroh: Arc<IrohBlobStore>,
    pool: DbPool,
    self_peer_id: &str,
) -> Result<BackfillReport> {
    let mut report = BackfillReport::default();
    let hashes = legacy.list_hashes()?;
    let now = chrono::Utc::now().to_rfc3339();

    for sha in hashes {
        report.scanned += 1;
        let bytes = match legacy.get(&sha).await {
            Ok(Some(b)) => b,
            Ok(None) => { report.errors += 1; continue; }
            Err(e) => { tracing::warn!(sha = %sha, error = %e, "backfill: legacy read failed"); report.errors += 1; continue; }
        };

        let blake3_hash = match iroh.add_bytes(bytes).await {
            Ok(h) => h,
            Err(e) => { tracing::warn!(sha = %sha, error = %e, "backfill: iroh add failed"); report.errors += 1; continue; }
        };

        // Idempotency check: was the existing inventory row already stamped?
        let mut conn = pool.get()?;
        let existing = crate::db::peer_blob_inventory::lookup_hosts(&mut conn, &sha, "1970-01-01T00:00:00Z")?;
        let already_stamped = existing.iter().any(|r| r.peer_id == self_peer_id && r.blake3_hash.is_some());

        if already_stamped {
            report.skipped_already_present += 1;
        } else {
            crate::db::peer_blob_inventory::record_self_custody(
                &mut conn, self_peer_id, &sha, Some(&blake3_hash.to_string()), &now,
            )?;
            report.backfilled += 1;
        }
    }
    Ok(report)
}
```

Add `pub mod backfill;` to `lib.rs`.

- [ ] **Step 7.3: Re-run tests** — same `cargo test` invocation as 7.1. Expected: PASS.

- [ ] **Step 7.4: Implement the binary** — Create `/projects/elohim/elohim/elohim-storage/src/bin/backfill_blake3.rs`:

```rust
//! Backfill BLAKE3 hashes for legacy SHA256-only blobs.
//!
//! Invocation:
//!   RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release \
//!     --features p2p-iroh --bin backfill-blake3 -- \
//!     --storage-dir /var/elohim/storage --self-peer-id 12D3KooW...
//!
//! Idempotent — re-runs safely. Stops on first DB error; partial completion
//! is OK (re-run picks up where it left off).

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Parser)]
struct Args {
    /// Storage directory containing both `blobs/` (legacy) and `blobs_iroh/`.
    #[arg(long)]
    storage_dir: PathBuf,
    /// Self peer ID for inventory stamping (libp2p multihash form).
    #[arg(long)]
    self_peer_id: String,
    /// SQLite DB URL (defaults to `{storage_dir}/storage.db`).
    #[arg(long)]
    db_url: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let legacy_dir = args.storage_dir.join("blobs");
    let iroh_dir = args.storage_dir.join("blobs_iroh");
    let db_url = args.db_url.unwrap_or_else(||
        format!("sqlite://{}/storage.db", args.storage_dir.display()));

    let legacy = Arc::new(elohim_storage::blob_store::BlobStore::new(&legacy_dir).await?);
    let iroh = Arc::new(elohim_storage::p2p_iroh::IrohBlobStore::load(&iroh_dir).await?);
    let pool = elohim_storage::db::open_pool(&db_url)?;

    let report = elohim_storage::backfill::backfill_blake3(
        legacy, iroh, pool, &args.self_peer_id).await?;

    println!("scanned={} backfilled={} skipped={} errors={}",
        report.scanned, report.backfilled, report.skipped_already_present, report.errors);

    if report.errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}
```

If `elohim_storage::db::open_pool` doesn't exist with that signature, find the actual constructor (grep for `fn .*-> DbPool` in `db/mod.rs`) and use it.

- [ ] **Step 7.5: Add bin to Cargo.toml** — In `elohim/elohim-storage/Cargo.toml`:

```toml
[[bin]]
name = "backfill-blake3"
path = "src/bin/backfill_blake3.rs"
required-features = ["p2p-iroh"]
```

- [ ] **Step 7.6: Verify** — `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release --features p2p-iroh --bin backfill-blake3`. Expected: builds.

- [ ] **Step 7.7: Commit** — `git add elohim/elohim-storage/src/backfill.rs elohim/elohim-storage/src/bin/backfill_blake3.rs elohim/elohim-storage/src/lib.rs elohim/elohim-storage/Cargo.toml elohim/elohim-storage/tests/backfill_blake3.rs && git commit -m "backfill: one-shot binary populates BLAKE3 for legacy SHA256 blobs"`

---

## Task 8: A2O scenario — seeded blob is dual-addressable

**Files:**
- Create `/projects/elohim/genesis/a2o/features/deployment/seeder-dual-write.feature`
- Create `/projects/elohim/genesis/a2o/steps/seeder-dual-write.steps.ts`

**Rationale:** Lock the cutover invariant into the executable spec. Per `genesis/a2o/CLAUDE.md`, this is a `@deployment` domain test. Per `feedback_a2o_is_human_experience_not_dev_bugs`, the scenario expresses an operator-observable property ("after seed completes, the same bytes serve from both addresses through doorway"), not a unit-test assertion.

- [ ] **Step 8.1: Write the feature** —

```gherkin
@e2e @deployment @requires:doorway @requires:seeded-content
Feature: Seeder writes blobs under both SHA256 and BLAKE3 addresses
  As a hub operator transitioning from libp2p to iroh
  I want every blob the seeder uploads to be fetchable by both addresses
  So that consumer-grade peers (libp2p-fallback) and iroh-canonical peers
  both reach the same content during and after cutover.

  Background:
    Given doorway "alpha" at "E2E_DOORWAY_ALPHA"
    And the doorway is connected to elohim-storage
    And elohim-storage runs with TransportBackend=Iroh

  Scenario: A freshly seeded HTML5 app serves under both addresses
    Given the seeder has uploaded an HTML5 app named "evolution-of-trust"
    And the upload response reported both blobHash and blake3Hash
    When I GET "/blob/{blobHash}" through the doorway
    Then the response status is 200
    And I record the response body as "sha256_bytes"
    When I GET "/blob/blake3-{blake3Hash}" through the doorway
    Then the response status is 200
    And the response body equals "sha256_bytes"

  Scenario: Re-seeding the same content does not duplicate inventory rows
    Given the seeder has uploaded an HTML5 app named "evolution-of-trust" once
    When the seeder uploads the same HTML5 app a second time
    Then peer_blob_inventory contains exactly one row for the SHA256 address with peer_id matching this peer
    And that row's blake3_hash column is populated

  Scenario: libp2p-only deployment still reports success with null blake3Hash
    Given elohim-storage is configured with TransportBackend=Libp2p
    When the seeder uploads an HTML5 app named "the-fertile-field"
    Then the upload response reports blobHash
    And the upload response reports blake3Hash as null
    And GET "/blob/{blobHash}" through the doorway returns 200
```

- [ ] **Step 8.2: Step skeleton** — Run `cd /projects/elohim/genesis/a2o && npx tsx scripts/generate-step-skeletons.ts` to scaffold missing step definitions, then implement them in `seeder-dual-write.steps.ts` using `E2EWorld`'s existing HTTP fetch helpers and DB query helpers (mirror patterns from `seeder.steps.ts`).

- [ ] **Step 8.3: Run the feature in WIP mode** — Tag scenarios `@wip` if backend gates aren't ready in CI yet. Per CLAUDE.md, "scenario + implementation commit together" — the WIP tag flips off only after Tasks 1-7 are merged AND the e2e harness has both transport backends available.

- [ ] **Step 8.4: Commit** — `git add genesis/a2o/features/deployment/seeder-dual-write.feature genesis/a2o/steps/seeder-dual-write.steps.ts && git commit -m "a2o: seeder dual-write scenarios for cutover gate #3"`

---

## Task 9: Self-review — verify cutover gate #3 closure

**Files:**
- No file changes; this is a verification task

- [ ] **Step 9.1: Schema-coverage** — Confirm `pnpm run schema:codegen:ts && pnpm run schema:validate` passes with no drift on `put-blob-response.schema.json`.

- [ ] **Step 9.2: Idempotency proof** — Run the integration test from Task 4: `cargo test --features p2p-iroh --test put_blob_dual_write put_blob_re_seed_is_idempotent`. PASS = idempotency closed.

- [ ] **Step 9.3: Backfill exit-zero** — Against a temp dir with 100 pre-existing legacy blobs and an empty iroh dir, run `backfill-blake3` once → expect `backfilled=100 errors=0`. Re-run → expect `backfilled=0 skipped=100 errors=0`.

- [ ] **Step 9.4: Partial-readiness preserved** — Confirm the test `put_blob_libp2p_only_deployment_succeeds_with_null_blake3` from Task 4 PASSES. This is the load-bearing assertion that memory anchor `project_seed_whoever_is_ready` is honored.

- [ ] **Step 9.5: Spec line 521 invariant** — Run `grep -rn 'iroh.*remove\|drop.*BlobStore\|remove.*sha256' elohim/elohim-storage/src/` and confirm zero matches that would delete the SHA256 path. Spec mandates SHA256 stays.

- [ ] **Step 9.6: Commit nothing if all green** — Mark this task done. The cutover gate #3 deliverable is complete.

---

## BLOCKED items (escalate to user before continuing)

### §1 — Pre-requisite plan landing

Task 4 assumes `2026-05-10-iroh-http-blob-graduation.md`'s Task 5 has landed, adding to `HttpServer`:
- `iroh_blob_store: Option<Arc<crate::p2p_iroh::IrohBlobStore>>`
- `with_iroh_blob_store(store: Arc<IrohBlobStore>) -> Self`
- `self_transport_manifest: Option<Arc<PeerTransportManifest>>`

If `grep -n "iroh_blob_store\|with_iroh_blob_store" /projects/elohim/elohim/elohim-storage/src/http.rs` returns zero hits, this plan is BLOCKED. Land cutover gate #2 first, then re-baseline this plan.

### §2 — `self_peer_id` source

Task 4 stamps `peer_blob_inventory` with `self.self_peer_id`. The exact source of this field on `HttpServer` is unverified — it may be on a different layer (e.g., the libp2p swarm initialization in `p2p/mod.rs:422`). If `HttpServer` does not carry self peer id, two options:
  - (a) Plumb it through as `with_self_peer_id(String)` builder (preferred, consistent with `with_iroh_blob_store`)
  - (b) Read from `self.self_transport_manifest.as_ref().map(|m| &m.libp2p_peer_id)` if Plan 1's manifest is wired

Confirm with user which option to take before implementing Task 4 Step 4.3. Option (a) is the implicit assumption in this plan's code blocks.

### §3 — Test helper visibility

Tasks 4, 6, 7 reference `HttpServer::test_put_blob`, `HttpServer::test_get_blob`, `HttpServer::db_pool`, and `elohim_storage::test_helpers::in_memory_db_pool`. If these helpers do not exist:
  - Add them as `#[cfg(test)] impl` blocks on `HttpServer` in `http.rs`, OR
  - Use the existing `tower::ServiceExt` + raw `Request<Incoming>` pattern (less ergonomic but exists already in some integration tests)

This is a test-ergonomics question, not a design question — the implementer chooses.

---

## Self-review summary

- **Spec coverage**: cutover gate #3 (spec line 512) — dual-write to `IrohBlobStore` AND `BlobStore`, BLAKE3 canonical post-cutover, SHA256 retained per line 521. CLOSED.
- **Partial-readiness**: Task 4 explicit test `put_blob_libp2p_only_deployment_succeeds_with_null_blake3` proves the seeder still succeeds when iroh is unavailable. Memory anchor `project_seed_whoever_is_ready` honored.
- **Idempotency**: Task 3 `record_self_custody` uses `replace_into` on `(peer_id, blob_hash)` PK; Task 4 explicit test `put_blob_re_seed_is_idempotent`; Task 7 backfill explicit `skipped_already_present` accounting.
- **Backfill scope**: Existing legacy `BlobStore` is the source set; backfill streams every entry through `IrohBlobStore::add_bytes` and updates the inventory row's `blake3_hash` column. One-shot binary `backfill-blake3` with idempotent re-run, gated on `p2p-iroh` feature.
- **No placeholder text**: every "TBD" / "appropriate" / "similar to" search yields zero hits.
- **No new crate deps**: blake3 (transitive via iroh-blobs), sha2 (existing in `blob_store.rs`), chrono (already used), clap (already used in `main.rs`), tracing (existing). The Decision-Required section is absent because no new deps are proposed.
