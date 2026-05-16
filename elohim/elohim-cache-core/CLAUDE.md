# Elohim Cache Core — Protocol Caching Substrate

The protocol's caching layer. Provides reach-aware LRU caching, tiered content resolution, write buffering for conductor protection, and disk-backed extraction caching for rendered content (HTML5 apps, etc.).

## Build & Test

```bash
# WASM-compatible path (no native features)
cargo check
cargo test

# Native path (includes extraction cache — requires tokio)
RUSTFLAGS="" cargo test --features native

# WASM build (requires wasm-pack)
wasm-pack build --target web --release
```

IMPORTANT: The system sets `RUSTFLAGS=--cfg getrandom_backend="custom"` for Holochain WASM. This breaks native builds for this crate. Always use `RUSTFLAGS=""` when building or testing natively.

## Architecture

### Two Compilation Targets

| Target | Features | Modules Available |
|--------|----------|-------------------|
| WASM (browser) | default | BlobCache, ChunkCache, ReachAwareCache, ContentResolver, WriteBuffer |
| Native (server/desktop) | `native` | All of above + extraction module (CacheBackend, DiskBackend, ExtractionCache) |

The `extraction` module is behind `#[cfg(feature = "native")]` because it requires tokio for async file I/O. WASM clients don't do disk caching.

### Module Map

```
src/
├── lib.rs              — BlobCache (LRU), ChunkCache (TTL), ReachAwareCache (8 isolated LRUs)
├── resolution.rs       — ContentResolver: tiered source resolution with learning + app registry
├── write_buffer.rs     — WriteBuffer: priority batching for conductor protection
└── extraction/         — [native only] Disk-backed extraction cache
    ├── backend.rs      — CacheBackend trait (async, pluggable storage)
    ├── disk.rs         — DiskBackend (filesystem implementation)
    ├── cache.rs        — ExtractionCache (TTL + budget + hash invalidation)
    └── error.rs        — CacheError
```

### Three-Layer Content Model

This crate serves the cache layer in the protocol's three-layer model:

```
Truth Layer (BlobStore)     — ZIP blobs, content-addressed, uniform across all peers
Cache Layer (this crate)    — Extracted/rendered content, TTL-governed, diverse per peer
Client Layer (future)       — Browser-side extraction when no cache peer is available
```

## Key Types

### Always Available (WASM + Native)

- **`BlobCache`** — O(log n) LRU with reach awareness, priority scoring. Used by doorway's projection cache.
- **`ChunkCache`** — TTL-based eviction for transient data (shards, sync chunks).
- **`ReachAwareCache`** — 8 independent LRU caches (private → commons). Prevents private content from evicting commons.
- **`ContentResolver`** — Tiered resolution (Local → Projection → Authoritative → External) with learning from history. Includes HTML5 app registry.
- **`WriteBuffer`** — Priority-based write batching with retry logic. Protects conductors during seeding/sync/recovery.

### Native Only (`feature = "native"`)

- **`CacheBackend`** — Async trait for pluggable storage backends (`get/put/delete/delete_prefix/exists/total_size`).
- **`DiskBackend`** — Filesystem implementation of CacheBackend. Keys map to paths. Path traversal protection built in.
- **`ExtractionCache`** — Wraps a CacheBackend with:
  - In-memory index (`HashMap<app_id, AppCacheEntry>`) for O(1) lookups
  - TTL-based expiry (configurable, default 1 hour)
  - Budget enforcement (evicts least-recently-accessed apps when over limit)
  - Hash-based invalidation (stale extractions auto-evict when blob hash changes)
- **`ExtractionCacheConfig`** — Serde-serializable config: `enabled`, `budget_bytes`, `ttl_secs`, `cache_dir`.

## Consumers

| Consumer | What it uses | Feature |
|----------|-------------|---------|
| doorway-service | `BlobCache`, `ContentResolver` | default (no native) |
| elohim-storage | `ExtractionCache`, `DiskBackend`, `ExtractionCacheConfig` | native |
| elohim-app (browser) | WASM build of BlobCache/ContentResolver/WriteBuffer | WASM target |
| elohim-library (TS) | Pure TypeScript mirror in `cache/` module (fallback when WASM unavailable) | N/A |

## Adding a New CacheBackend

Implement the `CacheBackend` trait:

```rust
#[async_trait]
impl CacheBackend for MyBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError>;
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;
    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError>;
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;
    async fn total_size(&self) -> Result<u64, CacheError>;
}
```

Then pass it to `ExtractionCache::new(Box::new(my_backend), config)`. The cache handles TTL, budget, and invalidation regardless of backend.

## Peer Diversity

The extraction cache budget varies by device — configured via `ExtractionCacheConfig.budget_bytes` and declared to the network via `NodeCapabilities.cache_budget_bytes` in elohim-storage. Constrained devices disable the cache entirely (`enabled: false`), and the protocol falls through to raw ZIP serving or client-side extraction.

## TypeScript Mirror

`app/elohim-library/projects/elohim-service/src/cache/` contains pure TypeScript implementations that mirror the WASM API. Browser clients try WASM first, fall back to TS transparently. The TS types live alongside the implementations in that same directory (`types.ts`, `content-resolver.ts`, `reach-aware-cache.ts`, `write-buffer.ts`). At runtime the WASM bundle is served by the host app under `/wasm/elohim-cache-core/` (generated by `wasm-pack build --target web --release`).
