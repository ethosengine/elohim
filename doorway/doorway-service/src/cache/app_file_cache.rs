//! App File Cache Service
//!
//! Manages a MongoDB-backed cache of extracted HTML5 app files. When a browser
//! loads an HTML5 app (e.g., a Sophia quiz), it fires 30+ concurrent requests
//! for JS/CSS/image assets. This service absorbs that web2 traffic pattern so
//! elohim-storage stays focused on P2P.
//!
//! ## In-flight Coalescing
//!
//! When multiple requests arrive for the same file before the first fetch
//! completes, only one fetch is made to elohim-storage. All other requests
//! wait on a broadcast channel and receive the result when it arrives.
//!
//! ## Invalidation
//!
//! Files are keyed by `{app_id}:{file_path}:{blob_hash}`. When a new
//! blob_hash arrives (re-seed), the old entries become unreachable. MongoDB's
//! TTL index on `last_accessed` garbage-collects stale entries after 24h.
//! `invalidate_app()` provides immediate bulk purge when needed.

use bson::{doc, DateTime};
use dashmap::DashMap;
use mongodb::options::ReplaceOptions;
use tokio::sync::broadcast;
use tracing::{debug, error, warn};

use crate::db::mongo::MongoClient;
use crate::db::schemas::{AppFileCacheDoc, APP_FILE_CACHE_COLLECTION};

// =============================================================================
// CachedFile — the public return type
// =============================================================================

/// A cached file ready to serve over HTTP
#[derive(Debug, Clone)]
pub struct CachedFile {
    /// Raw file bytes
    pub data: Vec<u8>,
    /// MIME content type
    pub content_type: String,
    /// Blob hash of the source zip bundle
    pub blob_hash: String,
}

// =============================================================================
// AppFileCacheService
// =============================================================================

/// Service managing the MongoDB-backed app file cache with in-flight coalescing.
///
/// Designed for concurrent access from the HTTP handler layer — all methods
/// are safe to call from multiple tokio tasks simultaneously.
pub struct AppFileCacheService {
    /// MongoDB client for persistent cache storage
    mongo: MongoClient,

    /// EPR agreement ID authorizing cache entries
    agreement_id: String,

    /// In-flight fetch coalescing: prevents thundering herd when many
    /// requests arrive for the same file before the first fetch completes.
    /// Key format: "apps:{app_id}:{file_path}"
    in_flight: DashMap<String, broadcast::Sender<Option<CachedFile>>>,
}

impl AppFileCacheService {
    /// Create a new app file cache service.
    ///
    /// The `agreement_id` is the EPR agreement authorizing this doorway to
    /// cache extracted app files. For self-hosted doorways this is typically
    /// "self-negotiated".
    pub fn new(mongo: &MongoClient, agreement_id: String) -> Self {
        Self {
            mongo: mongo.clone(),
            agreement_id,
            in_flight: DashMap::new(),
        }
    }

    /// Build the MongoDB `_id` for a cached file.
    ///
    /// Format: `{app_id}:{file_path}:{blob_hash}`
    pub fn cache_key(app_id: &str, file_path: &str, blob_hash: &str) -> String {
        format!("{app_id}:{file_path}:{blob_hash}")
    }

    /// Build the in-flight coalescing key for a file being fetched.
    ///
    /// Format: `apps:{app_id}:{file_path}`
    ///
    /// Note: this is blob_hash-independent because we only ever fetch the
    /// latest version of a file — the blob_hash is determined by the fetch.
    pub fn in_flight_key(app_id: &str, file_path: &str) -> String {
        format!("apps:{app_id}:{file_path}")
    }

    /// Look up a cached file by app_id, file_path, and blob_hash.
    ///
    /// On cache hit, updates `last_accessed` in a fire-and-forget spawn
    /// to keep the TTL index fresh without blocking the caller.
    pub async fn get(&self, app_id: &str, file_path: &str, blob_hash: &str) -> Option<CachedFile> {
        let mongo_id = Self::cache_key(app_id, file_path, blob_hash);

        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<AppFileCacheDoc>(APP_FILE_CACHE_COLLECTION);

        let doc = match collection.find_one(doc! { "_id": &mongo_id }).await {
            Ok(Some(doc)) => doc,
            Ok(None) => return None,
            Err(e) => {
                warn!(mongo_id = %mongo_id, error = %e, "App file cache lookup failed");
                return None;
            }
        };

        // Fire-and-forget: touch last_accessed to reset TTL
        let touch_collection = db.collection::<AppFileCacheDoc>(APP_FILE_CACHE_COLLECTION);
        let touch_id = mongo_id.clone();
        tokio::spawn(async move {
            let update = doc! {
                "$set": { "last_accessed": DateTime::now() }
            };
            if let Err(e) = touch_collection
                .update_one(doc! { "_id": &touch_id }, update)
                .await
            {
                debug!(mongo_id = %touch_id, error = %e, "Failed to touch last_accessed");
            }
        });

        Some(CachedFile {
            data: doc.data,
            content_type: doc.content_type,
            blob_hash: doc.blob_hash,
        })
    }

