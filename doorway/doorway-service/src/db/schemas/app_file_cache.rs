//! App file cache document schema
//!
//! Stores cached HTML5 app files extracted from zip bundles.
//! Doorway caches these files in MongoDB to absorb web2 traffic patterns
//! (30+ concurrent asset requests per page load) so elohim-storage stays
//! focused on P2P.
//!
//! Files are keyed by `{app_id}:{file_path}:{blob_hash}` for content-addressed
//! invalidation — when a new blob_hash arrives (re-seed), old entries are
//! automatically superseded.

use bson::{doc, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};

use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Collection name for app file cache
pub const APP_FILE_CACHE_COLLECTION: &str = "app_file_cache";

/// Cached HTML5 app file stored in MongoDB
///
/// Each document represents a single extracted file from an HTML5 app's
/// zip bundle. The `_id` is a composite key of `{app_id}:{file_path}:{blob_hash}`
/// which ensures content-addressed invalidation on re-seed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppFileCacheDoc {
    /// MongoDB document ID (format: "{app_id}:{file_path}:{blob_hash}")
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<String>,

    /// The app's content ID (EPR identifier)
    pub app_id: String,

    /// Relative file path within the app bundle (e.g., "index.html", "js/app.js")
    pub file_path: String,

    /// Blob hash of the zip bundle — invalidation key
    pub blob_hash: String,

    /// EPR agreement authorizing this cache entry
    pub agreement_id: String,

    /// MIME type of the file (e.g., "text/html", "application/javascript")
    pub content_type: String,

    /// Raw file bytes
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,

    /// When this file was first cached
    pub cached_at: DateTime,

    /// When this file was last accessed (used for TTL expiry)
    pub last_accessed: DateTime,

    /// Standard metadata (created_at, updated_at, is_deleted)
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for AppFileCacheDoc {
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

impl AppFileCacheDoc {
    /// Create a new app file cache document
    pub fn new(
        app_id: String,
        file_path: String,
        blob_hash: String,
        agreement_id: String,
        content_type: String,
        data: Vec<u8>,
    ) -> Self {
        let now = DateTime::now();
        Self {
            mongo_id: Some(format!("{app_id}:{file_path}:{blob_hash}")),
            app_id,
            file_path,
            blob_hash,
            agreement_id,
            content_type,
            data,
            cached_at: now,
            last_accessed: now,
            metadata: Metadata::new(),
        }
    }
}

impl IntoIndexes for AppFileCacheDoc {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Index on app_id for bulk invalidation
            (
                doc! { "app_id": 1 },
                Some(
                    IndexOptions::builder()
                        .name("app_id_index".to_string())
                        .build(),
                ),
            ),
            // Compound index on app_id + blob_hash for invalidation on re-seed
            (
                doc! { "app_id": 1, "blob_hash": 1 },
                Some(
                    IndexOptions::builder()
                        .name("app_id_blob_hash_index".to_string())
                        .build(),
                ),
            ),
            // TTL index on last_accessed — MongoDB auto-deletes after 24h of no access
            (
                doc! { "last_accessed": 1 },
                Some(
                    IndexOptions::builder()
                        .expire_after(std::time::Duration::from_secs(86400))
                        .name("last_accessed_ttl".to_string())
                        .build(),
                ),
            ),
        ]
    }
}

impl MutMetadata for AppFileCacheDoc {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_doc_sets_mongo_id() {
        let doc = AppFileCacheDoc::new(
            "app-123".to_string(),
            "index.html".to_string(),
            "sha256-abc".to_string(),
            "self-negotiated".to_string(),
            "text/html".to_string(),
            b"<html>hello</html>".to_vec(),
        );

        assert_eq!(
            doc.mongo_id,
            Some("app-123:index.html:sha256-abc".to_string())
        );
        assert_eq!(doc.app_id, "app-123");
        assert_eq!(doc.file_path, "index.html");
        assert_eq!(doc.blob_hash, "sha256-abc");
        assert_eq!(doc.agreement_id, "self-negotiated");
        assert_eq!(doc.content_type, "text/html");
        assert_eq!(doc.data, b"<html>hello</html>");
    }

    #[test]
    fn test_default_doc() {
        let doc = AppFileCacheDoc::default();
        assert!(doc.mongo_id.is_none());
        assert!(doc.app_id.is_empty());
        assert!(doc.data.is_empty());
    }

    #[test]
    fn test_indexes_defined() {
        let indexes = AppFileCacheDoc::into_indices();
        assert_eq!(
            indexes.len(),
            3,
            "Expected 3 indexes (app_id, app_id+blob_hash, TTL)"
        );
    }
}
