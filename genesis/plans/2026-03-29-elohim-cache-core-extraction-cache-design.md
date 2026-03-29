# Elohim Cache Core — Extraction Cache Design

## Problem

HTML5 apps (like Evolution of Trust) are served via `/apps/{app_id}/{file_path}`. Every file request (JS, CSS, images — 30+ concurrent on page load) triggers:

1. SQLite query — grabs pool connection, scans all html5-app content to find matching appId
2. Reads 6.75MB ZIP from blob store on disk
3. Decompresses ZIP in memory
4. Extracts single file from archive

This causes SQLite pool exhaustion (10-connection pool vs 30+ concurrent requests) and memory pressure (30 x 6.75MB concurrent decompressions in a 256Mi container).

## Root Cause Files

- `elohim/elohim-storage/src/http.rs:2646` — `handle_app_request`: per-request SQLite query + ZIP decompress
- `elohim/elohim-storage/src/db/mod.rs:103` — pool: 10 connections, 30s timeout
- `genesis/orchestrator/manifests/edgenode/alpha.yaml:277` — 256Mi container limit

## Design Principles

### Three-Layer Model with Peer Diversity

The ZIP blob is the EPR artifact. Extraction is projection. Different peers have different serving capabilities, but all peers store the truth.

```
Truth Layer (uniform)  — Every peer stores the blob. Replication/recovery/resilience.
Cache Layer (diverse)  — Config-driven. Peers declare cache budget. Hot items stay, cold expire.
Client Layer (future)  — When no cached peer is reachable, client extracts.
```

Peer diversity is expressed through configuration, not hardcoded roles:

| Peer type | Truth | Cache | Budget |
|-----------|-------|-------|--------|
| Doorway (server) | yes | DiskBackend, long TTL | 2GB |
| Desktop (Tauri) | yes | DiskBackend, medium TTL | 200MB |
| NAS/storage node | yes | DiskBackend, long TTL | 10GB |
| Phone | yes | Disabled | 0 |
| Constrained IoT | yes | Disabled | 0 |

Budget and TTL are configuration — derived from the device, not the role.

## Architecture

### Crate Rename

`elohim/holochain/holochain-cache-core/` → `elohim/elohim-cache-core/`

Package name: `elohim-cache-core`. This is the protocol's caching substrate, not Holochain-specific. Existing primitives (BlobCache, ChunkCache, ReachAwareCache, ContentResolver, WriteBuffer) move unchanged.

### CacheBackend Trait

```rust
#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError>;
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;
    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError>;
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;
    async fn size(&self) -> Result<u64, CacheError>;
}
```

### DiskBackend (first implementation)

Keys map to filesystem paths: `{cache_dir}/{key}` where key = `{app_id}/{file_path}`.

- `get` = `tokio::fs::read`
- `put` = `tokio::fs::create_dir_all` + `tokio::fs::write`
- `delete_prefix` = `tokio::fs::remove_dir_all`
- `exists` = `tokio::fs::try_exists`
- `size` = directory walk, sum file sizes (cached in-memory, refreshed periodically)

### ExtractionCache

Wraps `CacheBackend` with TTL, budget eviction, and hash-based invalidation.

```rust
pub struct ExtractionCache {
    backend: Box<dyn CacheBackend>,
    index: RwLock<HashMap<String, AppCacheEntry>>,
    config: ExtractionCacheConfig,
}

struct AppCacheEntry {
    blob_hash: String,
    extracted_at: u64,
    last_accessed: u64,
    total_size: u64,
}
```

**Operations:**
- `get_file(app_id, file_path)` → `Option<(Vec<u8>, &str)>` — cache hit or miss
- `put_app(app_id, blob_hash, files)` — cache all extracted files for an app
- `evict_app(app_id)` — remove an app's cache (re-seed, budget pressure)
- `is_current(app_id, blob_hash)` → bool — hash match + TTL check

**Eviction strategy:**
- TTL expiry: checked on access, stale entries evicted on next request
- Budget enforcement: when total size exceeds budget, evict least-recently-accessed apps
- Hash mismatch: if stored blob_hash differs from current, evict stale extraction

### ExtractionCacheConfig

```rust
pub struct ExtractionCacheConfig {
    pub enabled: bool,           // false on constrained devices
    pub budget_bytes: u64,       // derived from device resources
    pub ttl_secs: u64,           // how long hot items stay
    pub cache_dir: PathBuf,      // defaults to {storage_dir}/cache/extractions
}
```

Defaults: `enabled: true`, `budget_bytes: 512MB`, `ttl_secs: 3600`, `cache_dir: {storage_dir}/cache/extractions`.

### Integration into elohim-storage

**HttpServer additions:**

```rust
pub struct HttpServer {
    blob_store: Arc<BlobStore>,
    extraction_cache: Option<Arc<ExtractionCache>>,  // None = disabled
    app_index: Arc<RwLock<HashMap<String, String>>>,  // appId → blobHash
    // ... existing fields ...
}
```

