//! Projected document types
//!
//! Defines the structure for documents stored in the projection layer.

use bson::{doc, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::schemas::Metadata;
use crate::db::mongo::{IntoIndexes, MutMetadata};

/// A projected document stored in MongoDB
///
/// This is the generic container for all projected entries from the DHT.
/// Specific projections (content, paths, etc.) use typed collections but
/// all share this base structure.
///
/// ## Blob Integration
///
/// Content with associated blobs stores the blob hash and endpoints here.
/// This is the single source of truth for blob metadata in doorway -
/// TieredBlobCache only stores bytes, not metadata.
///
/// ```text
/// Signal Flow:
///   CacheSignal (content) → ProjectedDocument { blob_hash, ... }
///   ContentServerCommitted → updates blob_endpoints on existing doc
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectedDocument {
    /// MongoDB document ID (format: "{doc_type}:{id}")
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<String>,

    /// Document type (e.g., "Content", "LearningPath", "Relationship")
    pub doc_type: String,

    /// Document ID within its type (often the entry's id field)
    pub doc_id: String,

    /// Holochain action hash
    pub action_hash: String,

    /// Holochain entry hash (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,

    /// Author agent pubkey
    pub author: String,

    /// The projected data (denormalized, ready for queries)
    pub data: JsonValue,

    /// Search tokens for full-text search
    #[serde(default)]
    pub search_tokens: Vec<String>,

    /// Reach level for access control (e.g., "private", "family", "commons")
    /// Doorway enforces this when serving content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reach: Option<String>,

    /// Blob hash if this content has an associated blob (e.g., "sha256-abc123")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_hash: Option<String>,

    /// Endpoints where the blob can be fetched (populated by ContentServerCommitted)
    /// URLs point to elohim-storage instances that have the blob.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blob_endpoints: Vec<String>,

    /// Original entry creation timestamp
    pub created_at: DateTime,

    /// When this projection was created/updated
    pub projected_at: DateTime,

    /// Standard metadata (soft delete, timestamps)
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for ProjectedDocument {
    fn default() -> Self {
        let now = DateTime::now();
        Self {
            mongo_id: None,
            doc_type: String::new(),
            doc_id: String::new(),
            action_hash: String::new(),
            entry_hash: None,
            author: String::new(),
            data: JsonValue::Null,
            search_tokens: Vec::new(),
            reach: None,
            blob_hash: None,
            blob_endpoints: Vec::new(),
            created_at: now,
            projected_at: now,
            metadata: Metadata::default(),
        }
    }
}

impl ProjectedDocument {
    /// Create a new projected document
    pub fn new(
        doc_type: impl Into<String>,
        doc_id: impl Into<String>,
        action_hash: impl Into<String>,
        author: impl Into<String>,
        data: JsonValue,
    ) -> Self {
        let doc_type = doc_type.into();
        let doc_id = doc_id.into();
        let now = DateTime::now();

        Self {
            mongo_id: Some(format!("{}:{}", doc_type, doc_id)),
            doc_type,
            doc_id,
            action_hash: action_hash.into(),
            entry_hash: None,
            author: author.into(),
            data,
            search_tokens: Vec::new(),
            reach: None,
            blob_hash: None,
            blob_endpoints: Vec::new(),
            created_at: now,
            projected_at: now,
            metadata: Metadata::new(),
        }
    }

    /// Set the entry hash
    pub fn with_entry_hash(mut self, hash: impl Into<String>) -> Self {
        self.entry_hash = Some(hash.into());
        self
    }

    /// Add search tokens for full-text search
    pub fn with_search_tokens(mut self, tokens: Vec<String>) -> Self {
        self.search_tokens = tokens;
        self
    }

    /// Set the reach level for access control
    pub fn with_reach(mut self, reach: impl Into<String>) -> Self {
        self.reach = Some(reach.into());
        self
    }

    /// Set the blob hash (for content with associated binary data)
    pub fn with_blob_hash(mut self, hash: impl Into<String>) -> Self {
        self.blob_hash = Some(hash.into());
        self
    }

    /// Set the blob endpoints (URLs where blob can be fetched)
    pub fn with_blob_endpoints(mut self, endpoints: Vec<String>) -> Self {
        self.blob_endpoints = endpoints;
        self
    }

    /// Add blob endpoints (appends to existing, deduplicates)
    pub fn add_blob_endpoints(&mut self, endpoints: Vec<String>) {
        for endpoint in endpoints {
            if !self.blob_endpoints.contains(&endpoint) {
                self.blob_endpoints.push(endpoint);
            }
        }
    }

    /// Extract search tokens from text content
    pub fn extract_search_tokens(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() >= 3)
            .map(|word| word.to_lowercase())
            .collect()
    }
}

impl IntoIndexes for ProjectedDocument {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Index by type and projection time (for queries by type)
            (doc! { "doc_type": 1, "projected_at": -1 }, None),
            // Index by author
            (doc! { "author": 1 }, None),
            // Text index for search
            (
                doc! { "search_tokens": 1 },
                Some(IndexOptions::builder().build()),
            ),
            // Index by blob_hash for efficient endpoint lookups
            // Used by TieredBlobCache.get_or_fetch() to find blob sources
            (
                doc! { "blob_hash": 1 },
                Some(IndexOptions::builder().sparse(true).build()),
            ),
        ]
    }
}

