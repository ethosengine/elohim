//! Projection Store - unified access to projected data
//!
//! Combines an in-memory hot cache with MongoDB for persistence.
//! Reads check hot cache first, then MongoDB. Writes update both.

use bson::{doc, DateTime};
use dashmap::DashMap;
use futures_util::StreamExt;
use mongodb::options::FindOptions;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

use crate::db::MongoClient;
use crate::types::DoorwayError;

use super::document::{ProjectedDocument, ProjectionQuery};

/// Hot cache entry for projected documents
#[derive(Debug, Clone)]
struct HotCacheEntry {
    doc: ProjectedDocument,
    cached_at: std::time::Instant,
}

impl HotCacheEntry {
    fn new(doc: ProjectedDocument) -> Self {
        Self {
            doc,
            cached_at: std::time::Instant::now(),
        }
    }

    fn is_expired(&self, ttl_secs: u64) -> bool {
        self.cached_at.elapsed().as_secs() > ttl_secs
    }
}

/// Projection store configuration
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Maximum entries in hot cache
    pub max_hot_cache_entries: usize,

    /// Hot cache TTL in seconds
    pub hot_cache_ttl_secs: u64,

    /// MongoDB collection name for projections
    pub collection_name: String,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            max_hot_cache_entries: 10_000,
            hot_cache_ttl_secs: 300, // 5 minutes
            collection_name: "projected_entries".to_string(),
        }
    }
}

/// Per-operation MongoDB timeout. The client URI bounds only
/// serverSelectionTimeoutMS/connectTimeoutMS — a checked-out-but-stalled socket
/// read or upsert has NO bound, so a slow-but-alive mongo can park a tokio
/// worker indefinitely. Under the warm_stream projection firehose on a
/// CPU-throttled pod this starved /health → liveness SIGKILL (doorway-freeze
/// 2026-06-13/14). Bounding each op turns a silent unbounded park into a fast,
/// LOGGED error. Default 2000ms; override DOORWAY_MONGO_OP_TIMEOUT_MS (0 → default).
fn parse_mongo_op_timeout(raw: Option<String>) -> std::time::Duration {
    let ms = raw
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(2000);
    std::time::Duration::from_millis(ms)
}

fn mongo_op_timeout() -> std::time::Duration {
    parse_mongo_op_timeout(std::env::var("DOORWAY_MONGO_OP_TIMEOUT_MS").ok())
}

/// Projection Store - unified access to projected data
///
/// Architecture:
/// - Hot cache (DashMap) for fast reads
/// - MongoDB for persistence and queries
/// - Broadcast channel for real-time updates
pub struct ProjectionStore {
    /// In-memory hot cache
    hot_cache: DashMap<String, HotCacheEntry>,

    /// MongoDB client
    mongo: Option<MongoClient>,

    /// Configuration
    config: ProjectionConfig,

    /// Broadcast sender for projection updates
    update_tx: broadcast::Sender<ProjectedDocument>,
}

impl ProjectionStore {
    /// Create a new projection store with MongoDB
    pub async fn new(mongo: MongoClient, config: ProjectionConfig) -> Result<Self, DoorwayError> {
        // Create indexes on the collection
        Self::ensure_indexes(&mongo, &config.collection_name).await?;

        let (update_tx, _) = broadcast::channel(1000);

        info!(
            "ProjectionStore initialized with collection '{}', hot cache max {} entries",
            config.collection_name, config.max_hot_cache_entries
        );

        Ok(Self {
            hot_cache: DashMap::new(),
            mongo: Some(mongo),
            config,
            update_tx,
        })
    }

    /// Create a projection store without MongoDB (hot cache only)
    pub fn memory_only(config: ProjectionConfig) -> Self {
        let (update_tx, _) = broadcast::channel(1000);

        warn!("ProjectionStore running in memory-only mode (no MongoDB)");

        Self {
            hot_cache: DashMap::new(),
            mongo: None,
            config,
            update_tx,
        }
    }

    /// Ensure MongoDB indexes exist
    async fn ensure_indexes(
        mongo: &MongoClient,
        collection_name: &str,
    ) -> Result<(), DoorwayError> {
        let db = mongo.inner().database(mongo.db_name());
        let collection = db.collection::<ProjectedDocument>(collection_name);

        // Create indexes
        let indexes = vec![
            doc! { "doc_type": 1, "projected_at": -1 },
            doc! { "author": 1 },
            doc! { "doc_id": 1 },
            doc! { "search_tokens": 1 },
        ];

        for index_doc in indexes {
            let index = mongodb::IndexModel::builder().keys(index_doc).build();
            if let Err(e) = collection.create_index(index).await {
                warn!("Failed to create index: {}", e);
            }
        }

        Ok(())
    }

