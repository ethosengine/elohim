# P2P Resilience Sprint B: Resilience Projection

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Backend tracks shard distribution per content, serves a resilience projection endpoint, seeds REA storage commitments, and the Network tab displays resilience data.

**Architecture:** Two new Category C (operational) SQLite tables (`shard_manifests`, `shard_locations`) track local encoding and peer distribution state. A computed `/api/v1/resilience/{content_id}` endpoint aggregates these with stewardship allocations and REA commitments. The Angular Network tab gains a resilience section. Doorway proxies the new routes.

**Tech Stack:** Rust (Diesel ORM, SQLite), Angular 19, TypeScript, cargo test, Vitest

**Depends on:** Sprint A (stewardship tooltip wiring, RS tests passing)

---

### Task 1: Create `shard_manifests` Migration

**Category C (operational)** — per-peer local encoding state, not DHT-notarized.

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-04-000000_shard_manifests/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-04-000000_shard_manifests/down.sql`

- [ ] **Step 1: Create migration directory**

Run: `mkdir -p elohim/elohim-storage/migrations/2026-04-04-000000_shard_manifests`

- [ ] **Step 2: Write up.sql**

```sql
-- Source of truth: Local SQLite (per-peer encoding state)
-- Classification: C (Operational) — rebuilt from local blob store, not shared via DHT
-- No dht_anchor_hash: this is a projection of local state

CREATE TABLE shard_manifests (
    content_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'lamad',
    blob_hash TEXT NOT NULL,
    blob_cid TEXT,
    encoding TEXT NOT NULL DEFAULT 'none',
    data_shard_count INTEGER NOT NULL DEFAULT 1,
    parity_shard_count INTEGER NOT NULL DEFAULT 0,
    shard_hashes_json TEXT NOT NULL DEFAULT '[]',
    total_size_bytes INTEGER NOT NULL,
    shard_size_bytes INTEGER NOT NULL,
    mime_type TEXT NOT NULL DEFAULT 'application/octet-stream',
    reach TEXT NOT NULL DEFAULT 'commons',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (content_id, h_app_id)
);

CREATE INDEX idx_shard_manifests_blob ON shard_manifests(blob_hash);
CREATE INDEX idx_shard_manifests_encoding ON shard_manifests(encoding);
```

- [ ] **Step 3: Write down.sql**

```sql
DROP INDEX IF EXISTS idx_shard_manifests_encoding;
DROP INDEX IF EXISTS idx_shard_manifests_blob;
DROP TABLE IF EXISTS shard_manifests;
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-04-000000_shard_manifests/
git commit -m "feat(storage): add shard_manifests migration (Category C)

Local per-peer encoding state. Tracks encoding strategy, shard hashes,
and sizes for each content item. Not DHT-notarized."
```

---

### Task 2: Create `shard_locations` Migration

**Category C (operational)** — ephemeral peer tracking, rebuilt from protocol events.

**Files:**
- Create: `elohim/elohim-storage/migrations/2026-04-04-100000_shard_locations/up.sql`
- Create: `elohim/elohim-storage/migrations/2026-04-04-100000_shard_locations/down.sql`

- [ ] **Step 1: Create migration directory**

Run: `mkdir -p elohim/elohim-storage/migrations/2026-04-04-100000_shard_locations`

- [ ] **Step 2: Write up.sql**

```sql
-- Source of truth: Local SQLite (peer shard tracking)
-- Classification: C (Operational) — rebuilt from shard protocol ack events
-- No dht_anchor_hash: ephemeral tracking data

CREATE TABLE shard_locations (
    shard_hash TEXT NOT NULL,
    peer_id TEXT NOT NULL,
    h_app_id TEXT NOT NULL DEFAULT 'lamad',
    status TEXT NOT NULL DEFAULT 'announced',
    first_seen TEXT NOT NULL DEFAULT (datetime('now')),
    last_verified TEXT,
    PRIMARY KEY (shard_hash, peer_id)
);