    /// Insert or update a cached file.
    ///
    /// Uses upsert semantics — if the file already exists with the same
    /// composite key, it is replaced.
    pub async fn put(
        &self,
        app_id: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        data: Vec<u8>,
    ) {
        let doc = AppFileCacheDoc::new(
            app_id.to_string(),
            file_path.to_string(),
            blob_hash.to_string(),
            self.agreement_id.clone(),
            content_type.to_string(),
            data,
        );

        let mongo_id = doc
            .mongo_id
            .clone()
            .expect("AppFileCacheDoc::new always sets mongo_id");

        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<AppFileCacheDoc>(APP_FILE_CACHE_COLLECTION);

        let options = ReplaceOptions::builder().upsert(true).build();

        match collection
            .replace_one(doc! { "_id": &mongo_id }, doc)
            .with_options(options)
            .await
        {
            Ok(_) => {
                debug!(mongo_id = %mongo_id, "App file cached");
            }
            Err(e) => {
                error!(mongo_id = %mongo_id, error = %e, "Failed to cache app file");
            }
        }
    }

    /// Delete all cached files for an app.
    ///
    /// Returns the number of documents deleted.
    pub async fn invalidate_app(&self, app_id: &str) -> u64 {
        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<AppFileCacheDoc>(APP_FILE_CACHE_COLLECTION);

        match collection.delete_many(doc! { "app_id": app_id }).await {
            Ok(result) => {
                let count = result.deleted_count;
                if count > 0 {
                    debug!(app_id = %app_id, count = count, "Invalidated app file cache");
                }
                count
            }
            Err(e) => {
                error!(app_id = %app_id, error = %e, "Failed to invalidate app file cache");
                0
            }
        }
    }

    /// Begin an in-flight fetch for a file.
    ///
    /// If another task is already fetching this file, returns
    /// `Some(Receiver)` — the caller should await the receiver instead of
    /// fetching again.
    ///
    /// If no fetch is in progress, returns `None` — the caller is the
    /// "leader" and should perform the fetch, then call `finish_fetch()`.
    pub fn begin_fetch(
        &self,
        app_id: &str,
        file_path: &str,
    ) -> Option<broadcast::Receiver<Option<CachedFile>>> {
        let key = Self::in_flight_key(app_id, file_path);

        // Check if already in flight
        if let Some(sender) = self.in_flight.get(&key) {
            return Some(sender.subscribe());
        }

        // Register as the leader
        let (tx, _) = broadcast::channel(1);
        self.in_flight.insert(key, tx);
        None
    }

    /// Complete an in-flight fetch, broadcasting the result to all waiters.
    ///
    /// Must be called after `begin_fetch()` returns `None` (leader path),
    /// regardless of whether the fetch succeeded or failed.
    pub fn finish_fetch(&self, app_id: &str, file_path: &str, result: Option<CachedFile>) {
        let key = Self::in_flight_key(app_id, file_path);

        if let Some((_, sender)) = self.in_flight.remove(&key) {
            let waiting = sender.receiver_count();
            // Broadcast result — ignore errors (receivers may have dropped)
            let _ = sender.send(result);
            if waiting > 0 {
                debug!(
                    app_id = %app_id,
                    file_path = %file_path,
                    waiting = waiting,
                    "Coalesced app file fetch completed"
                );
            }
        }
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_format() {
        let key = AppFileCacheService::cache_key("app-123", "js/main.js", "sha256-abc");
        assert_eq!(key, "app-123:js/main.js:sha256-abc");
    }

    #[test]
    fn test_cache_key_with_nested_path() {
        let key =
            AppFileCacheService::cache_key("app-xyz", "assets/css/theme.css", "sha256-def456");
        assert_eq!(key, "app-xyz:assets/css/theme.css:sha256-def456");
    }

    #[test]
    fn test_in_flight_key_format() {
        let key = AppFileCacheService::in_flight_key("app-123", "index.html");
        assert_eq!(key, "apps:app-123:index.html");
    }

    #[test]
    fn test_in_flight_key_with_nested_path() {
        let key = AppFileCacheService::in_flight_key("app-xyz", "js/vendor/lib.js");
        assert_eq!(key, "apps:app-xyz:js/vendor/lib.js");
    }
}
