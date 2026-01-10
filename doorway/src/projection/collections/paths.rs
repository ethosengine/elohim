//! Learning Path projection schema
//!
//! Optimized projection for LearningPath entries with step counts,
//! chapter structure, and difficulty indexing.

use bson::{doc, DateTime, Document};
use mongodb::options::IndexOptions;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::schemas::Metadata;
use crate::db::mongo::{IntoIndexes, MutMetadata};

/// Projected chapter structure (denormalized from metadata)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChapterOverview {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub step_count: usize,
}

/// Projected learning path document
///
/// Denormalized view of LearningPath entries with step counts
/// and chapter structure for fast listing and navigation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathProjection {
    /// MongoDB document ID (format: "path-{id}")
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub mongo_id: Option<String>,

    /// Path ID from the entry
    pub id: String,

    /// Path version
    pub version: String,

    /// Holochain action hash
    pub action_hash: String,

    /// Holochain entry hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry_hash: Option<String>,

    /// Path title
    pub title: String,

    /// Path description
    pub description: String,

    /// Path purpose statement
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    /// Difficulty level
    pub difficulty: String,

    /// Estimated duration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<String>,

    /// Tags for filtering
    #[serde(default)]
    pub tags: Vec<String>,

    /// Visibility
    pub visibility: String,

    /// Author agent pubkey
    pub author: String,

    /// Total number of steps
    pub step_count: usize,

    /// Number of chapters
    pub chapter_count: usize,

    /// Chapter overview (from metadata)
    #[serde(default)]
    pub chapters: Vec<ChapterOverview>,

    /// Search tokens for full-text search
    #[serde(default)]
    pub search_tokens: Vec<String>,

    /// Original entry creation timestamp
    pub created_at: DateTime,

    /// Last update timestamp from entry
    pub updated_at: DateTime,

    /// When this projection was created/updated
    pub projected_at: DateTime,

    /// Standard metadata (soft delete, timestamps)
    #[serde(default)]
    pub metadata: Metadata,
}

impl Default for PathProjection {
    fn default() -> Self {
        let now = DateTime::now();
        Self {
            mongo_id: None,
            id: String::new(),
            version: String::new(),
            action_hash: String::new(),
            entry_hash: None,
            title: String::new(),
            description: String::new(),
            purpose: None,
            difficulty: String::new(),
            estimated_duration: None,
            tags: Vec::new(),
            visibility: String::new(),
            author: String::new(),
            step_count: 0,
            chapter_count: 0,
            chapters: Vec::new(),
            search_tokens: Vec::new(),
            created_at: now,
            updated_at: now,
            projected_at: now,
            metadata: Metadata::default(),
        }
    }
}

impl PathProjection {
    /// Create from raw path entry data and step count
    pub fn from_entry(
        id: String,
        action_hash: String,
        author: String,
        data: &JsonValue,
        step_count: usize,
    ) -> Self {
        let now = DateTime::now();

        // Extract fields from JSON
        let version = data.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let title = data.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = data.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let purpose = data.get("purpose")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let difficulty = data.get("difficulty")
            .and_then(|v| v.as_str())
            .unwrap_or("beginner")
            .to_string();

        let estimated_duration = data.get("estimated_duration")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let tags: Vec<String> = data.get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
            .unwrap_or_default();

        let visibility = data.get("visibility")
            .and_then(|v| v.as_str())
            .unwrap_or("private")
            .to_string();

        // Extract chapters from metadata_json
        let (chapters, chapter_count) = Self::extract_chapters(data);

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
            mongo_id: Some(format!("path-{}", id)),
            id,
            version,
            action_hash,
            entry_hash: None,
            title,
            description,
            purpose,
            difficulty,
            estimated_duration,
            tags,
            visibility,
            author,
            step_count,
            chapter_count,
            chapters,
            search_tokens,
            created_at: now,
            updated_at: now,
            projected_at: now,
            metadata: Metadata::new(),
        }
    }

    /// Extract chapter structure from metadata_json
    fn extract_chapters(data: &JsonValue) -> (Vec<ChapterOverview>, usize) {
        // Try to parse metadata_json if present
        let metadata_json = data.get("metadata_json")
            .and_then(|v| v.as_str())
            .unwrap_or("{}");

        let metadata: JsonValue = serde_json::from_str(metadata_json).unwrap_or(JsonValue::Null);

        let chapters_array = metadata.get("chapters")
            .and_then(|v| v.as_array());

        match chapters_array {
            Some(arr) => {
                let chapters: Vec<ChapterOverview> = arr.iter()
                    .filter_map(|ch| {
                        let id = ch.get("id")?.as_str()?.to_string();
                        let title = ch.get("title")?.as_str()?.to_string();
                        let description = ch.get("description")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string());

                        // Count modules/sections in this chapter
                        let modules = ch.get("modules")
                            .and_then(|v| v.as_array())
                            .map(|m| m.len())
                            .unwrap_or(0);

                        Some(ChapterOverview {
                            id,
                            title,
                            description,
                            step_count: modules, // Approximate
                        })
                    })
                    .collect();

                let count = chapters.len();
                (chapters, count)
            }
            None => (Vec::new(), 0),
        }
    }

    /// Tokenize text for search
    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .filter(|word| word.len() >= 3)
            .map(|word| word.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect())
            .filter(|word: &String| !word.is_empty())
            .collect()
    }

    /// Check if this path is publicly visible
    pub fn is_public(&self) -> bool {
        matches!(self.visibility.as_str(), "public" | "published")
    }
}