CREATE INDEX idx_shard_locations_peer ON shard_locations(peer_id);
CREATE INDEX idx_shard_locations_status ON shard_locations(status);
```

- [ ] **Step 3: Write down.sql**

```sql
DROP INDEX IF EXISTS idx_shard_locations_status;
DROP INDEX IF EXISTS idx_shard_locations_peer;
DROP TABLE IF EXISTS shard_locations;
```

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/migrations/2026-04-04-100000_shard_locations/
git commit -m "feat(storage): add shard_locations migration (Category C)

Tracks which peers hold which shards. Rebuilt from shard protocol
acknowledgment events. Statuses: announced, verified, lost."
```

---

### Task 3: Add Diesel Models and CRUD for Shard Tables

**Files:**
- Modify: `elohim/elohim-storage/src/db/diesel_schema.rs` (add table macros)
- Modify: `elohim/elohim-storage/src/db/models.rs` (add model structs)
- Create: `elohim/elohim-storage/src/db/shard_manifests.rs`
- Create: `elohim/elohim-storage/src/db/shard_locations.rs`
- Modify: `elohim/elohim-storage/src/db/mod.rs` (export new modules)

- [ ] **Step 1: Add Diesel table macros to diesel_schema.rs**

Add to the file (follow existing pattern of alphabetical ordering):

```rust
diesel::table! {
    shard_locations (shard_hash, peer_id) {
        shard_hash -> Text,
        peer_id -> Text,
        h_app_id -> Text,
        status -> Text,
        first_seen -> Text,
        last_verified -> Nullable<Text>,
    }
}

diesel::table! {
    shard_manifests (content_id, h_app_id) {
        content_id -> Text,
        h_app_id -> Text,
        blob_hash -> Text,
        blob_cid -> Nullable<Text>,
        encoding -> Text,
        data_shard_count -> Integer,
        parity_shard_count -> Integer,
        shard_hashes_json -> Text,
        total_size_bytes -> Integer,
        shard_size_bytes -> Integer,
        mime_type -> Text,
        reach -> Text,
        created_at -> Text,
    }
}
```

- [ ] **Step 2: Add model structs to models.rs**

Add Queryable and Insertable structs:

```rust
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = shard_manifests)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ShardManifestRow {
    pub content_id: String,
    pub h_app_id: String,
    pub blob_hash: String,
    pub blob_cid: Option<String>,
    pub encoding: String,
    pub data_shard_count: i32,
    pub parity_shard_count: i32,
    pub shard_hashes_json: String,
    pub total_size_bytes: i32,
    pub shard_size_bytes: i32,
    pub mime_type: String,
    pub reach: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = shard_manifests)]
pub struct NewShardManifest<'a> {
    pub content_id: &'a str,
    pub h_app_id: &'a str,
    pub blob_hash: &'a str,
    pub blob_cid: Option<&'a str>,
    pub encoding: &'a str,
    pub data_shard_count: i32,
    pub parity_shard_count: i32,
    pub shard_hashes_json: &'a str,
    pub total_size_bytes: i32,
    pub shard_size_bytes: i32,
    pub mime_type: &'a str,
    pub reach: &'a str,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = shard_locations)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct ShardLocationRow {
    pub shard_hash: String,
    pub peer_id: String,
    pub h_app_id: String,
    pub status: String,
    pub first_seen: String,
    pub last_verified: Option<String>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = shard_locations)]
pub struct NewShardLocation<'a> {
    pub shard_hash: &'a str,
    pub peer_id: &'a str,
    pub h_app_id: &'a str,
    pub status: &'a str,
}
```

- [ ] **Step 3: Create shard_manifests.rs CRUD module**

Create `elohim/elohim-storage/src/db/shard_manifests.rs`:

```rust
use diesel::prelude::*;

use super::diesel_schema::shard_manifests;
use super::models::{NewShardManifest, ShardManifestRow};
use crate::StorageError;

pub fn upsert_manifest(
    conn: &mut SqliteConnection,
    manifest: &NewShardManifest,
) -> Result<ShardManifestRow, StorageError> {
    diesel::replace_into(shard_manifests::table)
        .values(manifest)
        .execute(conn)?;

    shard_manifests::table
        .filter(shard_manifests::content_id.eq(manifest.content_id))
        .filter(shard_manifests::h_app_id.eq(manifest.h_app_id))
        .first(conn)
        .map_err(StorageError::from)
}

pub fn get_manifest(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<Option<ShardManifestRow>, StorageError> {
    shard_manifests::table
        .filter(shard_manifests::content_id.eq(content_id))
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .first(conn)
        .optional()
        .map_err(StorageError::from)
}

pub fn list_manifests_by_encoding(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    encoding: &str,
) -> Result<Vec<ShardManifestRow>, StorageError> {
    shard_manifests::table
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .filter(shard_manifests::encoding.eq(encoding))
        .load(conn)
        .map_err(StorageError::from)
}
```