    /// Get a projected document by ID
    ///
    /// Checks hot cache first, then MongoDB.
    pub async fn get(&self, doc_type: &str, doc_id: &str) -> Option<ProjectedDocument> {
        let cache_key = format!("{doc_type}:{doc_id}");

        // Check hot cache first
        if let Some(entry) = self.hot_cache.get(&cache_key) {
            if !entry.is_expired(self.config.hot_cache_ttl_secs) {
                debug!("Hot cache hit for {}", cache_key);
                return Some(entry.doc.clone());
            } else {
                // Expired, remove from cache
                drop(entry);
                self.hot_cache.remove(&cache_key);
            }
        }

        // Fall back to MongoDB
        if let Some(ref mongo) = self.mongo {
            match self.get_from_mongo(mongo, &cache_key).await {
                Ok(Some(doc)) => {
                    // Populate hot cache
                    self.hot_cache
                        .insert(cache_key, HotCacheEntry::new(doc.clone()));
                    self.evict_if_needed();
                    return Some(doc);
                }
                Ok(None) => return None,
                Err(e) => {
                    error!("MongoDB read failed for {}: {}", cache_key, e);
                    return None;
                }
            }
        }

        None
    }

    /// Get document from MongoDB
    async fn get_from_mongo(
        &self,
        mongo: &MongoClient,
        mongo_id: &str,
    ) -> Result<Option<ProjectedDocument>, DoorwayError> {
        let db = mongo.inner().database(mongo.db_name());
        let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

        match tokio::time::timeout(
            mongo_op_timeout(),
            collection.find_one(doc! { "_id": mongo_id, "metadata.is_deleted": { "$ne": true } }),
        )
        .await
        {
            Ok(r) => r.map_err(|e| DoorwayError::Database(format!("Find failed: {e}"))),
            Err(_) => Err(DoorwayError::Database(format!(
                "Find timed out after {}ms (mongo slow/parked): {mongo_id}",
                mongo_op_timeout().as_millis()
            ))),
        }
    }

    /// Store a projected document
    ///
    /// Writes to both hot cache and MongoDB.
    pub async fn set(&self, doc: ProjectedDocument) -> Result<(), DoorwayError> {
        let cache_key = doc
            .mongo_id
            .clone()
            .unwrap_or_else(|| format!("{}:{}", doc.doc_type, doc.doc_id));

        // Idempotency guard — skip redundant re-projections.
        //
        // warm-stream re-streams the entire corpus on every (re)connect and every
        // DHT signal re-projects, so under load the same documents are re-`set`
        // many times per minute (measured ~14k upserts/2min over a ~3.6k-doc
        // corpus on alpha — a re-projection firehose that saturated the gateway).
        // When the hot cache already holds a projection identical in every served
        // field, the Mongo upsert and the broadcast are pure overhead: skip both
        // and just refresh the entry's recency so it stays hot. The equivalence
        // check runs under a short-lived read guard that is dropped before the
        // recency bump, so no shard lock is ever held across work.
        let already_current = self
            .hot_cache
            .get(&cache_key)
            .map(|existing| existing.doc.is_equivalent_projection(&doc))
            .unwrap_or(false);
        if already_current {
            if let Some(mut existing) = self.hot_cache.get_mut(&cache_key) {
                existing.cached_at = std::time::Instant::now();
            }
            return Ok(());
        }

        // Write to MongoDB first (if available)
        if let Some(ref mongo) = self.mongo {
            self.upsert_to_mongo(mongo, &doc).await?;
        }

        // Update hot cache
        self.hot_cache
            .insert(cache_key, HotCacheEntry::new(doc.clone()));
        self.evict_if_needed();

        // Broadcast update
        let _ = self.update_tx.send(doc);

        Ok(())
    }