**app_index** — eliminates per-request SQLite scan:
- On startup: one query loads all html5-app content → `HashMap<appId, blobHash>`
- On seed/update: refresh the index
- On request: O(1) lookup, no DB connection needed

**Revised handle_app_request flow:**

```
1. Parse path → (app_id, file_path)
2. If extraction_cache is Some:
   a. cache.is_current(app_id, app_index[app_id])? → cache.get_file(app_id, file_path)
   b. Cache hit → serve from disk. No DB. No ZIP. No pool connection.
   c. Cache miss → extract ZIP once → cache.put_app() → serve file
3. If extraction_cache is None:
   a. Current behavior (ZIP decompress per request)
   b. Future: return raw ZIP blob, client extracts
```

**Invalidation:** Hash comparison is the mechanism. When content is re-seeded with a new blob_hash, `is_current()` returns false, next request triggers re-extraction. No explicit invalidation signal needed.

### NodeCapabilities Extension

Existing `NodeCapabilities` in `elohim-storage/src/identity.rs` already declares peer profiles (laptop, home_node, network_node) with `max_storage_bytes`. Extend with:

```rust
pub struct NodeCapabilities {
    pub storage: bool,
    pub always_on: bool,
    pub max_storage_bytes: u64,
    pub cache_budget_bytes: u64,     // NEW: extraction cache budget
    pub serve_family: bool,
    pub serve_public: bool,
}
```

Built-in profiles updated:
- `laptop()` → `cache_budget_bytes: 200MB`
- `home_node()` → `cache_budget_bytes: 2GB`
- `network_node()` → `cache_budget_bytes: 10GB`

This data is available for the resolution engine's `calculate_priority()` when cache-aware routing is wired (future sprint). Existing `CapacityAnnouncement` gossipsub infrastructure can carry it.

## Error Handling

Cache errors are non-fatal. A miss or backend failure falls through to the existing ZIP-decompress path.

```rust
pub enum CacheError {
    Io(std::io::Error),
    BudgetExceeded { limit: u64, requested: u64 },
    InvalidKey(String),
    BackendUnavailable,
}
```

The handler never returns 500 because the cache is broken — it just gets slower.

## Rename Scope

### Rust
- `elohim/elohim-cache-core/Cargo.toml` — package name `elohim-cache-core`
- `doorway/doorway-service/Cargo.toml` — path: `../../elohim/elohim-cache-core`
- `doorway/doorway-service/Dockerfile` — COPY path
- `doorway/doorway-service/src/cache/store.rs` + `resolution.rs` — `use elohim_cache_core::`
- `elohim/elohim-storage/Cargo.toml` — new dependency on `elohim-cache-core`

### TypeScript
- `app/elohim-app/package.json` — `file:` path
- `app/elohim-app/angular.json` — WASM asset copy path
- `app/elohim-app/src/types/holochain-cache-core.d.ts` — module declarations
- `app/elohim-library/.../types/holochain-cache-core.d.ts` — module declarations
- `app/elohim-library/.../cache/reach-aware-cache.ts` — WASM import path
- `app/elohim-library/.../cache/content-resolver.ts` — WASM import path
- `app/elohim-library/.../cache/write-buffer.ts` — WASM import path
- `app/elohim-library/.../cache/types.ts` — doc comment

### CI/Pipeline
- Orchestrator changeset patterns if `elohim/elohim-cache-core/` isn't covered by existing `elohim/` patterns

## Testing

| Test | Verifies |
|------|----------|
| DiskBackend unit tests | put/get/delete/delete_prefix/exists/size on temp dirs |
| ExtractionCache unit tests | TTL expiry, budget eviction, hash mismatch invalidation |
| handle_app_request integration | Cache hit (no DB), cache miss (extract + cache), disabled path |
| app_index test | Startup load, hash mismatch detection |
| Existing doorway proxy test | Unchanged — doorway forwards, storage serves faster |

## Acceptance Criteria

1. Loading Evolution of Trust on alpha produces zero HTTP 500 errors
2. Storage container stays well under 256Mi during concurrent app file requests
3. After first load, subsequent requests serve from disk cache without ZIP decompression
4. No `timed out waiting for connection` errors during app load
5. Cache invalidates correctly when app content is re-seeded with a new blob hash

## Deferred

| Item | Rationale |
|------|-----------|
| Client-side extraction (Service Worker + JSZip) | Separate concern; needs Angular + Tauri work |
| MongoBackend for doorway projection | Doorway has its own cache infra; revisit when needed |
| WASM build pipeline for renamed crate | TS fallback works; WASM is optional optimization |
| Budget auto-detection from device resources | Start config-driven, learn heuristics |
| Resolution engine cache-aware routing | `calculate_priority()` needs new input; `NodeCapabilities.cache_budget_bytes` is ready for it |
| CapacityAnnouncement gossipsub wiring | Broadcast-only today; separate sprint to consume announcements |
