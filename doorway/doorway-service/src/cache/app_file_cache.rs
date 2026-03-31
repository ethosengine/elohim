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
//! Files are keyed by `{slug}:{file_path}:{blob_hash}`. When a new
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
    /// Key format: "apps:{slug}:{file_path}"
    in_flight: DashMap<String, broadcast::Sender<Option<CachedFile>>>,

    /// slug -> blob_hash mapping (populated from content projection).
    /// Used to construct cache keys for HTML5 app file lookups.
    slug_index: Arc<RwLock<HashMap<String, String>>>,
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
            slug_index: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build the MongoDB `_id` for a cached file.
    ///
    /// Format: `{slug}:{file_path}:{blob_hash}`
    pub fn cache_key(slug: &str, file_path: &str, blob_hash: &str) -> String {
        format!("{slug}:{file_path}:{blob_hash}")
    }

    /// Build the in-flight coalescing key for a file being fetched.
    ///
    /// Format: `apps:{slug}:{file_path}`
    ///
    /// Note: this is blob_hash-independent because we only ever fetch the
    /// latest version of a file — the blob_hash is determined by the fetch.
    pub fn in_flight_key(slug: &str, file_path: &str) -> String {
        format!("apps:{slug}:{file_path}")
    }

    /// Look up a cached file by slug, file_path, and blob_hash.
    ///
    /// On cache hit, updates `last_accessed` in a fire-and-forget spawn
    /// to keep the TTL index fresh without blocking the caller.
    pub async fn get(&self, slug: &str, file_path: &str, blob_hash: &str) -> Option<CachedFile> {
        let mongo_id = Self::cache_key(slug, file_path, blob_hash);

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
        slug: &str,
        file_path: &str,
        blob_hash: &str,
        content_type: &str,
        data: Vec<u8>,
    ) {
        let doc = AppFileCacheDoc::new(
            slug.to_string(),
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
    pub async fn invalidate_app(&self, slug: &str) -> u64 {
        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<AppFileCacheDoc>(APP_FILE_CACHE_COLLECTION);

        match collection.delete_many(doc! { "slug": slug }).await {
            Ok(result) => {
                let count = result.deleted_count;
                if count > 0 {
                    debug!(slug = %slug, count = count, "Invalidated app file cache");
                }
                count
            }
            Err(e) => {
                error!(slug = %slug, error = %e, "Failed to invalidate app file cache");
                0
            }
        }
    }

    // =========================================================================
    // App Index — blob hash resolution for HTML5 apps
    // =========================================================================

    /// Load the slug index from the projection store (MongoDB).
    ///
    /// Queries `projected_entries` for Content documents with
    /// `contentFormat == "html5-app"` and builds a HashMap of
    /// `slug -> blob_hash`. Called at startup and can be called
    /// to refresh the entire index.
    pub async fn load_slug_index(&self) {
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
                warn!(error = %e, "Failed to query projected_entries for slug index");
                return;
            }
        };

        let mut index = HashMap::new();
        let mut count = 0u32;

        while let Ok(Some(doc)) = cursor.try_next().await {
            if let Some(data) = doc.get("data").and_then(|v| v.as_document()) {
                let blob_hash = data.get_str("blobHash").ok();

                // The HTML5 app's slug lives inside contentBody (a JSON string),
                // NOT in data.hAppId (which is the Holochain app context, e.g. "lamad").
                let html5_slug = extract_html5_slug_from_data(data);

                if let (Some(slug), Some(blob_hash)) = (html5_slug, blob_hash) {
                    if !slug.is_empty() && !blob_hash.is_empty() {
                        index.insert(slug.to_string(), blob_hash.to_string());
                        count += 1;
                    }
                }
            }
        }

        let mut locked = self.slug_index.write().await;
        *locked = index;

        info!(count = count, "Slug index loaded from projection store");
    }

    /// Resolve the current blob_hash for an identifier (slug or content address).
    ///
    /// Short-circuits for content addresses (`sha256-...`) — returns the
    /// identifier directly since it IS the blob hash. For slugs, checks
    /// the in-memory index first, then falls back to a lazy single-document
    /// query against MongoDB.
    pub async fn resolve_blob_hash(&self, identifier: &str) -> Option<String> {
        // Short-circuit: if identifier is already a content address, return it directly
        if identifier.starts_with("sha256-") && identifier.len() > 10 {
            return Some(identifier.to_string());
        }

        let slug = identifier;

        // Fast path: check index
        {
            let index = self.slug_index.read().await;
            if let Some(hash) = index.get(slug) {
                return Some(hash.clone());
            }
        }

        // Slow path: lazy load from MongoDB for this specific app
        let db = self.mongo.inner().database(self.mongo.db_name());
        let collection = db.collection::<bson::Document>(PROJECTED_ENTRIES_COLLECTION);

        // The HTML5 app's slug is inside contentBody (JSON string), not at
        // data.hAppId (which is the Holochain app context). We can't query
        // inside a JSON string with MongoDB, so scan all html5-app entries.
        let filter = doc! {
            "doc_type": "Content",
            "data.contentFormat": "html5-app",
            "metadata.is_deleted": { "$ne": true },
        };

        let mut cursor = match collection.find(filter).await {
            Ok(c) => c,
            Err(e) => {
                warn!(slug = %slug, error = %e, "Failed to query for app blob hash");
                return None;
            }
        };

        while let Ok(Some(doc)) = cursor.try_next().await {
            if let Some(data) = doc.get("data").and_then(|v| v.as_document()) {
                let html5_slug = extract_html5_slug_from_data(data);
                if html5_slug.as_deref() == Some(slug) {
                    if let Ok(blob_hash) = data.get_str("blobHash") {
                        if !blob_hash.is_empty() {
                            let mut index = self.slug_index.write().await;
                            index.insert(slug.to_string(), blob_hash.to_string());
                            return Some(blob_hash.to_string());
                        }
                    }
                }
            }
        }

        None
    }

    /// Remove a slug from the index so the next request re-resolves
    /// with a fresh blob_hash from MongoDB.
    ///
    /// Called by the invalidation hook when a content update signal
    /// arrives for an html5-app.
    pub async fn refresh_app(&self, slug: &str) {
        let mut index = self.slug_index.write().await;
        index.remove(slug);
        debug!(slug = %slug, "Removed app from index (will re-resolve on next request)");
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
        slug: &str,
        file_path: &str,
    ) -> Option<broadcast::Receiver<Option<CachedFile>>> {
        let key = Self::in_flight_key(slug, file_path);

        // Atomic check-and-insert via entry() API to prevent TOCTOU race.
        // Without this, two concurrent tasks could both see an empty slot,
        // both insert, and the first task's broadcast sender gets overwritten
        // — orphaning its waiters.
        match self.in_flight.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(entry) => Some(entry.get().subscribe()),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                let (tx, _) = broadcast::channel(1);
                entry.insert(tx);
                None
            }
        }
    }

    /// Complete an in-flight fetch, broadcasting the result to all waiters.
    ///
    /// Must be called after `begin_fetch()` returns `None` (leader path),
    /// regardless of whether the fetch succeeded or failed.
    pub fn finish_fetch(&self, slug: &str, file_path: &str, result: Option<CachedFile>) {
        let key = Self::in_flight_key(slug, file_path);

        if let Some((_, sender)) = self.in_flight.remove(&key) {
            let waiting = sender.receiver_count();
            // Broadcast result — ignore errors (receivers may have dropped)
            let _ = sender.send(result);
            if waiting > 0 {
                debug!(
                    slug = %slug,
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

/// Extract the `slug` from a projected content document's data field,
/// but only if the content format is `html5-app`.
///
/// Returns `None` for non-app content or if required fields are missing.
fn extract_html5_slug(doc: &crate::projection::document::ProjectedDocument) -> Option<String> {
    if doc.doc_type != "Content" {
        return None;
    }

    let data = &doc.data;
    let format = data.get("contentFormat").and_then(|v| v.as_str())?;
    if format != "html5-app" {
        return None;
    }

    // The HTML5 app's slug is inside contentBody (a JSON string containing
    // {slug, entryPoint, fallbackUrl}). data.hAppId is the Holochain app
    // context (e.g., "lamad"), NOT the HTML5 app identifier.
    if let Some(content_body) = data.get("contentBody").and_then(|v| v.as_str()) {
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(content_body) {
            if let Some(slug) = body.get("slug").and_then(|v| v.as_str()) {
                if !slug.is_empty() {
                    return Some(slug.to_string());
                }
            }
        }
    }

    // Fallback: try contentBody as object (not string) — depends on projection format
    if let Some(body) = data.get("contentBody") {
        if let Some(slug) = body.get("slug").and_then(|v| v.as_str()) {
            if !slug.is_empty() {
                return Some(slug.to_string());
            }
        }
    }

    // Last resort: use content id (e.g., "simulation-evolution-of-trust")
    data.get("id")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

/// Extract HTML5 app slug from a raw BSON data document.
/// Same logic as `extract_html5_slug` but for raw MongoDB documents
/// used in the slug index load and resolve paths.
fn extract_html5_slug_from_data(data: &bson::Document) -> Option<String> {
    // Parse contentBody (JSON string or BSON object) for the HTML5 slug
    if let Ok(content_body) = data.get_str("contentBody") {
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(content_body) {
            if let Some(slug) = body.get("slug").and_then(|v| v.as_str()) {
                if !slug.is_empty() {
                    return Some(slug.to_string());
                }
            }
        }
    }

    // Try contentBody as BSON document (not string)
    if let Ok(body) = data.get_document("contentBody") {
        if let Ok(slug) = body.get_str("slug") {
            if !slug.is_empty() {
                return Some(slug.to_string());
            }
        }
    }

    // Last resort: content id
    data.get_str("id").ok().map(|s| s.to_string())
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
                    if let Some(slug) = extract_html5_slug(&doc) {
                        info!(
                            slug = %slug,
                            doc_id = %doc.doc_id,
                            "HTML5 app content updated — invalidating cache"
                        );
                        let deleted = cache.invalidate_app(&slug).await;
                        cache.refresh_app(&slug).await;
                        debug!(
                            slug = %slug,
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
    // extract_html5_slug tests
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
    fn test_extract_html5_slug_with_slug_field() {
        // slug for HTML5 apps lives inside contentBody (JSON string),
        // NOT at the top level of data (which is the Holochain app context)
        let doc = make_projected_doc(
            "Content",
            "content-quiz-1",
            serde_json::json!({
                "contentFormat": "html5-app",
                "hAppId": "lamad",  // Holochain app context — NOT the HTML5 app slug
                "contentBody": "{\"slug\":\"quiz-1\",\"entryPoint\":\"index.html\"}",
                "blobHash": "sha256-abc"
            }),
        );
        assert_eq!(extract_html5_slug(&doc), Some("quiz-1".to_string()));
    }

    #[test]
    fn test_extract_html5_slug_falls_back_to_id() {
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
            extract_html5_slug(&doc),
            Some("simulation-phys".to_string())
        );
    }

    #[test]
    fn test_extract_html5_slug_ignores_non_html5_app() {
        let doc = make_projected_doc(
            "Content",
            "concept-123",
            serde_json::json!({
                "contentFormat": "markdown",
                "slug": "concept-123"
            }),
        );
        assert_eq!(extract_html5_slug(&doc), None);
    }

    #[test]
    fn test_extract_html5_slug_ignores_non_content_doc_type() {
        let doc = make_projected_doc(
            "Human",
            "human-abc",
            serde_json::json!({
                "contentFormat": "html5-app",
                "slug": "quiz-1"
            }),
        );
        assert_eq!(extract_html5_slug(&doc), None);
    }

    #[test]
    fn test_extract_html5_slug_returns_none_for_missing_format() {
        let doc = make_projected_doc(
            "Content",
            "content-no-format",
            serde_json::json!({
                "slug": "quiz-1",
                "blobHash": "sha256-abc"
            }),
        );
        assert_eq!(extract_html5_slug(&doc), None);
    }

    #[test]
    fn test_extract_html5_slug_returns_none_for_empty_slug() {
        let doc = make_projected_doc(
            "Content",
            "content-empty",
            serde_json::json!({
                "contentFormat": "html5-app",
                "slug": "",
                "id": ""
            }),
        );
        assert_eq!(extract_html5_slug(&doc), None);
    }
}