    /// Upsert document to MongoDB
    async fn upsert_to_mongo(
        &self,
        mongo: &MongoClient,
        doc: &ProjectedDocument,
    ) -> Result<(), DoorwayError> {
        let db = mongo.inner().database(mongo.db_name());
        let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

        let mongo_id = doc
            .mongo_id
            .clone()
            .unwrap_or_else(|| format!("{}:{}", doc.doc_type, doc.doc_id));

        // Use replace with upsert
        let options = mongodb::options::ReplaceOptions::builder()
            .upsert(true)
            .build();

        match tokio::time::timeout(
            mongo_op_timeout(),
            collection
                .replace_one(doc! { "_id": &mongo_id }, doc)
                .with_options(options),
        )
        .await
        {
            Ok(r) => {
                r.map_err(|e| DoorwayError::Database(format!("Upsert failed: {e}")))?;
            }
            Err(_) => {
                return Err(DoorwayError::Database(format!(
                    "Upsert timed out after {}ms (mongo slow/parked): {mongo_id}",
                    mongo_op_timeout().as_millis()
                )));
            }
        }

        debug!("Projected document upserted: {}", mongo_id);
        Ok(())
    }

    /// Query projected documents
    ///
    /// Always queries MongoDB (hot cache is for single-document lookups).
    pub async fn query(
        &self,
        query: ProjectionQuery,
    ) -> Result<Vec<ProjectedDocument>, DoorwayError> {
        let Some(ref mongo) = self.mongo else {
            // Memory-only mode: scan hot cache (inefficient but functional)
            return Ok(self.query_hot_cache(&query));
        };

        let db = mongo.inner().database(mongo.db_name());
        let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

        let filter = query.to_filter();

        // Build sort document
        let sort_doc = if let Some((field, direction)) = query.sort {
            doc! { field: direction }
        } else {
            // Default: newest first
            doc! { "projected_at": -1 }
        };

        // Build find options
        let options = FindOptions::builder()
            .limit(query.limit)
            .skip(query.skip)
            .sort(sort_doc)
            .build();

        let cursor = collection
            .find(filter)
            .with_options(options)
            .await
            .map_err(|e| DoorwayError::Database(format!("Query failed: {e}")))?;

        let results: Vec<ProjectedDocument> = cursor
            .filter_map(|doc| async {
                match doc {
                    Ok(d) => Some(d),
                    Err(e) => {
                        error!("Error reading document: {}", e);
                        None
                    }
                }
            })
            .collect()
            .await;

        Ok(results)
    }

    /// Pre-warm the hot cache from MongoDB at startup — the cold-start firehose cure.
    ///
    /// A fresh pod boots with an EMPTY hot cache, so warm_stream's idempotency
    /// guard (which keys off the hot cache, see `set`) MISSES on every streamed
    /// entry and re-`upsert`s the entire corpus — a CPU burst that, under a CFS
    /// quota, starves the /health task off the runtime → liveness SIGKILL (137)
    /// → crashloop (doorway-freeze 2026-06-14; an already-warm pod was stable
    /// only because its cache was full). The shared Mongo already holds the
    /// corpus, so loading it into the hot cache FIRST makes the cold-start replay
    /// hit the idempotency guard and skip the firehose — a fresh pod then behaves
    /// like an already-warm one. Bounded by `max_hot_cache_entries`; degrade-open
    /// (any failure just leaves the cache empty = prior behavior, never fatal).
    pub async fn prewarm_hot_cache_from_mongo(&self) -> Result<usize, DoorwayError> {
        let Some(ref mongo) = self.mongo else {
            return Ok(0); // memory-only mode: nothing to pre-warm from
        };
        let db = mongo.inner().database(mongo.db_name());
        let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

        let options = FindOptions::builder()
            .limit(self.config.max_hot_cache_entries as i64)
            .sort(doc! { "projected_at": -1 })
            .build();

        let cursor = match tokio::time::timeout(
            std::time::Duration::from_secs(30),
            collection
                .find(doc! { "metadata.is_deleted": { "$ne": true } })
                .with_options(options),
        )
        .await
        {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => return Err(DoorwayError::Database(format!("Pre-warm find failed: {e}"))),
            Err(_) => {
                return Err(DoorwayError::Database(
                    "Pre-warm find timed out after 30s".to_string(),
                ))
            }
        };

        let docs: Vec<ProjectedDocument> = cursor
            .filter_map(|doc| async {
                match doc {
                    Ok(d) => Some(d),
                    Err(e) => {
                        warn!("Pre-warm: skipping undeserializable doc: {e}");
                        None
                    }
                }
            })
            .collect()
            .await;

        let count = docs.len();
        for doc in docs {
            let key = doc
                .mongo_id
                .clone()
                .unwrap_or_else(|| format!("{}:{}", doc.doc_type, doc.doc_id));
            self.hot_cache.insert(key, HotCacheEntry::new(doc));
        }
        info!(
            prewarmed = count,
            "Hot cache pre-warmed from MongoDB (cold-start firehose guard)"
        );
        Ok(count)
    }

