# Resilient HTML5 App Delivery — Sprint 1: Doorway Projection Cache

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Cache extracted HTML5 app files in doorway's MongoDB so storage is never hit for repeat requests, solving the OOM/502 problem.

**Architecture:** Doorway's existing `/apps/` proxy route gains a MongoDB cache layer. On cache miss, proxy to storage and cache the response. On cache hit, return directly from MongoDB. Request coalescing prevents thundering herd on cold cache. Invalidation hooks into the existing projection subscriber.

**Tech Stack:** Rust (doorway-service), MongoDB (existing `MongoClient`), `DashMap` (in-flight coalescing), `tokio::sync::broadcast` (result broadcast)

**Design:** `genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md`

**A2O Scenarios:**
- `genesis/a2o/features/delivery/web2-absorption.feature` — 7 scenarios (all Sprint 1)
- `genesis/a2o/features/delivery/delivery-diagnostics.feature` — scenarios 1-6 (X-Cache headers, cache layer observability)

---

### Task 1: AppFileCache MongoDB Schema

**Files:**
- Create: `doorway/doorway-service/src/db/schemas/app_file_cache.rs`
- Modify: `doorway/doorway-service/src/db/schemas/mod.rs`

**Step 1: Write the schema file**

Create `doorway/doorway-service/src/db/schemas/app_file_cache.rs`:

```rust
//! App file cache schema
//!
//! Source of truth: ZIP blob in elohim-storage BlobStore (operational cache, Category C)
//! Reconstruction: delete collection, next request re-fetches from storage

use bson::{doc, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Cached file extracted from an HTML5 app ZIP blob.
///
/// Doorway caches these to absorb web2 traffic patterns (30+ concurrent
/// asset requests per page load) so elohim-storage stays focused on P2P.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppFileCache {
    /// MongoDB document ID (format: "{app_id}:{file_path}:{blob_hash}")
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<String>,

    /// App identifier (e.g., "evolution-of-trust")
    pub app_id: String,

    /// File path within the app (e.g., "pixi.min.js", "css/style.css")
    pub file_path: String,

    /// Blob hash of the source ZIP — the invalidation key.
    /// When content is re-seeded with a new blob_hash, all cached files
    /// for this app_id with the old hash are stale.
    pub blob_hash: String,

    /// EPR agreement authorizing this doorway to cache and serve this content.
    /// For self-operated nodes: a self-negotiated, self-accepted agreement.
    pub agreement_id: String,

    /// MIME content type (e.g., "application/javascript")
    pub content_type: String,

    /// Raw file bytes
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,

    /// When this file was cached
    pub cached_at: DateTime,

    /// Last time this file was served (for TTL eviction)
    pub last_accessed: DateTime,

    /// Standard metadata (soft delete, timestamps)
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for AppFileCache {
    fn default() -> Self {
        let now = DateTime::now();
        Self {
            mongo_id: None,
            app_id: String::new(),
            file_path: String::new(),
            blob_hash: String::new(),
            agreement_id: String::new(),
            content_type: String::new(),
            data: Vec::new(),
            cached_at: now,
            last_accessed: now,
            metadata: Metadata::default(),
        }
    }
}

impl IntoIndexes for AppFileCache {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Fast lookup by app_id (for invalidation: delete all files for an app)
            (doc! { "app_id": 1 }, None),
            // Compound: app + blob_hash (for bulk invalidation on re-seed)
            (doc! { "app_id": 1, "blob_hash": 1 }, None),
            // TTL index: auto-expire after 24 hours of no access
            (
                doc! { "last_accessed": 1 },
                Some(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(86400))
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for AppFileCache {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}
```

**Step 2: Register module in schemas/mod.rs**

Add to `doorway/doorway-service/src/db/schemas/mod.rs`:

```rust
pub mod app_file_cache;
pub use app_file_cache::AppFileCache;
```

