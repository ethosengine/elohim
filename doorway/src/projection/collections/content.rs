//! Content projection schema
//!
//! Optimized projection for Content entries with rich indexing for
//! content type, tags, reach, and full-text search.

use bson::{doc, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::mongo::{IntoIndexes, MutMetadata};
use crate::db::schemas::Metadata;

/// Projected content document
///
/// Denormalized view of Content entries optimized for queries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentProjection {
    /// MongoDB document ID (format: "content-{id}")
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<String>,

    /// Content ID from the entry
    pub id: String,

    /// Holochain action hash
    pub action_hash: String,

    /// Holochain entry hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,

    /// Content type (epic, concept, lesson, scenario, etc.)
    pub content_type: String,

    /// Title
    pub title: String,

    /// Description (truncated for index)
    pub description: String,

    /// Summary (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,

    /// Content format (markdown, html, video, etc.)
    pub content_format: String,

    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,

    /// Visibility reach (private, community, commons, etc.)
    pub reach: String,

    /// Trust score
    #[serde(default)]
    pub trust_score: f64,

    /// Estimated reading time in minutes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<u32>,

    /// Author agent pubkey
    pub author: String,

    /// Search tokens for full-text search
    #[serde(default)]
    pub search_tokens: Vec<String>,

    /// Original entry creation timestamp
    pub created_at: DateTime,

    /// Last update timestamp from entry
    pub updated_at: DateTime,

    /// When this projection was created/updated
    pub projected_at: DateTime,

    /// Full content (stored but not indexed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub extra_metadata: JsonValue,

    /// Standard metadata (soft delete, timestamps)
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for ContentProjection {
    fn default() -> Self {
        let now = DateTime::now();
        Self {
            mongo_id: None,
            id: String::new(),
            action_hash: String::new(),
            entry_hash: None,
            content_type: String::new(),
            title: String::new(),
            description: String::new(),
            summary: None,
            content_format: String::new(),
            tags: Vec::new(),
            reach: String::new(),
            trust_score: 0.0,
            estimated_minutes: None,
            author: String::new(),
            search_tokens: Vec::new(),
            created_at: now,
            updated_at: now,
            projected_at: now,
            content: None,
            extra_metadata: JsonValue::Null,
            metadata: Metadata::default(),
        }
    }
}

impl ContentProjection {
    /// Create from raw content entry data
    pub fn from_entry(id: String, action_hash: String, author: String, data: &JsonValue) -> Self {
        let now = DateTime::now();

        // Extract fields from JSON
        let content_type = data
            .get("content_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let title = data
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = data
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let summary = data
            .get("summary")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let content_format = data
            .get("content_format")
            .and_then(|v| v.as_str())
            .unwrap_or("markdown")
            .to_string();

        let tags: Vec<String> = data
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let reach = data
            .get("reach")
            .and_then(|v| v.as_str())
            .unwrap_or("private")
            .to_string();

        let trust_score = data
            .get("trust_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        let estimated_minutes = data
            .get("estimated_minutes")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        let content = data
            .get("content")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Generate search tokens from title, description, tags
        let mut search_tokens = Vec::new();
        search_tokens.extend(Self::tokenize(&title));
        search_tokens.extend(Self::tokenize(&description));
        for tag in &tags {
            search_tokens.push(tag.to_lowercase());
        }
        search_tokens.sort();
        search_tokens.dedup();

        Self {
            mongo_id: Some(format!("content-{id}")),
            id,
            action_hash,
            entry_hash: None,
            content_type,
            title,
            description,
            summary,
            content_format,
            tags,
            reach,
            trust_score,
            estimated_minutes,
            author,
            search_tokens,
            created_at: now,
            updated_at: now,
            projected_at: now,
            content,
            extra_metadata: JsonValue::Null,
            metadata: Metadata::new(),
        }
    }

    /// Tokenize text for search
    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() >= 3)
            .map(|word| {
                word.to_lowercase()
                    .chars()
                    .filter(|c| c.is_alphanumeric())
                    .collect()
            })
            .filter(|word: &String| !word.is_empty())
            .collect()
    }

    /// Check if this content is publicly accessible
    pub fn is_public(&self) -> bool {
        matches!(self.reach.as_str(), "public" | "commons")
    }
}

impl IntoIndexes for ContentProjection {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Index by content type
            (doc! { "content_type": 1 }, None),
            // Index by tags
            (doc! { "tags": 1 }, None),
            // Index by reach (for visibility filtering)
            (doc! { "reach": 1 }, None),
            // Index by author
            (doc! { "author": 1 }, None),
            // Compound index for common queries
            (
                doc! { "content_type": 1, "reach": 1, "projected_at": -1 },
                None,
            ),
            // Search tokens index
            (doc! { "search_tokens": 1 }, None),
        ]
    }
}