    /// Query hot cache (fallback for memory-only mode)
    fn query_hot_cache(&self, query: &ProjectionQuery) -> Vec<ProjectedDocument> {
        let mut results: Vec<ProjectedDocument> = self
            .hot_cache
            .iter()
            .filter(|entry| {
                let doc = &entry.doc;

                // Filter by type
                if let Some(ref doc_type) = query.doc_type {
                    if doc.doc_type != *doc_type {
                        return false;
                    }
                }

                // Filter by author
                if let Some(ref author) = query.author {
                    if doc.author != *author {
                        return false;
                    }
                }

                // Filter by IDs
                if let Some(ref ids) = query.doc_ids {
                    if !ids.contains(&doc.doc_id) {
                        return false;
                    }
                }

                true
            })
            .map(|entry| entry.doc.clone())
            .collect();

        // Sort by projected_at descending
        results.sort_by_key(|b| std::cmp::Reverse(b.projected_at));

        // Apply limit
        if let Some(limit) = query.limit {
            results.truncate(limit as usize);
        }

        results
    }

    /// Invalidate projections by pattern
    ///
    /// Pattern format: "{doc_type}:{doc_id}" or "{doc_type}:*" for all of a type
    pub async fn invalidate(&self, pattern: &str) -> Result<usize, DoorwayError> {
        let mut count = 0;

        // Invalidate hot cache
        if pattern.ends_with(":*") {
            let doc_type = pattern.trim_end_matches(":*");
            let keys_to_remove: Vec<String> = self
                .hot_cache
                .iter()
                .filter(|entry| entry.doc.doc_type == doc_type)
                .map(|entry| entry.key().clone())
                .collect();

            for key in keys_to_remove {
                self.hot_cache.remove(&key);
                count += 1;
            }
        } else if self.hot_cache.remove(pattern).is_some() {
            count += 1;
        }

        // Soft-delete in MongoDB
        if let Some(ref mongo) = self.mongo {
            let db = mongo.inner().database(mongo.db_name());
            let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

            let filter = if pattern.ends_with(":*") {
                let doc_type = pattern.trim_end_matches(":*");
                doc! { "doc_type": doc_type }
            } else {
                doc! { "_id": pattern }
            };

            let update = doc! {
                "$set": {
                    "metadata.is_deleted": true,
                    "metadata.deleted_at": DateTime::now(),
                }
            };

            match collection.update_many(filter, update).await {
                Ok(result) => {
                    count += result.modified_count as usize;
                }
                Err(e) => {
                    error!("MongoDB invalidation failed: {}", e);
                }
            }
        }

        debug!(
            "Invalidated {} projections matching pattern '{}'",
            count, pattern
        );
        Ok(count)
    }

    /// Subscribe to projection updates
    pub fn subscribe(&self) -> broadcast::Receiver<ProjectedDocument> {
        self.update_tx.subscribe()
    }

    /// Update blob endpoints for documents with matching blob_hash
    ///
    /// This is called when ContentServerCommitted signals arrive from the
    /// infrastructure DNA, indicating a P2P agent is now serving a blob.
    /// We find all projected documents with that blob_hash and add the
    /// new endpoints to their `blob_endpoints` list.
    ///
    /// Returns the number of documents updated.
    pub async fn update_blob_endpoints(
        &self,
        blob_hash: &str,
        endpoints: Vec<String>,
    ) -> Result<usize, DoorwayError> {
        if endpoints.is_empty() {
            return Ok(0);
        }

        let mut count = 0;

        // Update in MongoDB
        if let Some(ref mongo) = self.mongo {
            let db = mongo.inner().database(mongo.db_name());
            let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

            // Find documents with matching blob_hash and add endpoints
            // Using $addToSet to avoid duplicates
            let filter = doc! { "blob_hash": blob_hash };
            let update = doc! {
                "$addToSet": {
                    "blob_endpoints": { "$each": &endpoints }
                }
            };

            match collection.update_many(filter, update).await {
                Ok(result) => {
                    count = result.modified_count as usize;
                    if count > 0 {
                        debug!(
                            "Updated {} documents with blob_hash {} with {} new endpoints",
                            count,
                            blob_hash,
                            endpoints.len()
                        );
                    }
                }
                Err(e) => {
                    warn!("Failed to update blob endpoints in MongoDB: {}", e);
                }
            }
        }

        // Update hot cache entries with matching blob_hash
        for mut entry in self.hot_cache.iter_mut() {
            if entry.doc.blob_hash.as_deref() == Some(blob_hash) {
                entry.doc.add_blob_endpoints(endpoints.clone());
                count += 1;
            }
        }

        Ok(count)
    }

