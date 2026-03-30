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

use std::collections::HashMap;
use std::sync::Arc;

use bson::{doc, DateTime};
use dashmap::DashMap;
use futures_util::TryStreamExt;
use mongodb::options::ReplaceOptions;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info, warn};

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
/// Collection name for projected entries in MongoDB (the projection store).
const PROJECTED_ENTRIES_COLLECTION: &str = "projected_entries";

pub struct AppFileCacheService {
    /// MongoDB client for persistent cache storage
    mongo: MongoClient,

    /// EPR agreement ID authorizing cache entries
    agreement_id: String,

    /// In-flight fetch coalescing: prevents thundering herd when many
    /// requests arrive for the same file before the first fetch completes.
    /// Key format: "apps:{app_id}:{file_path}"
    in_flight: DashMap<String, broadcast::Sender<Option<CachedFile>>>,

    /// app_id -> blob_hash mapping (populated from content projection).
    /// Used to construct cache keys for HTML5 app file lookups.
    app_index: Arc<RwLock<HashMap<String, String>>>,
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
            app_index: Arc::new(RwLock::new(HashMap::new())),
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

    // =========================================================================
    // App Index — blob hash resolution for HTML5 apps
    // =========================================================================

    /// Load the app index from the projection store (MongoDB).
    ///
    /// Queries `projected_entries` for Content documents with
    /// `contentFormat == "html5-app"` and builds a HashMap of
    /// `app_id -> blob_hash`. Called at startup and can be called
    /// to refresh the entire index.
    pub async fn load_app_index(&self) {
        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<bson::Document>(PROJECTED_ENTRIES_COLLECTION);

        // Query for Content documents where data.contentFormat == "html5-app"
        let filter = doc! {
            "doc_type": "Content",
            "data.contentFormat": "html5-app",
            "metadata.is_deleted": { "$ne": true },
        };

        let mut cursor = match collection.find(filter).await {
            Ok(cursor) => cursor,
            Err(e) => {
                warn!(error = %e, "Failed to query projected_entries for app index");
                return;
            }
        };

        let mut index = HashMap::new();
        let mut count = 0u32;

        while let Ok(Some(doc)) = cursor.try_next().await {
            // Extract app_id and blob_hash from the data field
            if let Some(data) = doc.get("data").and_then(|v| v.as_document()) {
                let app_id = data
                    .get_str("appId")
                    .ok()
                    .or_else(|| data.get_str("id").ok());
                let blob_hash = data.get_str("blobHash").ok();

                if let (Some(app_id), Some(blob_hash)) = (app_id, blob_hash) {
                    if !app_id.is_empty() && !blob_hash.is_empty() {
                        index.insert(app_id.to_string(), blob_hash.to_string());
                        count += 1;
                    }
                }
            }
        }

        let mut locked = self.app_index.write().await;
        *locked = index;

        info!(count = count, "App index loaded from projection store");
    }

    /// Resolve the current blob_hash for an app_id.
    ///
    /// Checks the in-memory index first. On miss, performs a lazy
    /// single-document query against MongoDB and updates the index
    /// if found.
    pub async fn resolve_blob_hash(&self, app_id: &str) -> Option<String> {
        // Fast path: check index
        {
            let index = self.app_index.read().await;
            if let Some(hash) = index.get(app_id) {
                return Some(hash.clone());
            }
        }

        // Slow path: lazy load from MongoDB for this specific app
        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<bson::Document>(PROJECTED_ENTRIES_COLLECTION);

        let filter = doc! {
            "doc_type": "Content",
            "data.contentFormat": "html5-app",
            "$or": [
                { "data.appId": app_id },
                { "data.id": app_id },
            ],
            "metadata.is_deleted": { "$ne": true },
        };

        match collection.find_one(filter).await {
            Ok(Some(doc)) => {
                if let Some(data) = doc.get("data").and_then(|v| v.as_document()) {
                    if let Ok(blob_hash) = data.get_str("blobHash") {
                        if !blob_hash.is_empty() {
                            // Update index
                            let mut index = self.app_index.write().await;
                            index.insert(app_id.to_string(), blob_hash.to_string());
                            return Some(blob_hash.to_string());
                        }
                    }
                }
                None
            }
            Ok(None) => None,
            Err(e) => {
                warn!(app_id = %app_id, error = %e, "Failed to resolve blob_hash for app");
                None
            }
        }
    }