impl MutMetadata for ContentProjection {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

/// Query parameters for content projections
#[derive(Debug, Clone, Default)]
pub struct ContentQuery {
    /// Filter by content type
    pub content_type: Option<String>,

    /// Filter by multiple content types
    pub content_types: Option<Vec<String>>,

    /// Filter by tags (all must match)
    pub tags: Option<Vec<String>>,

    /// Filter by any of these tags
    pub any_tags: Option<Vec<String>>,

    /// Filter by reach (visibility)
    pub reach: Option<String>,

    /// Only public content (reach = "public" or "commons")
    pub public_only: bool,

    /// Filter by author
    pub author: Option<String>,

    /// Full-text search query
    pub search: Option<String>,

    /// Filter by content IDs
    pub ids: Option<Vec<String>>,

    /// Maximum results
    pub limit: Option<i64>,

    /// Skip for pagination
    pub skip: Option<u64>,
}

impl ContentQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by content type
    pub fn by_type(content_type: impl Into<String>) -> Self {
        Self {
            content_type: Some(content_type.into()),
            ..Default::default()
        }
    }

    /// Filter by tag
    pub fn by_tag(tag: impl Into<String>) -> Self {
        Self {
            tags: Some(vec![tag.into()]),
            ..Default::default()
        }
    }

    /// Only public content
    pub fn public() -> Self {
        Self {
            public_only: true,
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

    /// Convert to MongoDB filter document
    pub fn to_filter(&self) -> Document {
        let mut filter = doc! {};

        if let Some(ref content_type) = self.content_type {
            filter.insert("content_type", content_type);
        }

        if let Some(ref types) = self.content_types {
            filter.insert("content_type", doc! { "$in": types });
        }

        if let Some(ref tags) = self.tags {
            filter.insert("tags", doc! { "$all": tags });
        }

        if let Some(ref tags) = self.any_tags {
            filter.insert("tags", doc! { "$in": tags });
        }

        if let Some(ref reach) = self.reach {
            filter.insert("reach", reach);
        }

        if self.public_only {
            filter.insert("reach", doc! { "$in": ["public", "commons"] });
        }

        if let Some(ref author) = self.author {
            filter.insert("author", author);
        }

        if let Some(ref ids) = self.ids {
            filter.insert("id", doc! { "$in": ids });
        }

        if let Some(ref search) = self.search {
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
    fn test_content_projection_from_entry() {
        let data = serde_json::json!({
            "content_type": "concept",
            "title": "Economic Flows",
            "description": "Understanding value flow in networks",
            "tags": ["economics", "governance"],
            "reach": "commons",
            "trust_score": 0.95,
        });

        let proj = ContentProjection::from_entry(
            "content-123".to_string(),
            "uhCkk...".to_string(),
            "uhCAk...".to_string(),
            &data,
        );

        assert_eq!(proj.content_type, "concept");
        assert_eq!(proj.title, "Economic Flows");
        assert!(proj.is_public());
        assert!(proj.search_tokens.contains(&"economic".to_string()));
    }

    #[test]
    fn test_content_query_filter() {
        let query = ContentQuery::by_type("concept")
            .with_search("economics")
            .with_limit(10);

        let filter = query.to_filter();
        assert_eq!(filter.get_str("content_type").unwrap(), "concept");
    }
}