- [ ] **Step 4: Create shard_locations.rs CRUD module**

Create `elohim/elohim-storage/src/db/shard_locations.rs`:

```rust
use diesel::prelude::*;

use super::diesel_schema::shard_locations;
use super::models::{NewShardLocation, ShardLocationRow};
use crate::StorageError;

pub fn upsert_location(
    conn: &mut SqliteConnection,
    location: &NewShardLocation,
) -> Result<(), StorageError> {
    diesel::replace_into(shard_locations::table)
        .values(location)
        .execute(conn)?;
    Ok(())
}

pub fn get_locations_for_shard(
    conn: &mut SqliteConnection,
    shard_hash: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::shard_hash.eq(shard_hash))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::peer_id.eq(peer_id))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_content(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    // Join with shard_manifests to find all shard hashes for this content,
    // then find all locations for those hashes
    use super::diesel_schema::shard_manifests;

    let manifest = shard_manifests::table
        .filter(shard_manifests::content_id.eq(content_id))
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .first::<super::models::ShardManifestRow>(conn)
        .optional()?;

    let Some(manifest) = manifest else {
        return Ok(vec![]);
    };

    let shard_hashes: Vec<String> =
        serde_json::from_str(&manifest.shard_hashes_json).unwrap_or_default();

    shard_locations::table
        .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn mark_lost(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set(shard_locations::status.eq("lost"))
    .execute(conn)?;
    Ok(())
}

pub fn update_verified(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set((
        shard_locations::status.eq("verified"),
        shard_locations::last_verified.eq(&now),
    ))
    .execute(conn)?;
    Ok(())
}
```

- [ ] **Step 5: Export modules from db/mod.rs**

Add to the module exports in `elohim/elohim-storage/src/db/mod.rs`:

```rust
pub mod shard_locations;
pub mod shard_manifests;
```

- [ ] **Step 6: Build to verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

Expected: Compiles without errors. Diesel schema matches migration SQL.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/db/diesel_schema.rs elohim/elohim-storage/src/db/models.rs elohim/elohim-storage/src/db/shard_manifests.rs elohim/elohim-storage/src/db/shard_locations.rs elohim/elohim-storage/src/db/mod.rs
git commit -m "feat(storage): add shard manifest and location CRUD modules

