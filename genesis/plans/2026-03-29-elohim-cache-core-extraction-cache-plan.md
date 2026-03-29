# Elohim Cache Core — Extraction Cache Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Generalize holochain-cache-core to elohim-cache-core, add a disk-backed extraction cache for HTML5 apps, and wire it into elohim-storage to eliminate SQLite pool exhaustion and OOM on concurrent app file requests.

**Architecture:** Three-layer model — truth layer (BlobStore, uniform), cache layer (ExtractionCache with CacheBackend trait, diverse per peer config), client layer (future). The extraction cache uses DiskBackend to serve extracted ZIP files from disk with TTL and budget eviction.

**Tech Stack:** Rust (tokio, async-trait, serde), elohim-cache-core crate, elohim-storage integration. TypeScript rename is mechanical.

**Design doc:** `genesis/plans/2026-03-29-elohim-cache-core-extraction-cache-design.md`

---

## Task 1: Rename Crate — Move and Update Paths

This is a mechanical rename. No logic changes. Do all file moves and path updates, then verify compilation.

**Files:**
- Move: `elohim/holochain/holochain-cache-core/` → `elohim/elohim-cache-core/`
- Modify: `elohim/elohim-cache-core/Cargo.toml` (package name)
- Modify: `doorway/doorway-service/Cargo.toml:97` (dependency path)
- Modify: `doorway/doorway-service/Dockerfile:19,30` (COPY + sed paths)
- Modify: `doorway/doorway-service/src/cache/store.rs:17` (use statement)
- Modify: `doorway/doorway-service/src/cache/resolution.rs:35` (use statement)
- Modify: `genesis/orchestrator/Jenkinsfile:31` (DNA changePatterns)

**Step 1: Move the directory**

```bash
git mv elohim/holochain/holochain-cache-core elohim/elohim-cache-core
```

**Step 2: Update Cargo.toml package name**

In `elohim/elohim-cache-core/Cargo.toml`, change:
```toml
name = "holochain-cache-core"
description = "High-performance O(log n) cache for Holochain/Elohim applications"
```
to:
```toml
name = "elohim-cache-core"
description = "High-performance O(log n) cache for Elohim Protocol applications"
```

**Step 3: Update doorway dependency path**

In `doorway/doorway-service/Cargo.toml:97`, change:
```toml
holochain-cache-core = { path = "../../elohim/holochain/holochain-cache-core" }
```
to:
```toml
elohim-cache-core = { path = "../../elohim/elohim-cache-core" }
```

**Step 4: Update doorway Rust imports**

In `doorway/doorway-service/src/cache/store.rs:17`, change:
```rust
use holochain_cache_core::BlobCache;
```
to:
```rust
use elohim_cache_core::BlobCache;
```

In `doorway/doorway-service/src/cache/resolution.rs:35`, change:
```rust
use holochain_cache_core::{ContentResolver, SourceTier};
```
to:
```rust
use elohim_cache_core::{ContentResolver, SourceTier};
```

Search for any other `holochain_cache_core` references in `doorway/doorway-service/src/` and update them all.

**Step 5: Update doorway Dockerfile**

In `doorway/doorway-service/Dockerfile:19`, change:
```dockerfile
COPY elohim/holochain/holochain-cache-core ./elohim/holochain/holochain-cache-core
```
to:
```dockerfile
COPY elohim/elohim-cache-core ./elohim/elohim-cache-core
```

In `doorway/doorway-service/Dockerfile:30`, change the sed command:
```dockerfile
sed -i 's|path = "../../elohim/holochain/holochain-cache-core"|path = "elohim/holochain/holochain-cache-core"|' Cargo.toml
```
to:
```dockerfile
sed -i 's|path = "../../elohim/elohim-cache-core"|path = "elohim/elohim-cache-core"|' Cargo.toml
```

**Step 6: Update orchestrator changeset patterns**

In `genesis/orchestrator/Jenkinsfile:31`, the DNA pipeline changePatterns contains `'elohim/holochain/holochain-cache-core/'`. Change to `'elohim/elohim-cache-core/'`.

Also add `'elohim/elohim-cache-core/'` to the Edge pipeline changePatterns at line 40 (since elohim-storage will depend on it).

**Step 7: Update crate doc comment**

In `elohim/elohim-cache-core/src/lib.rs:1`, change:
```rust
//! Holochain Cache Core - High-Performance Content-Reach Aware Cache
```
to:
```rust
//! Elohim Cache Core - High-Performance Content-Reach Aware Cache
```

**Step 8: Verify doorway compiles**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check
```

Expected: compiles with no errors.

**Step 9: Verify cache-core tests pass**

```bash
cd elohim/elohim-cache-core && cargo test
```

Expected: all existing tests pass.

**Step 10: Commit**

```bash
git add -A
git commit -m "refactor: rename holochain-cache-core to elohim-cache-core

Move from elohim/holochain/ to elohim/ — this crate is the protocol's
caching substrate, not Holochain-specific. Update all dependency paths
in doorway, Dockerfile, and orchestrator."
```

---

## Task 2: Add Native Feature + Extraction Module Skeleton

The existing cache-core compiles to WASM. The new extraction modules need tokio (native-only). Feature-gate them.

**Files:**
- Modify: `elohim/elohim-cache-core/Cargo.toml` (add dependencies)
- Create: `elohim/elohim-cache-core/src/extraction/mod.rs`
- Modify: `elohim/elohim-cache-core/src/lib.rs` (add module)

**Step 1: Add native feature and dependencies to Cargo.toml**

Add to `elohim/elohim-cache-core/Cargo.toml`:

```toml
[features]
default = []
native = ["dep:tokio", "dep:async-trait"]