**Step 3: Verify it compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: compiles with no errors

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/db/schemas/app_file_cache.rs doorway/doorway-service/src/db/schemas/mod.rs
git commit -m "feat(doorway): add AppFileCache MongoDB schema for projection cache"
```

---

### Task 2: AppFileCacheService — Cache Read/Write/Invalidate

**Files:**
- Create: `doorway/doorway-service/src/cache/app_file_cache.rs`
- Modify: `doorway/doorway-service/src/cache/mod.rs`

**Step 1: Write the failing test**

Add to the bottom of `app_file_cache.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let key = AppFileCacheService::cache_key("my-app", "js/main.js", "sha256-abc");
        assert_eq!(key, "my-app:js/main.js:sha256-abc");
    }

    #[test]
    fn test_in_flight_key_format() {
        let key = AppFileCacheService::in_flight_key("my-app", "js/main.js");
        assert_eq!(key, "apps:my-app:js/main.js");
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test app_file_cache 2>&1 | tail -10`
Expected: FAIL — `AppFileCacheService` not defined

**Step 3: Write the service**

Create `doorway/doorway-service/src/cache/app_file_cache.rs`:

```rust
//! App file projection cache service
//!
//! Caches extracted HTML5 app files in MongoDB so storage is never hit
//! for repeat requests. This is doorway's web2 absorption role —
//! protecting the P2P network from browser traffic patterns.
//!
//! Source of truth: ZIP blob in elohim-storage BlobStore (operational cache)

use std::sync::Arc;

use bson::{doc, DateTime};
use bytes::Bytes;
use dashmap::DashMap;
use tokio::sync::broadcast;
use tracing::{debug, info, warn};

use crate::db::mongo::MongoCollection;
use crate::db::schemas::AppFileCache;
use crate::db::MongoClient;

/// Cached file result returned to the handler
#[derive(Debug, Clone)]
pub struct CachedFile {
    pub data: Vec<u8>,
    pub content_type: String,
    pub blob_hash: String,
}

/// Service managing the app file projection cache in MongoDB.
///
/// Handles cache lookup, storage, invalidation, and request coalescing.
pub struct AppFileCacheService {
    /// MongoDB collection for cached files
    collection: MongoCollection<AppFileCache>,
    /// In-flight request tracker for coalescing (prevents thundering herd)
    in_flight: DashMap<String, broadcast::Sender<Option<CachedFile>>>,
    /// Default agreement_id for self-operated nodes
    agreement_id: String,
}

impl AppFileCacheService {
    /// Create a new cache service
    pub async fn new(mongo: &MongoClient, agreement_id: String) -> Result<Self, crate::types::DoorwayError> {
        let collection = mongo.collection::<AppFileCache>("app_file_cache").await?;
        Ok(Self {
            collection,
            in_flight: DashMap::new(),
            agreement_id,
        })
    }

    /// Build the MongoDB _id for a cached file
    pub fn cache_key(app_id: &str, file_path: &str, blob_hash: &str) -> String {
        format!("{app_id}:{file_path}:{blob_hash}")
    }

    /// Build the in-flight coalescing key (no blob_hash — dedupes across versions)
    pub fn in_flight_key(app_id: &str, file_path: &str) -> String {
        format!("apps:{app_id}:{file_path}")
    }

    /// Look up a cached file. Returns None on miss.
    /// Updates last_accessed on hit for TTL tracking.
    pub async fn get(&self, app_id: &str, file_path: &str, blob_hash: &str) -> Option<CachedFile> {
        let key = Self::cache_key(app_id, file_path, blob_hash);

        match self.collection.inner().find_one(doc! { "_id": &key }).await {
            Ok(Some(entry)) => {
                // Touch last_accessed (fire and forget)
                let collection = self.collection.inner().clone();
                let key_clone = key.clone();
                tokio::spawn(async move {
                    let _ = collection
                        .update_one(
                            doc! { "_id": &key_clone },
                            doc! { "$set": { "last_accessed": DateTime::now() } },
                        )
                        .await;
                });

                debug!(app_id, file_path, "App file cache HIT");
                Some(CachedFile {
                    data: entry.data,
                    content_type: entry.content_type,
                    blob_hash: entry.blob_hash,
                })
            }
            Ok(None) => {
                debug!(app_id, file_path, "App file cache MISS");
                None
            }
            Err(e) => {
                warn!(error = %e, app_id, file_path, "App file cache lookup failed");
                None
            }
        }
    }

    /// Cache a file. Overwrites if exists (upsert).
    pub async fn put(
        &self,
        app_id: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        data: Vec<u8>,
    ) {
        let key = Self::cache_key(app_id, file_path, blob_hash);
        let now = DateTime::now();

        let entry = AppFileCache {
            mongo_id: Some(key.clone()),
            app_id: app_id.to_string(),
            file_path: file_path.to_string(),
            blob_hash: blob_hash.to_string(),
            agreement_id: self.agreement_id.clone(),
            content_type: content_type.to_string(),
            data,
            cached_at: now,
            last_accessed: now,
            ..AppFileCache::default()
        };

        match self.collection.inner().replace_one(
            doc! { "_id": &key },
            &entry,
        ).upsert(true).await {
            Ok(_) => debug!(app_id, file_path, "Cached app file"),
            Err(e) => warn!(error = %e, app_id, file_path, "Failed to cache app file (non-fatal)"),
        }
    }

    /// Invalidate all cached files for an app (called on re-seed with new blob_hash).
    pub async fn invalidate_app(&self, app_id: &str) -> u64 {
        match self.collection.inner().delete_many(doc! { "app_id": app_id }).await {
            Ok(result) => {
                let count = result.deleted_count;
                if count > 0 {
                    info!(app_id, count, "Invalidated app file cache");
                }
                count
            }
            Err(e) => {
                warn!(error = %e, app_id, "Failed to invalidate app file cache");
                0
            }
        }
    }

    /// Begin an in-flight request. Returns:
    /// - None if this is the first request (caller should fetch from storage)
    /// - Some(receiver) if another request is already fetching (caller should wait)
    pub fn begin_fetch(&self, app_id: &str, file_path: &str) -> Option<broadcast::Receiver<Option<CachedFile>>> {
        let key = Self::in_flight_key(app_id, file_path);

        // Try to insert a new sender
        if let Some(existing) = self.in_flight.get(&key) {
            // Another request is already in-flight — subscribe to its result
            Some(existing.value().subscribe())
        } else {
            // We're first — create the broadcast channel
            let (tx, _) = broadcast::channel(1);
            self.in_flight.insert(key, tx);
            None
        }
    }

    /// Complete an in-flight request, broadcasting result to waiters.
    pub fn finish_fetch(&self, app_id: &str, file_path: &str, result: Option<CachedFile>) {
        let key = Self::in_flight_key(app_id, file_path);
        if let Some((_, tx)) = self.in_flight.remove(&key) {
            // Ignore send errors (no receivers = nobody waiting)
            let _ = tx.send(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let key = AppFileCacheService::cache_key("my-app", "js/main.js", "sha256-abc");
        assert_eq!(key, "my-app:js/main.js:sha256-abc");
    }

    #[test]
    fn test_in_flight_key_format() {
        let key = AppFileCacheService::in_flight_key("my-app", "js/main.js");
        assert_eq!(key, "apps:my-app:js/main.js");
    }
}
```

**Step 4: Register in cache/mod.rs**

Add to `doorway/doorway-service/src/cache/mod.rs` after existing module declarations:

```rust
pub mod app_file_cache;
pub use app_file_cache::{AppFileCacheService, CachedFile};
```

**Step 5: Run tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test app_file_cache -- --nocapture 2>&1 | tail -10`
Expected: 2 tests pass

**Step 6: Verify full compile**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: compiles clean

**Step 7: Commit**

```bash
git add doorway/doorway-service/src/cache/app_file_cache.rs doorway/doorway-service/src/cache/mod.rs
git commit -m "feat(doorway): add AppFileCacheService with coalescing and invalidation"
```

---

### Task 3: Wire AppFileCacheService into AppState

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (AppState struct + constructors)

**Step 1: Add the field to AppState**

In `http.rs`, add to `AppState` struct (after `delivery_relay` field):

```rust
    /// App file projection cache (MongoDB) — absorbs web2 traffic for HTML5 apps
    pub app_file_cache: Option<Arc<AppFileCacheService>>,
```

**Step 2: Initialize in constructors**

In `AppState::new()` (dev mode, no mongo): set `app_file_cache: None`

In `AppState::with_services()`: set `app_file_cache: None` (initialized later via `init_projection`)

In `AppState::with_projection()` (full production): after MongoDB client is available, add:

```rust
let app_file_cache = match &mongo {
    Some(m) => {
        match AppFileCacheService::new(m, "self-negotiated".to_string()).await {
            Ok(svc) => {
                info!("App file projection cache initialized");
                Some(Arc::new(svc))
            }
            Err(e) => {
                warn!(error = %e, "Failed to init app file cache (continuing without)");
                None
            }
        }
    }
    None => None,
};
```

Wire into AppState construction: `app_file_cache,`

**Step 3: Add import**

Add at top of `http.rs`:

```rust
use crate::cache::AppFileCacheService;
```

**Step 4: Verify compile**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: compiles (may have unused warning — that's fine, wired in Task 4)

**Step 5: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): wire AppFileCacheService into AppState"
```

---

### Task 4: Cache-First App Route Handler

**Files:**
- Modify: `doorway/doorway-service/src/routes/apps.rs`
- Modify: `doorway/doorway-service/src/server/http.rs` (route dispatch)

**Step 1: Rewrite apps.rs with cache-first logic**

Replace the contents of `doorway/doorway-service/src/routes/apps.rs` with a cache-first handler. The key changes:

1. `handle_app_request` now takes `Arc<AppState>` instead of just `storage_url`
2. Parse `app_id` and `file_path` from the path
3. If `app_file_cache` is available: check cache first
4. On cache miss: use coalescing, proxy to storage, cache result
5. On cache hit: return directly

The function signature changes to:

```rust
pub async fn handle_app_request(
    _req: Request<Incoming>,
    state: Arc<AppState>,
    path: &str,
) -> Response<Full<Bytes>>
```

Parse path: split `/apps/{app_id}/{file_path}` into components. Minimum 3 segments (apps, app_id, at least one file segment).

Cache-first flow:
1. If `state.app_file_cache` is Some, try `cache.get(app_id, file_path, blob_hash)`
   - Need blob_hash: query content projection by app_id to get current blob_hash
   - Cache this mapping in a DashMap on AppState for fast path (same pattern as storage's app_index)
2. Cache HIT: return with `X-Cache: HIT` header
3. Cache MISS: `cache.begin_fetch()` for coalescing
   - If another request in-flight: wait for broadcast, return with `X-Cache: HIT-COALESCED`
   - If we're first: proxy to storage, cache result, `finish_fetch()`, return with `X-Cache: MISS`

**Step 2: Update route dispatch in http.rs**

Change the `/apps/` match arm (line ~1270) to pass `state` instead of just `storage_url`:

```rust
(Method::GET, p) if p.starts_with("/apps/") => {
    debug!(path = %p, "Handling app request (projection cache)");
    return Ok(to_boxed(
        routes::handle_app_request(req, Arc::clone(&state), p).await,
    ));
}
```

**Step 3: Verify compile**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -10`

**Step 4: Run existing tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -15`
Expected: all existing tests pass

**Step 5: Commit**

```bash
git add doorway/doorway-service/src/routes/apps.rs doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): cache-first app route handler with request coalescing"
```

---

### Task 5: Blob Hash Resolution (App Index)

**Files:**
- Modify: `doorway/doorway-service/src/cache/app_file_cache.rs`

The cache needs to know the current `blob_hash` for an `app_id` to construct cache keys. Two approaches:

1. **Startup load**: Query content projection for all `content_format == "html5-app"` entries, extract `blob_hash` from content body JSON, build `HashMap<app_id, blob_hash>` (same pattern as storage's `load_app_index`)
2. **Lazy resolution**: On first request for an app_id, query content projection, cache the mapping

**Step 1: Add app_index to the service**

Add to `AppFileCacheService`:

```rust
    /// app_id → blob_hash mapping (populated from content projection)
    app_index: Arc<tokio::sync::RwLock<HashMap<String, String>>>,
```

**Step 2: Add load_app_index method**

Queries the content projection collection for `content_format == "html5-app"`, parses `content_body` JSON for `appId` and `blobHash` fields, populates the index.

**Step 3: Add resolve_blob_hash method**

Public method that checks the index first, falls back to a projection query on miss, updates the index.

**Step 4: Add refresh_app method**

Called by invalidation hook — removes an app_id from the index so next request re-resolves.

**Step 5: Run tests and commit**

```bash
git commit -m "feat(doorway): app index for blob hash resolution in projection cache"
```

---

### Task 6: Projection Subscriber Invalidation Hook

**Files:**
- Modify: `doorway/doorway-service/src/projection/subscriber.rs` (or the projection engine)

**Step 1: Find where content upsert signals are processed**

The projection subscriber already receives content update signals. When a content signal arrives with `content_format == "html5-app"`:

1. Extract `app_id` from the content body
2. Call `app_file_cache.invalidate_app(app_id)` to clear stale cached files
3. Call `app_file_cache.refresh_app(app_id)` to clear the blob hash index entry

**Step 2: Wire the invalidation**

The `AppFileCacheService` reference needs to be passed to the projection subscriber (via Arc, same pattern as other services).

**Step 3: Test invalidation flow**

Write a test that:
1. Puts a cached file
2. Calls invalidate_app
3. Verifies get returns None

**Step 4: Commit**

```bash
git commit -m "feat(doorway): invalidate app file cache on content re-seed signal"
```

---

### Task 7: Update Alpha Manifest (Resource Limits)

**Files:**
- Modify: `genesis/orchestrator/manifests/edgenode/alpha.yaml`

**Step 1: Update storage container resource limits**

Change storage memory limit from 256Mi to 1Gi and CPU from 250m to 500m (matching the kubectl patch already applied).

**Step 2: Commit**

```bash
git add genesis/orchestrator/manifests/edgenode/alpha.yaml
git commit -m "fix(infra): persist storage 1Gi memory limit in alpha manifest"
```

---

### Task 8: Integration Verification

**Step 1: Run full doorway test suite**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20`
Expected: all tests pass, including new app_file_cache tests

**Step 2: Run clippy**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -10`
Expected: no warnings

**Step 3: Run fmt check**

Run: `cd doorway/doorway-service && cargo fmt --check 2>&1 | tail -5`
Expected: no formatting issues

**Step 4: Final commit if any fixes needed**

---

## Testing Notes

- MongoDB is required for integration tests. Unit tests use the cache key/coalescing logic which doesn't need MongoDB.
- The `in_flight` coalescing can be tested without MongoDB by verifying `begin_fetch`/`finish_fetch` behavior with broadcast channels.
- The invalidation flow can be tested by asserting `invalidate_app` calls delete_many with the correct filter.
- End-to-end verification on alpha: after deploying, load Evolution of Trust twice. First load should show `X-Cache: MISS` headers. Second load should show `X-Cache: HIT` on all assets. Storage pod should show zero app-serving requests on second load.

## Key Files Reference

| File | Purpose |
|------|--------|
| `doorway/doorway-service/src/db/schemas/app_file_cache.rs` | MongoDB schema (Task 1) |
| `doorway/doorway-service/src/cache/app_file_cache.rs` | Cache service (Task 2) |
| `doorway/doorway-service/src/server/http.rs` | AppState wiring + route dispatch (Tasks 3-4) |
| `doorway/doorway-service/src/routes/apps.rs` | Cache-first handler (Task 4) |
| `doorway/doorway-service/src/projection/subscriber.rs` | Invalidation hook (Task 6) |
| `genesis/orchestrator/manifests/edgenode/alpha.yaml` | Resource limits (Task 7) |
| `doorway/doorway-service/src/db/mongo.rs` | MongoClient, IntoIndexes, MutMetadata traits |
| `doorway/doorway-service/src/projection/collections/content.rs` | ContentProjection (for app_index queries) |