impl MutMetadata for ProjectedDocument {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

/// Query parameters for projection queries
#[derive(Debug, Clone, Default)]
pub struct ProjectionQuery {
    /// Filter by document type
    pub doc_type: Option<String>,

    /// Filter by author
    pub author: Option<String>,

    /// Filter by specific IDs
    pub doc_ids: Option<Vec<String>>,

    /// Full-text search query
    pub search: Option<String>,

    /// Custom BSON filter
    pub filter: Option<Document>,

    /// Maximum results
    pub limit: Option<i64>,

    /// Skip for pagination
    pub skip: Option<u64>,

    /// Sort field and direction (1 = asc, -1 = desc)
    pub sort: Option<(String, i32)>,
}

impl ProjectionQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by document type
    pub fn by_type(doc_type: impl Into<String>) -> Self {
        Self {
            doc_type: Some(doc_type.into()),
            ..Default::default()
        }
    }

    /// Filter by author
    pub fn by_author(author: impl Into<String>) -> Self {
        Self {
            author: Some(author.into()),
            ..Default::default()
        }
    }

    /// Add search query
    pub fn with_search(mut self, query: impl Into<String>) -> Self {
        self.search = Some(query.into());
        self
    }

    /// Add limit
    pub fn with_limit(mut self, limit: i64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Add skip for pagination
    pub fn with_skip(mut self, skip: u64) -> Self {
        self.skip = Some(skip);
        self
    }

    /// Convert to MongoDB filter document
    pub fn to_filter(&self) -> Document {
        let mut filter = self.filter.clone().unwrap_or_else(|| doc! {});

        if let Some(ref doc_type) = self.doc_type {
            filter.insert("doc_type", doc_type);
        }

        if let Some(ref author) = self.author {
            filter.insert("author", author);
        }

        if let Some(ref ids) = self.doc_ids {
            filter.insert("doc_id", doc! { "$in": ids });
        }

        if let Some(ref search) = self.search {
            // Simple token matching (MongoDB text search would need a text index)
            let tokens: Vec<String> = search
                .split_whitespace()
                .filter(|w| w.len() >= 3)
                .map(|w| w.to_lowercase())
                .collect();
            if !tokens.is_empty() {
                filter.insert("search_tokens", doc! { "$all": tokens });
            }
        }

        // Don't return soft-deleted documents
        filter.insert("metadata.is_deleted", doc! { "$ne": true });

        filter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_projected_document_creation() {
        let doc = ProjectedDocument::new(
            "Content",
            "content-123",
            "uhCkk...",
            "uhCAk...",
            serde_json::json!({ "title": "Test" }),
        );

        assert_eq!(doc.doc_type, "Content");
        assert_eq!(doc.doc_id, "content-123");
        assert_eq!(doc.mongo_id, Some("Content:content-123".to_string()));
    }

    #[test]
    fn test_search_token_extraction() {
        let tokens = ProjectedDocument::extract_search_tokens("The quick brown fox jumps");
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(!tokens.contains(&"the".to_string())); // Too short
    }

    #[test]
    fn test_query_to_filter() {
        let query = ProjectionQuery::by_type("Content")
            .with_search("economics governance")
            .with_limit(10);

        let filter = query.to_filter();
        assert_eq!(filter.get_str("doc_type").unwrap(), "Content");
    }
}