[dependencies]
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
async-trait = { version = "0.1", optional = true }
tokio = { version = "1", features = ["fs", "sync"], optional = true }
thiserror = "1"
```

Note: `thiserror` is unconditional (error types are always available).

**Step 2: Create extraction module skeleton**

Create `elohim/elohim-cache-core/src/extraction/mod.rs`:

```rust
//! Extraction Cache — disk-backed cache for rendered/extracted content
//!
//! Requires the `native` feature (not available in WASM builds).
//!
//! ## Architecture
//!
//! ```text
//! Truth Layer (BlobStore)  →  Cache Layer (ExtractionCache)  →  Serve
//!   ZIP blob on disk           Extracted files on disk           HTTP response
//!   Content-addressed          TTL + budget governed             No DB, no ZIP
//! ```

mod backend;
mod disk;
mod cache;
mod error;

pub use backend::CacheBackend;
pub use disk::DiskBackend;
pub use cache::{ExtractionCache, ExtractionCacheConfig, AppCacheEntry};
pub use error::CacheError;
```

Create stub files for each submodule (will be filled in Tasks 3-5):

`elohim/elohim-cache-core/src/extraction/error.rs`:
```rust
//! Cache error types

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Budget exceeded: limit {limit} bytes, requested {requested} bytes")]
    BudgetExceeded { limit: u64, requested: u64 },

    #[error("Invalid cache key: {0}")]
    InvalidKey(String),

    #[error("Cache backend unavailable")]
    BackendUnavailable,
}
```

`elohim/elohim-cache-core/src/extraction/backend.rs`:
```rust
//! CacheBackend trait — pluggable storage for extraction cache
```

`elohim/elohim-cache-core/src/extraction/disk.rs`:
```rust
//! DiskBackend — filesystem-backed cache storage
```

`elohim/elohim-cache-core/src/extraction/cache.rs`:
```rust
//! ExtractionCache — TTL + budget governed cache wrapping a CacheBackend
```

**Step 3: Register module in lib.rs**

Add to `elohim/elohim-cache-core/src/lib.rs` after the existing module exports:

```rust
#[cfg(feature = "native")]
pub mod extraction;
```

**Step 4: Verify it compiles (both modes)**

```bash
cd elohim/elohim-cache-core
# Without native feature (WASM-compatible path)
cargo check
# With native feature
cargo check --features native
```

Expected: both compile with no errors.

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(cache-core): add native feature + extraction module skeleton

Feature-gated behind 'native' — WASM builds unaffected.
Modules: CacheBackend trait, DiskBackend, ExtractionCache, CacheError."
```

---

## Task 3: CacheBackend Trait

**Files:**
- Modify: `elohim/elohim-cache-core/src/extraction/backend.rs`

**Step 1: Write the trait with doc tests**

```rust
//! CacheBackend trait — pluggable storage for extraction cache
//!
//! Implementations provide the actual storage mechanism (disk, memory, etc.).
//! The trait is async because some backends (MongoDB, S3) need it.

use async_trait::async_trait;
use super::CacheError;

/// Pluggable storage backend for cached content.
///
/// Keys are forward-slash-separated paths (e.g., `"app_id/js/main.js"`).
/// Implementations must be Send + Sync for use in async HTTP handlers.
#[async_trait]
pub trait CacheBackend: Send + Sync {
    /// Get cached bytes by key. Returns None on miss.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    /// Store bytes at key. Returns true if this was a new entry (not overwrite).
    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError>;

    /// Delete a single entry. Returns true if it existed.
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;

    /// Delete all entries whose key starts with prefix.
    /// Used to evict an entire app: `delete_prefix("my-app/")`.
    /// Returns count of entries deleted.
    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError>;

    /// Check if key exists without reading bytes.
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Total size of all cached data in bytes.
    async fn total_size(&self) -> Result<u64, CacheError>;
}
```

**Step 2: Verify it compiles**

```bash
cd elohim/elohim-cache-core && cargo check --features native
```

Expected: compiles (no implementors yet, that's fine).

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(cache-core): define CacheBackend trait

Async trait with get/put/delete/delete_prefix/exists/total_size.
Pluggable storage backend for extraction cache."
```

---

## Task 4: DiskBackend Implementation

**Files:**
- Modify: `elohim/elohim-cache-core/src/extraction/disk.rs`

**Step 1: Write failing tests**

Add to `elohim/elohim-cache-core/src/extraction/disk.rs`:

```rust
//! DiskBackend — filesystem-backed cache storage
//!
//! Maps cache keys to file paths: `{root_dir}/{key}`.
//! Keys use forward slashes, mapped to OS path separators.

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;
use super::backend::CacheBackend;
use super::CacheError;

/// Filesystem-backed cache storage.
///
/// Stores cached files under a root directory. Keys are path-like
/// strings (e.g., `"my-app/js/main.js"`) mapped directly to the filesystem.
pub struct DiskBackend {
    root_dir: PathBuf,
}

impl DiskBackend {
    /// Create a new disk backend rooted at the given directory.
    /// Creates the directory if it doesn't exist.
    pub async fn new(root_dir: PathBuf) -> Result<Self, CacheError> {
        fs::create_dir_all(&root_dir).await?;
        Ok(Self { root_dir })
    }

    /// Resolve a cache key to a filesystem path.
    /// Validates key to prevent path traversal.
    fn resolve_path(&self, key: &str) -> Result<PathBuf, CacheError> {
        if key.contains("..") || key.contains('\0') || key.starts_with('/') {
            return Err(CacheError::InvalidKey(key.to_string()));
        }
        Ok(self.root_dir.join(key))
    }
}

#[async_trait]
impl CacheBackend for DiskBackend {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let path = self.resolve_path(key)?;
        match fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn put(&self, key: &str, data: Vec<u8>) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        let is_new = !path.exists();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&path, &data).await?;
        Ok(is_new)
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        match fs::remove_file(&path).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn delete_prefix(&self, prefix: &str) -> Result<u64, CacheError> {
        let path = self.resolve_path(prefix.trim_end_matches('/'))?;
        if !path.exists() {
            return Ok(0);
        }
        // Count files before deleting
        let count = count_files_recursive(&path).await;
        match fs::remove_dir_all(&path).await {
            Ok(()) => Ok(count),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(0),
            Err(e) => Err(CacheError::Io(e)),
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let path = self.resolve_path(key)?;
        Ok(fs::try_exists(&path).await.unwrap_or(false))
    }

    async fn total_size(&self) -> Result<u64, CacheError> {
        Ok(dir_size_recursive(&self.root_dir).await)
    }
}

/// Count files recursively in a directory.
async fn count_files_recursive(dir: &std::path::Path) -> u64 {
    let mut count = 0u64;
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(ft) = entry.file_type().await {
                if ft.is_file() {
                    count += 1;
                } else if ft.is_dir() {
                    count += Box::pin(count_files_recursive(&entry.path())).await;
                }
            }
        }
    }
    count
}

