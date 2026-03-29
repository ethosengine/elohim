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
    /// Returns the file bytes, or None on cache miss.
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