impl IntoIndexes for PathProjection {
    fn into_indices() -> Vec<(Document, Option<IndexOptions>)> {
        vec![
            // Index by difficulty
            (doc! { "difficulty": 1 }, None),
            // Index by tags
            (doc! { "tags": 1 }, None),
            // Index by visibility
            (doc! { "visibility": 1 }, None),
            // Index by author
            (doc! { "author": 1 }, None),
            // Compound index for common queries
            (doc! { "visibility": 1, "difficulty": 1, "projected_at": -1 }, None),
            // Search tokens index
            (doc! { "search_tokens": 1 }, None),
        ]
    }
}

impl MutMetadata for PathProjection {
    fn mut_metadata(&mut self) -> &mut Metadata {
        &mut self.metadata
    }
}

/// Query parameters for path projections
#[derive(Debug, Clone, Default)]
pub struct PathQuery {
    /// Filter by difficulty
    pub difficulty: Option<String>,

    /// Filter by multiple difficulties
    pub difficulties: Option<Vec<String>>,

    /// Filter by tags (all must match)
    pub tags: Option<Vec<String>>,

    /// Filter by visibility
    pub visibility: Option<String>,

    /// Only public paths
    pub public_only: bool,

    /// Filter by author
    pub author: Option<String>,

    /// Full-text search query
    pub search: Option<String>,

    /// Filter by path IDs
    pub ids: Option<Vec<String>>,

    /// Maximum results
    pub limit: Option<i64>,

    /// Skip for pagination
    pub skip: Option<u64>,
}

impl PathQuery {
    /// Create a new query
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by difficulty
    pub fn by_difficulty(difficulty: impl Into<String>) -> Self {
        Self {
            difficulty: Some(difficulty.into()),
            ..Default::default()
        }
    }

    /// Only public paths
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

        if let Some(ref difficulty) = self.difficulty {
            filter.insert("difficulty", difficulty);
        }

        if let Some(ref difficulties) = self.difficulties {
            filter.insert("difficulty", doc! { "$in": difficulties });
        }

        if let Some(ref tags) = self.tags {
            filter.insert("tags", doc! { "$all": tags });
        }

        if let Some(ref visibility) = self.visibility {
            filter.insert("visibility", visibility);
        }

        if self.public_only {
            filter.insert("visibility", doc! { "$in": ["public", "published"] });
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
    fn test_path_projection_from_entry() {
        let data = serde_json::json!({
            "title": "Elohim Protocol",
            "description": "Learn the foundational concepts",
            "difficulty": "intermediate",
            "visibility": "public",
            "tags": ["governance", "economics"],
            "metadata_json": r#"{"chapters":[{"id":"ch1","title":"Introduction","modules":[]}]}"#,
        });

        let proj = PathProjection::from_entry(
            "elohim-protocol".to_string(),
            "uhCkk...".to_string(),
            "uhCAk...".to_string(),
            &data,
            47,
        );

        assert_eq!(proj.title, "Elohim Protocol");
        assert_eq!(proj.step_count, 47);
        assert_eq!(proj.chapter_count, 1);
        assert!(proj.is_public());
    }

    #[test]
    fn test_path_query_filter() {
        let query = PathQuery::public()
            .with_search("governance")
            .with_limit(10);

        let filter = query.to_filter();
        assert!(filter.contains_key("visibility"));
    }
}