    /// Get blob endpoints for a given blob_hash
    ///
    /// Looks up any projected document with this blob_hash and returns
    /// its blob_endpoints. Used by TieredBlobCache.get_or_fetch().
    pub async fn get_blob_endpoints(&self, blob_hash: &str) -> Option<Vec<String>> {
        // Check hot cache first
        for entry in self.hot_cache.iter() {
            if entry.doc.blob_hash.as_deref() == Some(blob_hash)
                && !entry.doc.blob_endpoints.is_empty()
            {
                return Some(entry.doc.blob_endpoints.clone());
            }
        }

        // Fall back to MongoDB
        if let Some(ref mongo) = self.mongo {
            let db = mongo.inner().database(mongo.db_name());
            let collection = db.collection::<ProjectedDocument>(&self.config.collection_name);

            let filter = doc! {
                "blob_hash": blob_hash,
                "blob_endpoints": { "$exists": true, "$ne": [] }
            };

            if let Ok(Some(doc)) = collection.find_one(filter).await {
                if !doc.blob_endpoints.is_empty() {
                    return Some(doc.blob_endpoints);
                }
            }
        }

        None
    }

    /// Get hot cache statistics
    pub fn hot_cache_stats(&self) -> HotCacheStats {
        let total = self.hot_cache.len();
        let expired = self
            .hot_cache
            .iter()
            .filter(|entry| entry.is_expired(self.config.hot_cache_ttl_secs))
            .count();

        HotCacheStats {
            total_entries: total,
            expired_entries: expired,
            max_entries: self.config.max_hot_cache_entries,
        }
    }

    /// Evict oldest entries if hot cache is over capacity.
    ///
    /// Batched, single-pass eviction. The previous implementation did
    /// `while len > max { iter().min_by_key() }` — a full-map rescan per evicted
    /// entry (O(n²)) that held DashMap shard locks across each scan. With a
    /// stable cache key the live corpus stays well under the cap so this path is
    /// latent, but under any workload that did cross the cap it was a synchronous
    /// lock-holding hot-path scan — a wedge waiting to happen. Collect-sort-drop
    /// in one pass is O(n log n) once per over-cap write instead.
    fn evict_if_needed(&self) {
        let max = self.config.max_hot_cache_entries;
        if self.hot_cache.len() <= max {
            return;
        }

        // Find and remove expired entries first (single pass).
        let expired_keys: Vec<String> = self
            .hot_cache
            .iter()
            .filter(|entry| entry.is_expired(self.config.hot_cache_ttl_secs))
            .map(|entry| entry.key().clone())
            .collect();

        for key in expired_keys {
            self.hot_cache.remove(&key);
        }

        // If still over capacity, evict the oldest entries in ONE pass: snapshot
        // (key, cached_at), sort by age, drop the overflow. No per-entry rescan.
        let overflow = self.hot_cache.len().saturating_sub(max);
        if overflow == 0 {
            return;
        }
        let mut by_age: Vec<(String, std::time::Instant)> = self
            .hot_cache
            .iter()
            .map(|entry| (entry.key().clone(), entry.cached_at))
            .collect();
        by_age.sort_by_key(|(_, cached_at)| *cached_at);
        for (key, _) in by_age.into_iter().take(overflow) {
            self.hot_cache.remove(&key);
        }
    }

    /// Check if MongoDB is available
    pub fn has_mongodb(&self) -> bool {
        self.mongo.is_some()
    }

    /// Count projected documents by type.
    ///
    /// Returns (content, humans, relationships) from the hot cache.
    /// Used by the /health/startup endpoint to report projection progress.
    pub fn count_by_type(&self) -> (usize, usize, usize) {
        let mut content = 0usize;
        let mut humans = 0usize;
        let mut relationships = 0usize;

        for entry in self.hot_cache.iter() {
            match entry.doc.doc_type.as_str() {
                "Content" => content += 1,
                "Human" => humans += 1,
                "Relationship" => relationships += 1,
                _ => {}
            }
        }

        (content, humans, relationships)
    }
}