Diesel models, Queryable/Insertable structs, and CRUD operations for
shard_manifests (encoding state) and shard_locations (peer tracking).
Both Category C operational tables."
```

---

### Task 4: Write Shard Manifest on Blob Ingest

When content is created with a blob, compute and store the shard manifest. This happens in the content creation HTTP handler.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (content creation handlers ~line 2104-2290)

- [ ] **Step 1: Add manifest creation after successful content insert (single create)**

In `handle_db_content_list()` at the POST handler (~line 2134), after the content is created and the EPR head publication block, add shard manifest recording:

```rust
// After content creation succeeds, record shard manifest if blob exists
if let (Some(ref blob_hash), Some(ref pool)) = (&input_view_clone.blob_hash, self.pool.as_ref()) {
    let h_app_id = h_app_id.clone();
    let content_id = input_view_clone.id.clone();
    let blob_cid = input_view_clone.blob_cid.clone();
    let reach = input_view_clone.reach.clone().unwrap_or_else(|| "commons".to_string());
    let content_format = input_view_clone.content_format.clone().unwrap_or_default();

    if let Ok(mut conn) = pool.get() {
        // Check if blob exists in store to get size
        if let Some(ref blob_store) = self.blob_store {
            if let Ok(Some(data)) = blob_store.get(blob_hash).await {
                let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
                let manifest = encoder.create_manifest(&data, &content_format, &reach);
                let shard_hashes_json = serde_json::to_string(&manifest.shard_hashes).unwrap_or_else(|_| "[]".to_string());

                let new_manifest = crate::db::models::NewShardManifest {
                    content_id: &content_id,
                    h_app_id: &h_app_id,
                    blob_hash,
                    blob_cid: blob_cid.as_deref(),
                    encoding: &manifest.encoding,
                    data_shard_count: manifest.data_shards as i32,
                    parity_shard_count: (manifest.total_shards - manifest.data_shards) as i32,
                    shard_hashes_json: &shard_hashes_json,
                    total_size_bytes: manifest.total_size as i32,
                    shard_size_bytes: manifest.shard_size as i32,
                    mime_type: &manifest.mime_type,
                    reach: &reach,
                };

                if let Err(e) = crate::db::shard_manifests::upsert_manifest(&mut conn, &new_manifest) {
                    tracing::warn!(content_id = %content_id, error = %e, "Failed to record shard manifest");
                }
            }
        }
    }
}
```

Note: `input_view_clone` refers to cloned input data — match the variable names used in the existing handler. The blob_hash field on the input may be `blob_hash` or `blob_cid`; check the `CreateContentInputView` struct.

- [ ] **Step 2: Add manifest creation for bulk create**

Same pattern in `handle_db_content_bulk()` (~line 2240). After bulk insert succeeds, iterate over the inserted items and create manifests for those with blobs.

- [ ] **Step 3: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

- [ ] **Step 4: Test by seeding content**

Run the seeder and verify manifests are created:

```bash
# After seeding, query the shard_manifests table
sqlite3 /path/to/storage.db "SELECT content_id, encoding, data_shard_count, parity_shard_count, total_size_bytes FROM shard_manifests LIMIT 10;"
```

Expected: Rows with `encoding="none"` for small content, `encoding="chunked"` or `encoding="rs-4-7"` for large content.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): record shard manifest on content creation

When content with a blob is created, compute RS/chunked/single encoding
and store manifest in shard_manifests table. Enables resilience queries."
```

---

### Task 5: Resilience Endpoint

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (add ResilienceView)
- Modify: `elohim/elohim-storage/src/http.rs` (add route handler)