/// Sum file sizes recursively in a directory.
async fn dir_size_recursive(dir: &std::path::Path) -> u64 {
    let mut size = 0u64;
    if let Ok(mut entries) = fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            if let Ok(ft) = entry.file_type().await {
                if ft.is_file() {
                    if let Ok(meta) = entry.metadata().await {
                        size += meta.len();
                    }
                } else if ft.is_dir() {
                    size += Box::pin(dir_size_recursive(&entry.path())).await;
                }
            }
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn temp_backend() -> (DiskBackend, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let backend = DiskBackend::new(tmp.path().to_path_buf()).await.unwrap();
        (backend, tmp)
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let (backend, _tmp) = temp_backend().await;
        let is_new = backend.put("app1/index.html", b"<html>hello</html>".to_vec()).await.unwrap();
        assert!(is_new);

        let data = backend.get("app1/index.html").await.unwrap();
        assert_eq!(data, Some(b"<html>hello</html>".to_vec()));
    }

    #[tokio::test]
    async fn test_get_missing_returns_none() {
        let (backend, _tmp) = temp_backend().await;
        let data = backend.get("nonexistent/file.js").await.unwrap();
        assert_eq!(data, None);
    }

    #[tokio::test]
    async fn test_put_overwrite_returns_false() {
        let (backend, _tmp) = temp_backend().await;
        backend.put("app1/index.html", b"v1".to_vec()).await.unwrap();
        let is_new = backend.put("app1/index.html", b"v2".to_vec()).await.unwrap();
        assert!(!is_new);

        let data = backend.get("app1/index.html").await.unwrap();
        assert_eq!(data, Some(b"v2".to_vec()));
    }

    #[tokio::test]
    async fn test_delete() {
        let (backend, _tmp) = temp_backend().await;
        backend.put("app1/style.css", b"body{}".to_vec()).await.unwrap();
        assert!(backend.delete("app1/style.css").await.unwrap());
        assert!(!backend.delete("app1/style.css").await.unwrap()); // already gone
        assert_eq!(backend.get("app1/style.css").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_delete_prefix() {
        let (backend, _tmp) = temp_backend().await;
        backend.put("app1/index.html", b"html".to_vec()).await.unwrap();
        backend.put("app1/js/main.js", b"js".to_vec()).await.unwrap();
        backend.put("app1/css/style.css", b"css".to_vec()).await.unwrap();
        backend.put("app2/index.html", b"other".to_vec()).await.unwrap();

        let deleted = backend.delete_prefix("app1").await.unwrap();
        assert_eq!(deleted, 3);

        assert_eq!(backend.get("app1/index.html").await.unwrap(), None);
        assert_eq!(backend.get("app2/index.html").await.unwrap(), Some(b"other".to_vec()));
    }

    #[tokio::test]
    async fn test_exists() {
        let (backend, _tmp) = temp_backend().await;
        assert!(!backend.exists("app1/index.html").await.unwrap());
        backend.put("app1/index.html", b"html".to_vec()).await.unwrap();
        assert!(backend.exists("app1/index.html").await.unwrap());
    }

    #[tokio::test]
    async fn test_total_size() {
        let (backend, _tmp) = temp_backend().await;
        backend.put("a/1.txt", b"hello".to_vec()).await.unwrap(); // 5 bytes
        backend.put("a/2.txt", b"world!".to_vec()).await.unwrap(); // 6 bytes

        let size = backend.total_size().await.unwrap();
        assert_eq!(size, 11);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (backend, _tmp) = temp_backend().await;
        assert!(matches!(
            backend.put("../escape/file", b"bad".to_vec()).await,
            Err(CacheError::InvalidKey(_))
        ));
        assert!(matches!(
            backend.get("../../etc/passwd").await,
            Err(CacheError::InvalidKey(_))
        ));
    }
}
```

**Step 2: Add tempfile dev-dependency**

In `elohim/elohim-cache-core/Cargo.toml`:

```toml
[dev-dependencies]
wasm-bindgen-test = "0.3"
tempfile = "3"
tokio = { version = "1", features = ["full", "macros", "test-util"] }
```

**Step 3: Run tests to verify they pass**

```bash
cd elohim/elohim-cache-core && cargo test --features native -- extraction::disk
```

Expected: all 7 tests pass.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(cache-core): implement DiskBackend with tests

Filesystem-backed CacheBackend: keys map to paths under root dir.
Path traversal protection. 7 tests covering put/get/delete/prefix/size."
```

---

## Task 5: ExtractionCache Implementation

**Files:**
- Modify: `elohim/elohim-cache-core/src/extraction/cache.rs`

**Step 1: Write ExtractionCache with tests**

```rust
//! ExtractionCache — TTL + budget governed cache wrapping a CacheBackend
//!
//! Manages extracted app files with:
//! - TTL-based expiry (hot items stay, cold items expire)
//! - Budget enforcement (evict LRA apps when over budget)
//! - Hash-based invalidation (stale extractions auto-evict)

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use super::backend::CacheBackend;
use super::CacheError;

/// Configuration for the extraction cache.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionCacheConfig {
    /// Whether the extraction cache is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Maximum total cache size in bytes
    #[serde(default = "default_budget")]
    pub budget_bytes: u64,
    /// Time-to-live in seconds for cached extractions
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
    /// Directory for cached extractions
    #[serde(default)]
    pub cache_dir: PathBuf,
}

fn default_enabled() -> bool { true }
fn default_budget() -> u64 { 512 * 1024 * 1024 } // 512 MB
fn default_ttl() -> u64 { 3600 } // 1 hour

impl Default for ExtractionCacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            budget_bytes: default_budget(),
            ttl_secs: default_ttl(),
            cache_dir: PathBuf::new(), // Must be set by caller
        }
    }
}

/// Metadata for a cached app extraction.
#[derive(Debug, Clone)]
pub struct AppCacheEntry {
    /// Blob hash this extraction was derived from
    pub blob_hash: String,
    /// When the extraction was cached (unix timestamp secs)
    pub extracted_at: u64,
    /// Last time any file from this app was accessed (unix timestamp secs)
    pub last_accessed: u64,
    /// Total size of all extracted files in bytes
    pub total_size: u64,
}

/// Extraction cache — serves pre-extracted content from a CacheBackend.
///
/// Wraps a `CacheBackend` with an in-memory index for fast lookups,
/// TTL-based expiry, budget enforcement, and blob hash validation.
pub struct ExtractionCache {
    backend: Box<dyn CacheBackend>,
    index: RwLock<HashMap<String, AppCacheEntry>>,
    config: ExtractionCacheConfig,
}

impl ExtractionCache {
    /// Create a new extraction cache with the given backend and config.
    pub fn new(backend: Box<dyn CacheBackend>, config: ExtractionCacheConfig) -> Self {
        Self {
            backend,
            index: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Check if an app's extraction is current (matching hash, not expired).
    pub async fn is_current(&self, app_id: &str, blob_hash: &str) -> bool {
        let index = self.index.read().await;
        match index.get(app_id) {
            Some(entry) => {
                entry.blob_hash == blob_hash && !self.is_expired(entry)
            }
            None => false,
        }
    }

    /// Get a cached file from an extracted app.
    ///
    /// Returns the file bytes and MIME content-type, or None on cache miss.
    /// Touches the app's last_accessed timestamp on hit.
    pub async fn get_file(&self, app_id: &str, file_path: &str) -> Option<Vec<u8>> {
        // Check index first (fast path, read lock)
        {
            let index = self.index.read().await;
            match index.get(app_id) {
                Some(entry) if !self.is_expired(entry) => {}
                _ => return None,
            }
        }

        let key = format!("{}/{}", app_id, file_path);
        match self.backend.get(&key).await {
            Ok(Some(data)) => {
                // Touch last_accessed
                let mut index = self.index.write().await;
                if let Some(entry) = index.get_mut(app_id) {
                    entry.last_accessed = now_secs();
                }
                Some(data)
            }
            _ => None,
        }
    }

    /// Cache all extracted files for an app.
    ///
    /// `files` is a vec of (relative_path, bytes) pairs from ZIP extraction.
    /// Enforces budget — may evict other apps to make room.
    pub async fn put_app(
        &self,
        app_id: &str,
        blob_hash: &str,
        files: Vec<(String, Vec<u8>)>,
    ) -> Result<(), CacheError> {
        let total_size: u64 = files.iter().map(|(_, data)| data.len() as u64).sum();

        // Check single-app budget
        if total_size > self.config.budget_bytes {
            return Err(CacheError::BudgetExceeded {
                limit: self.config.budget_bytes,
                requested: total_size,
            });
        }

        // Evict to make room if needed
        self.enforce_budget(total_size).await?;

        // Evict any stale extraction for this app
        self.evict_app(app_id).await?;

        // Write all files
        for (path, data) in &files {
            let key = format!("{}/{}", app_id, path);
            self.backend.put(&key, data.clone()).await?;
        }

        // Update index
        let now = now_secs();
        let mut index = self.index.write().await;
        index.insert(app_id.to_string(), AppCacheEntry {
            blob_hash: blob_hash.to_string(),
            extracted_at: now,
            last_accessed: now,
            total_size,
        });

        Ok(())
    }

    /// Evict an app's cached files.
    pub async fn evict_app(&self, app_id: &str) -> Result<(), CacheError> {
        let mut index = self.index.write().await;
        if index.remove(app_id).is_some() {
            // delete_prefix expects the app directory prefix
            let prefix = format!("{}/", app_id);
            self.backend.delete_prefix(&prefix).await?;
        }
        Ok(())
    }

    /// Get cache statistics.
    pub async fn stats(&self) -> ExtractionCacheStats {
        let index = self.index.read().await;
        let cached_apps = index.len() as u32;
        let total_cached_bytes: u64 = index.values().map(|e| e.total_size).sum();
        ExtractionCacheStats {
            cached_apps,
            total_cached_bytes,
            budget_bytes: self.config.budget_bytes,
            ttl_secs: self.config.ttl_secs,
        }
    }

    // --- Private helpers ---

    fn is_expired(&self, entry: &AppCacheEntry) -> bool {
        let age = now_secs().saturating_sub(entry.last_accessed);
        age > self.config.ttl_secs
    }

    /// Evict least-recently-accessed apps until there's room for `needed_bytes`.
    async fn enforce_budget(&self, needed_bytes: u64) -> Result<(), CacheError> {
        let mut index = self.index.write().await;
        let current_size: u64 = index.values().map(|e| e.total_size).sum();

        if current_size + needed_bytes <= self.config.budget_bytes {
            return Ok(());
        }

        // Sort by last_accessed ascending (oldest first)
        let mut apps: Vec<(String, u64, u64)> = index
            .iter()
            .map(|(id, e)| (id.clone(), e.last_accessed, e.total_size))
            .collect();
        apps.sort_by_key(|(_, accessed, _)| *accessed);

        let mut freed = 0u64;
        let target = (current_size + needed_bytes).saturating_sub(self.config.budget_bytes);

        for (app_id, _, size) in &apps {
            if freed >= target {
                break;
            }
            let prefix = format!("{}/", app_id);
            self.backend.delete_prefix(&prefix).await?;
            index.remove(app_id);
            freed += size;
        }

        Ok(())
    }
}

/// Cache statistics snapshot.
#[derive(Debug, Clone)]
pub struct ExtractionCacheStats {
    pub cached_apps: u32,
    pub total_cached_bytes: u64,
    pub budget_bytes: u64,
    pub ttl_secs: u64,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::disk::DiskBackend;

    async fn test_cache(ttl: u64, budget: u64) -> (ExtractionCache, tempfile::TempDir) {
        let tmp = tempfile::tempdir().unwrap();
        let backend = DiskBackend::new(tmp.path().to_path_buf()).await.unwrap();
        let config = ExtractionCacheConfig {
            enabled: true,
            budget_bytes: budget,
            ttl_secs: ttl,
            cache_dir: tmp.path().to_path_buf(),
        };
        (ExtractionCache::new(Box::new(backend), config), tmp)
    }

    fn sample_files() -> Vec<(String, Vec<u8>)> {
        vec![
            ("index.html".into(), b"<html>hello</html>".to_vec()),
            ("js/main.js".into(), b"console.log('hi')".to_vec()),
            ("css/style.css".into(), b"body { margin: 0 }".to_vec()),
        ]
    }

    #[tokio::test]
    async fn test_put_and_get_file() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;
        cache.put_app("app1", "hash-abc", sample_files()).await.unwrap();

        let html = cache.get_file("app1", "index.html").await;
        assert_eq!(html, Some(b"<html>hello</html>".to_vec()));

        let js = cache.get_file("app1", "js/main.js").await;
        assert_eq!(js, Some(b"console.log('hi')".to_vec()));
    }

    #[tokio::test]
    async fn test_get_missing_app_returns_none() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;
        assert_eq!(cache.get_file("nonexistent", "index.html").await, None);
    }

    #[tokio::test]
    async fn test_is_current_with_matching_hash() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;
        cache.put_app("app1", "hash-abc", sample_files()).await.unwrap();

        assert!(cache.is_current("app1", "hash-abc").await);
        assert!(!cache.is_current("app1", "hash-different").await);
        assert!(!cache.is_current("nonexistent", "hash-abc").await);
    }

    #[tokio::test]
    async fn test_evict_app() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;
        cache.put_app("app1", "hash-abc", sample_files()).await.unwrap();

        cache.evict_app("app1").await.unwrap();
        assert_eq!(cache.get_file("app1", "index.html").await, None);
        assert!(!cache.is_current("app1", "hash-abc").await);
    }

    #[tokio::test]
    async fn test_hash_mismatch_triggers_re_extraction() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;
        cache.put_app("app1", "hash-v1", sample_files()).await.unwrap();

        // Simulate re-seed with new hash
        assert!(!cache.is_current("app1", "hash-v2").await);

        // Put with new hash evicts old
        let new_files = vec![("index.html".into(), b"<html>v2</html>".to_vec())];
        cache.put_app("app1", "hash-v2", new_files).await.unwrap();

        assert!(cache.is_current("app1", "hash-v2").await);
        assert_eq!(cache.get_file("app1", "index.html").await, Some(b"<html>v2</html>".to_vec()));
    }

    #[tokio::test]
    async fn test_budget_eviction() {
        // Budget of 100 bytes
        let (cache, _tmp) = test_cache(3600, 100).await;

        // App1: 50 bytes
        let files1 = vec![("data.bin".into(), vec![0u8; 50])];
        cache.put_app("app1", "h1", files1).await.unwrap();

        // App2: 60 bytes — exceeds budget, should evict app1
        let files2 = vec![("data.bin".into(), vec![1u8; 60])];
        cache.put_app("app2", "h2", files2).await.unwrap();

        // App1 should be evicted
        assert_eq!(cache.get_file("app1", "data.bin").await, None);
        // App2 should exist
        assert!(cache.get_file("app2", "data.bin").await.is_some());
    }

    #[tokio::test]
    async fn test_over_budget_single_app_rejected() {
        let (cache, _tmp) = test_cache(3600, 50).await;

        // 100 bytes > 50 byte budget
        let files = vec![("data.bin".into(), vec![0u8; 100])];
        let result = cache.put_app("app1", "h1", files).await;
        assert!(matches!(result, Err(CacheError::BudgetExceeded { .. })));
    }

    #[tokio::test]
    async fn test_stats() {
        let (cache, _tmp) = test_cache(3600, 1024 * 1024).await;

        let stats = cache.stats().await;
        assert_eq!(stats.cached_apps, 0);
        assert_eq!(stats.total_cached_bytes, 0);

        cache.put_app("app1", "h1", sample_files()).await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.cached_apps, 1);
        assert!(stats.total_cached_bytes > 0);
    }
}
```

**Step 2: Run tests**

```bash
cd elohim/elohim-cache-core && cargo test --features native -- extraction::cache
```

Expected: all 8 tests pass.

**Step 3: Commit**

```bash
git add -A
git commit -m "feat(cache-core): implement ExtractionCache with tests

TTL-based expiry, LRA budget eviction, hash-based invalidation.
8 tests covering put/get/evict/budget/stats/hash-mismatch."
```

---

## Task 6: Add elohim-cache-core Dependency to elohim-storage

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/src/lib.rs`
- Modify: `elohim/elohim-storage/src/config.rs`

**Step 1: Add dependency**

In `elohim/elohim-storage/Cargo.toml`, add:

```toml
# Protocol cache substrate
elohim-cache-core = { path = "../elohim-cache-core", features = ["native"] }
```

**Step 2: Add ExtractionCacheConfig to storage Config**

In `elohim/elohim-storage/src/config.rs`, add the import and field:

```rust
use elohim_cache_core::extraction::ExtractionCacheConfig;
```

Add to `Config` struct (after existing fields):

```rust
    /// Extraction cache for HTML5 apps and rendered content
    #[serde(default)]
    pub extraction_cache: ExtractionCacheConfig,
```

In `impl Default for Config`, add:

```rust
            extraction_cache: ExtractionCacheConfig {
                enabled: true,
                budget_bytes: 512 * 1024 * 1024, // 512 MB
                ttl_secs: 3600,
                cache_dir: PathBuf::new(), // Set from storage_dir at runtime
            },
```

Add a helper method:

```rust
    /// Get extraction cache directory (defaults to {storage_dir}/cache/extractions)
    pub fn extraction_cache_dir(&self) -> PathBuf {
        if self.extraction_cache.cache_dir.as_os_str().is_empty() {
            self.storage_dir.join("cache").join("extractions")
        } else {
            self.extraction_cache.cache_dir.clone()
        }
    }
```

**Step 3: Verify it compiles**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles.

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(storage): add elohim-cache-core dependency + ExtractionCacheConfig

Config gets extraction_cache field with defaults (512MB budget, 1hr TTL).
Cache dir defaults to {storage_dir}/cache/extractions."
```

---

## Task 7: app_index + Extraction Cache in HttpServer

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (struct + constructor + builder + handler)

**Step 1: Add fields to HttpServer struct**

At `http.rs:117-138`, add two new fields to `HttpServer`:

```rust
    /// Extraction cache for HTML5 app files (None = disabled)
    extraction_cache: Option<Arc<elohim_cache_core::extraction::ExtractionCache>>,
    /// In-memory index: appId → blobHash (avoids per-request SQLite scan)
    app_index: Arc<RwLock<std::collections::HashMap<String, String>>>,
```

Add the necessary imports at the top of `http.rs`:

```rust
use elohim_cache_core::extraction::{ExtractionCache, ExtractionCacheConfig};
```

**Step 2: Update constructor**

In `HttpServer::new()` at line 167, add:

```rust
            extraction_cache: None,
            app_index: Arc::new(RwLock::new(std::collections::HashMap::new())),
```

**Step 3: Add builder methods**

After the existing `with_*` methods (around line 234):

```rust
    /// Set the extraction cache
    pub fn with_extraction_cache(mut self, cache: Arc<ExtractionCache>) -> Self {
        self.extraction_cache = Some(cache);
        self
    }

    /// Load the app index from database (call after db_pool is set)
    pub async fn load_app_index(&self) -> Result<(), StorageError> {
        let conn = &mut self.get_conn()?;
        let app_ctx = db::AppContext::default_lamad();
        let query = ContentQuery {
            content_format: Some("html5-app".to_string()),
            limit: 100,
            ..Default::default()
        };
        let items = db::content_diesel::list_content(conn, &app_ctx, &query)?;

        let mut index = self.app_index.write().await;
        index.clear();
        for item in items {
            if let Some(ref content_body) = item.content.content_body {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                    if let Some(app_id) = obj.get("appId").and_then(|v| v.as_str()) {
                        let blob_hash = item.content.blob_hash.clone().unwrap_or_default();
                        if !blob_hash.is_empty() {
                            info!(app_id = %app_id, blob_hash = %blob_hash, "Indexed HTML5 app");
                            index.insert(app_id.to_string(), blob_hash);
                        }
                    }
                }
            }
        }
        info!(count = index.len(), "App index loaded");
        Ok(())
    }
```

**Step 4: Rewrite handle_app_request with cache path**

Replace the entire `handle_app_request` method (lines 2646-2889) with the cached version. The structure:

```rust
    async fn handle_app_request(&self, path: &str) -> Result<Response<Full<Bytes>>, StorageError> {
        use std::io::Read;
        use zip::ZipArchive;

        // Parse path: /apps/{app_id}/{file_path}
        let remainder = path.strip_prefix("/apps/").unwrap_or("");
        let (app_id, file_path) = match remainder.find('/') {
            Some(pos) => (&remainder[..pos], &remainder[pos + 1..]),
            None => (remainder, "index.html"),
        };

        if app_id.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Missing app_id"}"#)))
                .unwrap());
        }

        // Validate file_path for path traversal
        if file_path.contains("..") || file_path.contains('\0') || file_path.starts_with('/') {
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(r#"{"error": "Invalid file path"}"#)))
                .unwrap());
        }

        debug!(app_id = %app_id, file_path = %file_path, "App file request");

        // --- Fast path: check extraction cache ---
        let blob_hash = {
            let index = self.app_index.read().await;
            index.get(app_id).cloned()
        };

        if let (Some(ref cache), Some(ref hash)) = (&self.extraction_cache, &blob_hash) {
            // Check if cache is current for this app + hash
            if cache.is_current(app_id, hash).await {
                if let Some(data) = cache.get_file(app_id, file_path).await {
                    let content_type = Self::get_mime_type(file_path);
                    debug!(app_id = %app_id, file_path = %file_path, "Cache HIT");
                    return Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, data.len())
                        .header(header::CACHE_CONTROL, "public, max-age=3600")
                        .header("X-App-Id", app_id)
                        .header("X-Cache", "HIT")
                        .body(Full::new(Bytes::from(data)))
                        .unwrap());
                }
            }
        }

        // --- Slow path: DB lookup + ZIP extraction ---
        debug!(app_id = %app_id, "Cache MISS — extracting from ZIP");

        // If we don't have the blob_hash from index, do the DB query
        let blob_hash = match blob_hash {
            Some(h) => h,
            None => {
                // Fall back to DB query (populates index for next time)
                let hash = self.lookup_app_blob_hash(app_id).await?;
                match hash {
                    Some(h) => h,
                    None => {
                        return Ok(Response::builder()
                            .status(StatusCode::NOT_FOUND)
                            .header(header::CONTENT_TYPE, "application/json")
                            .body(Full::new(Bytes::from(format!(
                                r#"{{"error": "App not found: {}"}}"#, app_id
                            ))))
                            .unwrap());
                    }
                }
            }
        };

        if blob_hash.is_empty() {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Full::new(Bytes::from(
                    r#"{"error": "App ZIP not available (no blob_hash)"}"#
                )))
                .unwrap());
        }

        // Fetch ZIP from blob store
        let zip_data = match self.blob_store.get(&blob_hash).await {
            Ok(data) => data,
            Err(StorageError::NotFound(_)) => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "App ZIP blob not found: {}"}}"#, blob_hash
                    ))))
                    .unwrap());
            }
            Err(e) => return Err(e),
        };

        // Extract ALL files from ZIP and cache them
        let cursor = std::io::Cursor::new(&zip_data);
        let mut archive = match ZipArchive::new(cursor) {
            Ok(a) => a,
            Err(e) => {
                warn!(error = %e, "Invalid ZIP archive");
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "Invalid ZIP archive: {}"}}"#, e
                    ))))
                    .unwrap());
            }
        };

        // Extract all files for caching
        let mut all_files: Vec<(String, Vec<u8>)> = Vec::new();
        let mut requested_file_data: Option<Vec<u8>> = None;
        let normalized_path = file_path.trim_start_matches('/');

        for i in 0..archive.len() {
            if let Ok(mut f) = archive.by_index(i) {
                if f.is_dir() {
                    continue;
                }
                let name = f.name().to_string();
                let mut contents = Vec::new();
                if f.read_to_end(&mut contents).is_ok() {
                    // Check if this is the requested file
                    if name == normalized_path
                        || name.ends_with(normalized_path)
                        || name.ends_with(&format!("/{}", normalized_path))
                    {
                        requested_file_data = Some(contents.clone());
                    }
                    all_files.push((name, contents));
                }
            }
        }

        // Cache the extracted files (non-fatal if caching fails)
        if let Some(ref cache) = self.extraction_cache {
            if let Err(e) = cache.put_app(app_id, &blob_hash, all_files).await {
                warn!(error = %e, app_id = %app_id, "Failed to cache extraction (non-fatal)");
            }
        }

        // Serve the requested file
        let contents = match requested_file_data {
            Some(data) => data,
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Full::new(Bytes::from(format!(
                        r#"{{"error": "File not found in app: {}"}}"#, normalized_path
                    ))))
                    .unwrap());
            }
        };

        let content_type = Self::get_mime_type(file_path);

        info!(
            app_id = %app_id,
            file_path = %file_path,
            content_type = %content_type,
            size = contents.len(),
            "Serving app file (extracted + cached)"
        );

        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .header(header::CONTENT_LENGTH, contents.len())
            .header(header::CACHE_CONTROL, "public, max-age=3600")
            .header("X-App-Id", app_id)
            .header("X-Cache", "MISS")
            .body(Full::new(Bytes::from(contents)))
            .unwrap())
    }

    /// Look up blob hash for an app by querying DB and updating app_index.
    async fn lookup_app_blob_hash(&self, app_id: &str) -> Result<Option<String>, StorageError> {
        let mut conn = self.get_conn()?;
        let app_ctx = db::AppContext::default_lamad();
        let query = ContentQuery {
            content_format: Some("html5-app".to_string()),
            limit: 100,
            ..Default::default()
        };

        let items = db::content_diesel::list_content(&mut conn, &app_ctx, &query)?;
        let mut found_hash = None;

        for item in items {
            if let Some(ref content_body) = item.content.content_body {
                if let Ok(obj) = serde_json::from_str::<serde_json::Value>(content_body) {
                    if let Some(content_app_id) = obj.get("appId").and_then(|v| v.as_str()) {
                        let hash = item.content.blob_hash.clone().unwrap_or_default();
                        if !hash.is_empty() {
                            // Update index while we're here
                            let mut index = self.app_index.write().await;
                            index.insert(content_app_id.to_string(), hash.clone());
                        }
                        if content_app_id == app_id {
                            found_hash = item.content.blob_hash.clone();
                        }
                    }
                }
            }
        }

        Ok(found_hash)
    }
```

**Step 5: Verify it compiles**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles.

**Step 6: Commit**

```bash
git add -A
git commit -m "feat(storage): wire extraction cache into handle_app_request

Cache HIT path: O(1) index lookup + disk read. No DB, no ZIP, no pool.
Cache MISS path: extract all files, cache to disk, serve requested file.
Non-fatal cache errors fall through to ZIP extraction."
```

---

## Task 8: Wire Cache Initialization in main.rs

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs` (around line 386-389 where HttpServer is constructed)

**Step 1: Initialize extraction cache after config is loaded**

After the blob store initialization (line 254) and before HTTP server creation (line 388), add:

```rust
    // Initialize extraction cache if enabled
    let extraction_cache = if config.extraction_cache.enabled {
        let cache_dir = config.extraction_cache_dir();
        tokio::fs::create_dir_all(&cache_dir).await?;
        info!(dir = %cache_dir.display(), "Initializing extraction cache");

        let backend = elohim_cache_core::extraction::DiskBackend::new(cache_dir).await
            .map_err(|e| anyhow::anyhow!("Failed to create cache backend: {}", e))?;

        let mut cache_config = config.extraction_cache.clone();
        cache_config.cache_dir = config.extraction_cache_dir();

        let cache = Arc::new(elohim_cache_core::extraction::ExtractionCache::new(
            Box::new(backend),
            cache_config,
        ));
        info!(
            budget_mb = config.extraction_cache.budget_bytes / (1024 * 1024),
            ttl_secs = config.extraction_cache.ttl_secs,
            "Extraction cache enabled"
        );
        Some(cache)
    } else {
        info!("Extraction cache disabled");
        None
    };
```

**Step 2: Wire into HttpServer**

Where `HttpServer::new()` is called (line 388-389), chain the new builder:

```rust
    let mut http_server = HttpServer::new(blob_store.clone(), http_addr)
        .with_progress_hub(Arc::clone(&progress_hub));

    if let Some(ref cache) = extraction_cache {
        http_server = http_server.with_extraction_cache(Arc::clone(cache));
    }
```

**Step 3: Load app index after DB pool is set**

After the DB pool is configured on the server (search for `with_db_pool` or `with_services` in main.rs), add:

```rust
    // Load app index for HTML5 app caching
    if let Err(e) = http_server.load_app_index().await {
        warn!("Failed to load app index (non-fatal): {}", e);
    }
```

**Step 4: Verify it compiles**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(storage): initialize extraction cache on startup

DiskBackend created at {storage_dir}/cache/extractions.
App index loaded from DB on startup for O(1) appId → blobHash lookups."
```

---

## Task 9: Extend NodeCapabilities with cache_budget_bytes

**Files:**
- Modify: `elohim/elohim-storage/src/identity.rs:25-82`

**Step 1: Add field to NodeCapabilities**

At `identity.rs:25-36`, add the new field:

```rust
pub struct NodeCapabilities {
    pub storage: bool,
    pub always_on: bool,
    pub max_storage_bytes: u64,
    /// Cache budget for extraction/rendering cache (0 = disabled)
    pub cache_budget_bytes: u64,
    pub serve_family: bool,
    pub serve_public: bool,
}
```

**Step 2: Update Default impl**

In the `Default` impl (line 38-48), add:

```rust
            cache_budget_bytes: 512 * 1024 * 1024, // 512 MB default
```

**Step 3: Update profile constructors**

In `laptop()` (line 52-60):
```rust
            cache_budget_bytes: 200 * 1024 * 1024, // 200 MB
```

In `home_node()` (line 63-71):
```rust
            cache_budget_bytes: 2 * 1024 * 1024 * 1024, // 2 GB
```

In `network_node()` (line 74-82):
```rust
            cache_budget_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
```

**Step 4: Verify it compiles**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 5: Commit**

```bash
git add -A
git commit -m "feat(storage): add cache_budget_bytes to NodeCapabilities

Peer profiles declare extraction cache budget:
laptop=200MB, home_node=2GB, network_node=10GB.
Ready for resolution engine cache-aware routing (future sprint)."
```

---

## Task 10: TypeScript Rename (Mechanical)

**Files:**
- Modify: `app/elohim-app/package.json:85`
- Modify: `app/elohim-app/angular.json:50-51`
- Rename: `app/elohim-app/src/types/holochain-cache-core.d.ts` → `elohim-cache-core.d.ts`
- Rename: `app/elohim-library/projects/elohim-service/src/types/holochain-cache-core.d.ts` → `elohim-cache-core.d.ts`
- Modify: `app/elohim-library/projects/elohim-service/src/cache/reach-aware-cache.ts:765`
- Modify: `app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts:653`
- Modify: `app/elohim-library/projects/elohim-service/src/cache/write-buffer.ts:749`
- Modify: `app/elohim-library/projects/elohim-service/src/cache/types.ts:7`

**Step 1: Update package.json**

In `app/elohim-app/package.json:85`, change:
```json
"holochain-cache-core": "file:../../elohim/holochain/holochain-cache-core/pkg",
```
to:
```json
"elohim-cache-core": "file:../../elohim/elohim-cache-core/pkg",
```

**Step 2: Update angular.json**

In `app/elohim-app/angular.json:50-51`, change:
```json
"input": "node_modules/holochain-cache-core",
"output": "/wasm/holochain-cache-core"
```
to:
```json
"input": "node_modules/elohim-cache-core",
"output": "/wasm/elohim-cache-core"
```

**Step 3: Rename type declaration files**

```bash
git mv app/elohim-app/src/types/holochain-cache-core.d.ts app/elohim-app/src/types/elohim-cache-core.d.ts
git mv app/elohim-library/projects/elohim-service/src/types/holochain-cache-core.d.ts app/elohim-library/projects/elohim-service/src/types/elohim-cache-core.d.ts
```

Update the module declarations inside both files:
```typescript
declare module '/wasm/elohim-cache-core/elohim_cache_core.js' {
  export * from 'elohim-cache-core';
  export { default } from 'elohim-cache-core';
}

declare module 'elohim-cache-core' {
  // ... existing type exports ...
}
```

**Step 4: Update WASM import paths in cache TS files**

In `reach-aware-cache.ts:765`, change:
```typescript
const path = wasmPath || '/wasm/holochain-cache-core/holochain_cache_core.js';
```
to:
```typescript
const path = wasmPath || '/wasm/elohim-cache-core/elohim_cache_core.js';
```

In `content-resolver.ts:653`, change:
```typescript
const wasmPath = '/wasm/holochain-cache-core/holochain_cache_core.js';
```
to:
```typescript
const wasmPath = '/wasm/elohim-cache-core/elohim_cache_core.js';
```

In `write-buffer.ts:749`, change:
```typescript
const wasmPath = '/wasm/holochain-cache-core/holochain_cache_core.js';
```
to:
```typescript
const wasmPath = '/wasm/elohim-cache-core/elohim_cache_core.js';
```

In `types.ts:7`, update the doc comment:
```typescript
 * These interfaces mirror the WASM elohim-cache-core module,
```

**Step 5: Verify Angular build**

```bash
cd app/elohim-app && pnpm run build
```

Expected: builds successfully (WASM is optional, TS fallback works regardless of rename).

Note: if `pnpm install` is needed first due to the package.json change, run:
```bash
cd /projects/elohim && pnpm install
```

**Step 6: Commit**

```bash
git add -A
git commit -m "refactor: rename holochain-cache-core to elohim-cache-core in TypeScript

Update package.json, angular.json, type declarations, and WASM import
paths. TS fallback continues to work regardless of WASM availability."
```

---

## Task 11: Run Full Test Suite + Verify

**Step 1: Run cache-core tests**

```bash
cd elohim/elohim-cache-core && cargo test --features native
```

Expected: all tests pass (existing + new extraction tests).

**Step 2: Run doorway compilation**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo check
```

Expected: compiles.

**Step 3: Run elohim-storage compilation**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles.

**Step 4: Run elohim-storage tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins
```

Expected: existing tests pass.

**Step 5: Verify Angular lint**

```bash
cd app/elohim-app && pnpm run lint
```

Expected: no new lint errors from the rename.

**Step 6: Final commit if any cleanup needed**

If any compilation issues were found and fixed, commit the fixes.