/// Hot cache statistics
#[derive(Debug, Clone)]
pub struct HotCacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub max_entries: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_only_store() {
        let store = ProjectionStore::memory_only(ProjectionConfig::default());
        assert!(!store.has_mongodb());
    }

    #[tokio::test]
    async fn prewarm_memory_only_is_noop() {
        // No Mongo → pre-warm degrades open to a no-op (0 entries), never errors.
        // (The Mongo-backed cold-start-firehose-skip behavior is verified by the deploy.)
        let store = ProjectionStore::memory_only(ProjectionConfig::default());
        assert_eq!(store.prewarm_hot_cache_from_mongo().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_hot_cache_set_get() {
        let store = ProjectionStore::memory_only(ProjectionConfig::default());

        let doc = ProjectedDocument::new(
            "Content",
            "test-123",
            "uhCkk...",
            "uhCAk...",
            serde_json::json!({ "title": "Test" }),
        );

        store.set(doc.clone()).await.unwrap();

        let retrieved = store.get("Content", "test-123").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().doc_id, "test-123");
    }

    #[tokio::test]
    async fn redundant_reprojection_is_skipped_real_change_reprojects() {
        // Regression for the alpha re-projection firehose: warm-stream re-streams
        // the whole corpus repeatedly, each entry rebuilt fresh (new created_at/
        // projected_at) but identical in served content. `set` must recognise the
        // no-op and skip the broadcast (and, in prod, the Mongo upsert); a genuine
        // content change must still re-project + broadcast.
        let store = ProjectionStore::memory_only(ProjectionConfig::default());
        let mut rx = store.subscribe();

        let initial = ProjectedDocument::new(
            "Content",
            "c-1",
            "cache-stream",
            "cache-stream",
            serde_json::json!({ "title": "Stable" }),
        );
        store.set(initial).await.unwrap(); // broadcast #1 (first projection)

        // Re-stream: a FRESH doc (different created_at/projected_at) with identical
        // served content. Must be skipped — proves the timestamp fields are
        // excluded from equivalence.
        let restream = ProjectedDocument::new(
            "Content",
            "c-1",
            "cache-stream",
            "cache-stream",
            serde_json::json!({ "title": "Stable" }),
        );
        store.set(restream).await.unwrap(); // no broadcast (equivalent)

        // A real content change must re-project + broadcast.
        let changed = ProjectedDocument::new(
            "Content",
            "c-1",
            "cache-stream",
            "cache-stream",
            serde_json::json!({ "title": "Changed" }),
        );
        store.set(changed).await.unwrap(); // broadcast #2

        let mut broadcasts = 0;
        while rx.try_recv().is_ok() {
            broadcasts += 1;
        }
        assert_eq!(
            broadcasts, 2,
            "redundant re-projection must be skipped; only the initial and the changed projection broadcast"
        );

        let got = store.get("Content", "c-1").await.unwrap();
        assert_eq!(got.data, serde_json::json!({ "title": "Changed" }));
    }

    #[tokio::test]
    async fn eviction_keeps_cache_within_cap_over_flood() {
        // The latent eviction path: push well past the cap with distinct keys and
        // assert the batched eviction trims back to (at most) the cap. Exercises
        // the over-capacity branch that the live corpus never reaches.
        let store = ProjectionStore::memory_only(ProjectionConfig {
            max_hot_cache_entries: 100,
            hot_cache_ttl_secs: 300,
            collection_name: "test".to_string(),
        });
        for i in 0..250 {
            let doc = ProjectedDocument::new(
                "Content",
                format!("c-{i}"),
                format!("h-{i}"),
                "author",
                serde_json::json!({ "n": i }),
            );
            store.set(doc).await.unwrap();
        }
        assert!(
            store.hot_cache_stats().total_entries <= 100,
            "eviction must keep the hot cache at or under the configured cap"
        );
    }

    #[test]
    fn mongo_op_timeout_parses_env_and_defaults() {
        // default when unset / invalid / zero; explicit value otherwise.
        assert_eq!(super::parse_mongo_op_timeout(None).as_millis(), 2000);
        assert_eq!(
            super::parse_mongo_op_timeout(Some("500".to_string())).as_millis(),
            500
        );
        assert_eq!(
            super::parse_mongo_op_timeout(Some("0".to_string())).as_millis(),
            2000,
            "zero must fall back to the default, never a 0ms (instant-timeout) op"
        );
        assert_eq!(
            super::parse_mongo_op_timeout(Some("nope".to_string())).as_millis(),
            2000
        );
    }
}