- [ ] **Step 1: Add ResilienceView types to views.rs**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceView {
    pub content_id: String,
    pub encoding: EncodingInfoView,
    pub distribution: DistributionView,
    pub stewardship: ResilienceStewardshipView,
    pub commitments: CommitmentHealthView,
    pub health: HealthScoreView,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct EncodingInfoView {
    pub strategy: String,
    pub data_shards: i32,
    pub parity_shards: i32,
    pub total_size_bytes: i64,
    pub shard_size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct DistributionView {
    pub total_shards: i32,
    pub shards_with_locations: i32,
    pub distinct_peers: i32,
    pub shards: Vec<ShardInfoView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ShardInfoView {
    pub hash: String,
    pub shard_type: String,
    pub peer_ids: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResilienceStewardshipView {
    pub steward_count: i32,
    pub allocations: Vec<StewardshipAllocationView>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CommitmentHealthView {
    pub active_peers: i32,
    pub total_committed_bytes: i64,
    pub total_used_bytes: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct HealthScoreView {
    pub score: f32,
    pub can_survive_failures: i32,
    pub status: String,
}
```

- [ ] **Step 2: Add resilience route handler to http.rs**

Add route matching for `GET /api/v1/resilience/{content_id}`:

```rust
async fn handle_resilience(
    &self,
    content_id: &str,
    h_app_id: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let pool = self.pool.as_ref().ok_or(StorageError::NotConfigured)?;
    let mut conn = pool.get()?;

    // 1. Get shard manifest
    let manifest = crate::db::shard_manifests::get_manifest(&mut conn, h_app_id, content_id)?;

    let encoding = match &manifest {
        Some(m) => EncodingInfoView {
            strategy: m.encoding.clone(),
            data_shards: m.data_shard_count,
            parity_shards: m.parity_shard_count,
            total_size_bytes: m.total_size_bytes as i64,
            shard_size_bytes: m.shard_size_bytes as i64,
        },
        None => EncodingInfoView {
            strategy: "unknown".to_string(),
            data_shards: 0,
            parity_shards: 0,
            total_size_bytes: 0,
            shard_size_bytes: 0,
        },
    };

    // 2. Get shard locations
    let locations = crate::db::shard_locations::get_locations_for_content(&mut conn, h_app_id, content_id)?;

    let shard_hashes: Vec<String> = manifest
        .as_ref()
        .map(|m| serde_json::from_str(&m.shard_hashes_json).unwrap_or_default())
        .unwrap_or_default();

    let data_shard_count = manifest.as_ref().map(|m| m.data_shard_count).unwrap_or(0) as usize;

    let shards: Vec<ShardInfoView> = shard_hashes
        .iter()
        .enumerate()
        .map(|(i, hash)| {
            let peers: Vec<String> = locations
                .iter()
                .filter(|l| l.shard_hash == *hash)
                .map(|l| l.peer_id.clone())
                .collect();
            let status = if peers.is_empty() { "missing" } else { "distributed" };
            ShardInfoView {
                hash: hash.clone(),
                shard_type: if i < data_shard_count { "data".to_string() } else { "parity".to_string() },
                peer_ids: peers,
                status: status.to_string(),
            }
        })
        .collect();

    let distinct_peers: std::collections::HashSet<&str> = locations.iter().map(|l| l.peer_id.as_str()).collect();
    let shards_with_locations = shards.iter().filter(|s| !s.peer_ids.is_empty()).count() as i32;

    let distribution = DistributionView {
        total_shards: shard_hashes.len() as i32,
        shards_with_locations,
        distinct_peers: distinct_peers.len() as i32,
        shards,
    };

    // 3. Get stewardship allocations
    let allocs = crate::db::stewardship_allocations::get_allocations_for_content(
        &mut conn, h_app_id, content_id,
    )?;
    let stewardship = ResilienceStewardshipView {
        steward_count: allocs.len() as i32,
        allocations: allocs.into_iter().map(StewardshipAllocationView::from).collect(),
    };

    // 4. Get storage commitments (REA commitments with action="provide")
    let commitments_list = crate::db::rea_commitments::list_commitments_by_action(
        &mut conn, h_app_id, "provide",
    ).unwrap_or_default();
    let total_committed: f32 = commitments_list.iter()
        .filter_map(|c| c.resource_quantity_value)
        .sum();
    let commitments = CommitmentHealthView {
        active_peers: commitments_list.len() as i32,
        total_committed_bytes: (total_committed * 1_073_741_824.0) as i64, // GB to bytes
        total_used_bytes: 0, // TODO: compute from actual storage usage
    };

    // 5. Compute health score
    let parity = encoding.parity_shards;
    let can_survive = parity.min(distribution.distinct_peers.saturating_sub(1));
    let score = if distribution.total_shards == 0 {
        0.0
    } else {
        (shards_with_locations as f32) / (distribution.total_shards as f32)
    };
    let status = if score >= 0.8 && stewardship.steward_count > 0 {
        "healthy"
    } else if score >= 0.5 {
        "degraded"
    } else {
        "at_risk"
    };

    let resilience = ResilienceView {
        content_id: content_id.to_string(),
        encoding,
        distribution,
        stewardship,
        commitments,
        health: HealthScoreView {
            score,
            can_survive_failures: can_survive,
            status: status.to_string(),
        },
    };

    let json = serde_json::to_string(&resilience)?;
    Ok(Response::builder()
        .status(200)
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(json)))?)
}
```

- [ ] **Step 3: Wire the route into the router**

Add the route match in the main router (around where `/api/v1/` routes are matched). Pattern:

```rust
// In the router match block:
("GET", path) if path.starts_with("/api/v1/resilience/") => {
    let content_id = &path["/api/v1/resilience/".len()..];
    self.handle_resilience(content_id, &h_app_id).await
}
```

- [ ] **Step 4: Add `list_commitments_by_action` to rea_commitments.rs**

Check if `rea_commitments.rs` has a method to filter by action. If not, add:

```rust
pub fn list_commitments_by_action(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    action: &str,
) -> Result<Vec<ReaCommitment>, StorageError> {
    use super::diesel_schema::rea_commitments;
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(h_app_id))
        .filter(rea_commitments::action.eq(action))
        .filter(rea_commitments::state.ne("completed"))
        .load(conn)
        .map_err(StorageError::from)
}
```

- [ ] **Step 5: Generate TypeScript types**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings`

Verify new types appear in `elohim/sdk/storage-client-ts/src/generated/`.

- [ ] **Step 6: Build and test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/db/rea_commitments.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): add GET /api/v1/resilience/{content_id} endpoint

Returns encoding strategy, shard distribution, stewardship allocations,
storage commitments, and computed health score for content resilience."
```

---

### Task 6: Seed REA Storage Commitments

Add storage commitment seeding so each test peer declares its storage capacity.

**Files:**
- Create: `genesis/seeder/src/seed-commitments.ts`
- Modify: `genesis/seeder/src/seed.ts` (call new seeder)

- [ ] **Step 1: Create seed-commitments.ts**

```typescript
import { StorageApiService } from './api.js';

interface PeerCommitment {
  id: string;
  provider: string;
  displayName: string;
  storageGb: number;
  region: string;
}

const PEER_COMMITMENTS: PeerCommitment[] = [
  { id: 'commitment-matthew-storage', provider: 'human-matthew-manager', displayName: 'Matthew', storageGb: 4000, region: 'home-lab' },
  { id: 'commitment-jessica-storage', provider: 'human-jessica-spouse', displayName: 'Jessica', storageGb: 2000, region: 'home-lab' },
  { id: 'commitment-pete-storage', provider: 'human-pete-pastor', displayName: 'Pete', storageGb: 1000, region: 'church-office' },
  { id: 'commitment-timothy-storage', provider: 'human-timothy-tutor', displayName: 'Timothy', storageGb: 500, region: 'university' },
  { id: 'commitment-frank-storage', provider: 'human-frank-farmer', displayName: 'Frank', storageGb: 2000, region: 'farm' },
];

export async function seedStorageCommitments(api: StorageApiService): Promise<void> {
  console.log(`[seed-commitments] Seeding ${PEER_COMMITMENTS.length} storage commitments...`);

  for (const peer of PEER_COMMITMENTS) {
    try {
      await api.createCommitment({
        id: peer.id,
        action: 'provide',
        provider: peer.provider,
        receiver: 'network',
        resourceConformsTo: 'storage-capacity',
        resourceQuantityValue: peer.storageGb,
        resourceQuantityUnit: 'GB',
        state: 'accepted',
        note: `${peer.displayName} storage node — ${peer.storageGb}GB committed (${peer.region})`,
      });
      console.log(`  [+] ${peer.displayName}: ${peer.storageGb}GB`);
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg.includes('UNIQUE') || msg.includes('already exists')) {
        console.log(`  [=] ${peer.displayName}: already exists`);
      } else {
        console.warn(`  [!] ${peer.displayName}: ${msg}`);
      }
    }
  }

  console.log('[seed-commitments] Done.');
}
```

- [ ] **Step 2: Wire into main seed flow**

In `genesis/seeder/src/seed.ts`, import and call `seedStorageCommitments` after stewardship seeding. Check the existing flow to find the right insertion point — it's typically after `seedStewardship()`.

- [ ] **Step 3: Check StorageApiService has createCommitment method**

Look in the seeder's API service. If `createCommitment()` doesn't exist, add it:

```typescript
async createCommitment(input: Record<string, unknown>): Promise<unknown> {
  return this.post('/db/commitments', input);
}
```

The HTTP route for commitments should already exist in elohim-storage. Verify with: `grep -n "commitments" elohim/elohim-storage/src/http.rs | head -20`

- [ ] **Step 4: Test seeding**

Run the seeder and verify commitments were created:

```bash
curl http://localhost:8888/db/commitments?action=provide | jq '.[] | {id, provider, resourceQuantityValue}'
```

Expected: 5 commitment records, one per peer.

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/seed-commitments.ts genesis/seeder/src/seed.ts
git commit -m "feat(seeder): seed REA storage commitments for 5 test peers

Each peer declares storage capacity as a REA commitment (action=provide).
Matthew: 4TB, Jessica: 2TB, Pete: 1TB, Timothy: 500GB, Frank: 2TB."
```

---

### Task 7: Network Tab — Resilience Section (Angular)

Add the resilience data display to the Network tab in content-viewer.

**Files:**
- Create: `app/elohim-app/src/app/lamad/services/resilience.service.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css`

- [ ] **Step 1: Create ResilienceService**

```typescript
import { HttpClient } from '@angular/common/http';
import { inject, Injectable } from '@angular/core';
import { Observable } from 'rxjs';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import type { ResilienceView } from '@elohim/storage-client/generated';

@Injectable({ providedIn: 'root' })
export class ResilienceService {
  private readonly http = inject(HttpClient);
  private readonly storageClient = inject(StorageClientService);

  getContentResilience(contentId: string): Observable<ResilienceView> {
    const baseUrl = this.storageClient.getStorageBaseUrl();
    return this.http.get<ResilienceView>(`${baseUrl}/api/v1/resilience/${contentId}`);
  }
}
```

- [ ] **Step 2: Add resilience data to content-viewer component**

In the component class, add:

```typescript
// Add import
import { ResilienceService } from '../../services/resilience.service';
import type { ResilienceView } from '@elohim/storage-client/generated';

// Add field
resilience: ResilienceView | null = null;

// Inject service
private readonly resilienceService = inject(ResilienceService);

// Add load method
private loadResilience(nodeId: string): void {
  this.resilienceService
    .getContentResilience(nodeId)
    .pipe(takeUntil(this.destroy$))
    .subscribe({
      next: resilience => {
        this.resilience = resilience;
      },
      error: () => {
        // Resilience is supplemental
      },
    });
}
```

Call `this.loadResilience(nodeId)` where other data loads are triggered (in the content load handler, alongside `loadStewardship`).

- [ ] **Step 3: Add resilience section to Network tab HTML**

Replace the Network tab section (lines 619-658) with:

```html
<!-- Network Tab -->
<section *ngSwitchCase="'network'" class="network-tab">
  <!-- Resilience Section -->
  <div class="resilience-section" data-testid="viewer-resilience-section">
    <div class="resilience-header">
      <h3>Resilience</h3>
      <span
        class="health-badge"
        [ngClass]="resilience?.health?.status || 'unknown'"
        data-testid="viewer-health-badge"
      >{{ (resilience?.health?.status || 'unknown') | titlecase }}</span>
    </div>

    <div class="resilience-grid" *ngIf="resilience; else loadingResilience">
      <div class="resilience-metric">
        <span class="metric-label">Encoding</span>
        <span class="metric-value">{{ resilience.encoding.strategy }}
          <span *ngIf="resilience.encoding.parityShards > 0">
            (can lose {{ resilience.health.canSurviveFailures }} peers)
          </span>
        </span>
      </div>

      <div class="resilience-metric">
        <span class="metric-label">Shards</span>
        <span class="metric-value">
          {{ resilience.distribution.shardsWithLocations }}/{{ resilience.distribution.totalShards }} distributed
        </span>
      </div>

      <div class="resilience-metric">
        <span class="metric-label">Peers</span>
        <span class="metric-value">{{ resilience.distribution.distinctPeers }} holding shards</span>
      </div>

      <div class="resilience-metric" *ngIf="resilience.stewardship.stewardCount > 0">
        <span class="metric-label">Stewards</span>
        <span class="metric-value">
          <span *ngFor="let a of resilience.stewardship.allocations; let last = last">
            {{ a.stewardPresenceId }} ({{ (a.allocationRatio * 100) | number:'1.0-0' }}%){{ last ? '' : ' · ' }}
          </span>
        </span>
      </div>

      <div class="resilience-metric" *ngIf="resilience.commitments.totalCommittedBytes > 0">
        <span class="metric-label">Storage Committed</span>
        <span class="metric-value">{{ resilience.commitments.totalCommittedBytes / 1073741824 | number:'1.0-0' }}GB</span>
      </div>
    </div>

    <ng-template #loadingResilience>
      <div class="resilience-loading">Loading resilience data...</div>
    </ng-template>
  </div>

  <!-- Attention Metrics -->
  <app-content-analytics
    *ngIf="node"
    [contentId]="node.id"
    data-testid="viewer-content-analytics"
  ></app-content-analytics>

  <div class="network-header">
    <h3>Content Neighborhood</h3>
    <p>Explore related concepts in the knowledge graph</p>
  </div>

  <div class="mini-graph-container">
    <app-mini-graph
      *ngIf="node"
      [focusNodeId]="node.id"
      [depth]="1"
      [maxNodes]="20"
      [height]="350"
      (nodeSelected)="onGraphNodeSelected($event)"
      (exploreRequested)="exploreInGraph()"
    ></app-mini-graph>
  </div>

  <div class="network-actions">
    <button
      type="button"
      class="btn-explore-graph"
      (click)="exploreInGraph()"
      data-testid="viewer-explore-graph"
    >
      <span class="icon">🔭</span>
      Explore in Full Graph
    </button>
  </div>
</section>
```

- [ ] **Step 4: Add resilience CSS styles**

Add to the component CSS:

```css
/* Resilience Section */
.resilience-section {
  background: var(--surface-elevated, #f8f9fa);
  border-radius: 12px;
  padding: 1.25rem;
  margin-bottom: 1.5rem;
}

.resilience-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 1rem;
}

.resilience-header h3 {
  margin: 0;
  font-size: 1rem;
}

.health-badge {
  font-size: 0.75rem;
  font-weight: 600;
  padding: 0.25rem 0.75rem;
  border-radius: 12px;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.health-badge.healthy { background: #dcfce7; color: #166534; }
.health-badge.degraded { background: #fef9c3; color: #854d0e; }
.health-badge.at_risk { background: #fee2e2; color: #991b1b; }
.health-badge.unknown { background: #f3f4f6; color: #6b7280; }

.resilience-grid {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.resilience-metric {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  font-size: 0.875rem;
}

.resilience-loading {
  font-size: 0.85rem;
  opacity: 0.5;
  padding: 0.5rem 0;
}
```

- [ ] **Step 5: Add proxy route for resilience endpoint**

In `app/elohim-app/proxy.conf.mjs`, ensure `/api/v1/resilience` is proxied. It likely already is via the `/api` catch-all. Verify.

- [ ] **Step 6: Verify in browser**

Navigate to a content page, click the Network tab. The resilience section should show encoding strategy, shard counts, peer counts, steward allocations, and health status.

- [ ] **Step 7: Commit**

```bash
git add app/elohim-app/src/app/lamad/services/resilience.service.ts app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(lamad): add resilience section to Network tab

Shows encoding strategy, shard distribution, peer count, steward
allocations, storage commitments, and health score from the
resilience API endpoint."
```

---

### Task 8: Doorway Proxy for Resilience Routes

**Files:**
- Modify: `doorway/doorway-service/src/routes/` or `doorway/doorway-service/src/http.rs`

- [ ] **Step 1: Add proxy rule for `/api/v1/resilience/*`**

Check how existing `/api/v1/` routes are proxied. The doorway likely has a catch-all proxy for `/api/v1/*` to elohim-storage. Verify:

```bash
grep -n "api/v1" doorway/doorway-service/src/http.rs | head -20
```

If `/api/v1/*` is already proxied, no change needed. If routes are explicit, add:

```rust
("GET", path) if path.starts_with("/api/v1/resilience/") => {
    self.proxy_to_storage(req).await
}
```

- [ ] **Step 2: Verify proxy works**

```bash
curl http://localhost:8888/api/v1/resilience/manifesto-foundations | jq
```

Expected: Resilience JSON response.

- [ ] **Step 3: Commit (if changes needed)**

```bash
git add doorway/doorway-service/src/
git commit -m "feat(doorway): proxy /api/v1/resilience/* to storage"
```

---

### Summary: Sprint B Delivers

After all 8 tasks:
- `shard_manifests` table records encoding strategy per content on ingest
- `shard_locations` table tracks which peers hold which shards (ready for Sprint C to populate)
- `GET /api/v1/resilience/{content_id}` returns full resilience projection
- REA storage commitments seeded for 5 test peers
- Network tab shows resilience data: encoding, shards, peers, stewards, commitments, health
- Doorway proxies the new endpoint