    /// Remove an app_id from the index so the next request re-resolves
    /// with a fresh blob_hash from MongoDB.
    ///
    /// Called by the invalidation hook when a content update signal
    /// arrives for an html5-app.
    pub async fn refresh_app(&self, app_id: &str) {
        let mut index = self.app_index.write().await;
        index.remove(app_id);
        debug!(app_id = %app_id, "Removed app from index (will re-resolve on next request)");
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
// Projection Invalidation Hook
// =============================================================================

/// Extract the `appId` from a projected content document's data field,
/// but only if the content format is `html5-app`.
///
/// Returns `None` for non-app content or if required fields are missing.
fn extract_html5_app_id(doc: &crate::projection::document::ProjectedDocument) -> Option<String> {
    if doc.doc_type != "Content" {
        return None;
    }

    let data = &doc.data;
    let format = data.get("contentFormat").and_then(|v| v.as_str())?;
    if format != "html5-app" {
        return None;
    }

    // Prefer appId, fall back to id
    data.get("appId")
        .and_then(|v| v.as_str())
        .or_else(|| data.get("id").and_then(|v| v.as_str()))
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Spawn a background task that watches the projection store's update channel
/// for html5-app content changes and invalidates the app file cache accordingly.
///
/// When a content update signal arrives with `contentFormat == "html5-app"`:
/// 1. `invalidate_app(app_id)` — clears all cached files for that app
/// 2. `refresh_app(app_id)` — clears the blob hash index entry so the next
///    request re-resolves with the fresh blob_hash
pub fn spawn_app_cache_invalidation_task(
    cache: Arc<AppFileCacheService>,
    mut update_rx: tokio::sync::broadcast::Receiver<crate::projection::document::ProjectedDocument>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("App file cache invalidation hook started");

        loop {
            match update_rx.recv().await {
                Ok(doc) => {
                    if let Some(app_id) = extract_html5_app_id(&doc) {
                        info!(
                            app_id = %app_id,
                            doc_id = %doc.doc_id,
                            "HTML5 app content updated — invalidating cache"
                        );
                        let deleted = cache.invalidate_app(&app_id).await;
                        cache.refresh_app(&app_id).await;
                        debug!(
                            app_id = %app_id,
                            deleted_files = deleted,
                            "App cache invalidation complete"
                        );
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    warn!(
                        lagged = n,
                        "App cache invalidation hook lagged — some updates may not have triggered invalidation"
                    );
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    info!(
                        "Projection update channel closed — app cache invalidation hook stopping"
                    );
                    break;
                }
            }
        }
    })
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

    // =========================================================================
    // extract_html5_app_id tests
    // =========================================================================

    fn make_projected_doc(
        doc_type: &str,
        doc_id: &str,
        data: serde_json::Value,
    ) -> crate::projection::document::ProjectedDocument {
        crate::projection::document::ProjectedDocument::new(
            doc_type,
            doc_id,
            "test-action",
            "test-author",
            data,
        )
    }

    #[test]
    fn test_extract_html5_app_id_with_app_id_field() {
        let doc = make_projected_doc(
            "Content",
            "content-quiz-1",
            serde_json::json!({
                "contentFormat": "html5-app",
                "appId": "quiz-1",
                "blobHash": "sha256-abc"
            }),
        );
        assert_eq!(extract_html5_app_id(&doc), Some("quiz-1".to_string()));
    }

    #[test]
    fn test_extract_html5_app_id_falls_back_to_id() {
        let doc = make_projected_doc(
            "Content",
            "simulation-phys",
            serde_json::json!({
                "contentFormat": "html5-app",
                "id": "simulation-phys",
                "blobHash": "sha256-def"
            }),
        );
        assert_eq!(
            extract_html5_app_id(&doc),
            Some("simulation-phys".to_string())
        );
    }

    #[test]
    fn test_extract_html5_app_id_ignores_non_html5_app() {
        let doc = make_projected_doc(
            "Content",
            "concept-123",
            serde_json::json!({
                "contentFormat": "markdown",
                "appId": "concept-123"
            }),
        );
        assert_eq!(extract_html5_app_id(&doc), None);
    }

    #[test]
    fn test_extract_html5_app_id_ignores_non_content_doc_type() {
        let doc = make_projected_doc(
            "Human",
            "human-abc",
            serde_json::json!({
                "contentFormat": "html5-app",
                "appId": "quiz-1"
            }),
        );
        assert_eq!(extract_html5_app_id(&doc), None);
    }

    #[test]
    fn test_extract_html5_app_id_returns_none_for_missing_format() {
        let doc = make_projected_doc(
            "Content",
            "content-no-format",
            serde_json::json!({
                "appId": "quiz-1",
                "blobHash": "sha256-abc"
            }),
        );
        assert_eq!(extract_html5_app_id(&doc), None);
    }

    #[test]
    fn test_extract_html5_app_id_returns_none_for_empty_app_id() {
        let doc = make_projected_doc(
            "Content",
            "content-empty",
            serde_json::json!({
                "contentFormat": "html5-app",
                "appId": "",
                "id": ""
            }),
        );
        assert_eq!(extract_html5_app_id(&doc), None);
    }
}
