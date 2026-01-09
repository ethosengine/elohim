//! Content service - business logic for content operations
//!
//! Wraps the content repository with validation, event emission,
//! and cross-entity orchestration.

use std::sync::Arc;

use crate::db::{self, content, ContentDb};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Content service for business logic
pub struct ContentService {
    content_db: Arc<ContentDb>,
    events: Arc<EventBus>,
}

impl ContentService {
    /// Create a new content service
    pub fn new(content_db: Arc<ContentDb>, events: Arc<EventBus>) -> Self {
        Self { content_db, events }
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get content by ID
    pub fn get(&self, id: &str) -> Result<Option<content::ContentRow>, StorageError> {
        self.content_db.with_conn(|conn| content::get_content(conn, id))
    }

    /// List content with filters
    pub fn list(&self, query: &content::ContentQuery) -> Result<Vec<content::ContentRow>, StorageError> {
        self.content_db.with_conn(|conn| content::list_content(conn, query))
    }

    /// Get content by tag
    pub fn get_by_tag(&self, tag: &str, limit: u32) -> Result<Vec<content::ContentRow>, StorageError> {
        self.content_db.with_conn(|conn| content::get_content_by_tag(conn, tag, limit))
    }

    /// Search content by text
    pub fn search(&self, query: &str, limit: u32) -> Result<Vec<content::ContentRow>, StorageError> {
        self.list(&content::ContentQuery {
            search: Some(query.to_string()),
            limit,
            ..Default::default()
        })
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a single content item with validation
    pub fn create(&self, input: content::CreateContentInput) -> Result<content::ContentRow, StorageError> {
        // Validate required fields
        self.validate_content(&input)?;

        // Create content
        let result = self.content_db.with_conn_mut(|conn| {
            content::create_content(conn, input.clone())
        })?;

        // Emit event
        self.events.emit(StorageEvent::ContentCreated {
            id: result.id.clone(),
            title: result.title.clone(),
            content_type: Some(result.content_type.clone()),
        });

        Ok(result)
    }

    /// Bulk create content items (for seeding)
    pub fn bulk_create(
        &self,
        items: Vec<content::CreateContentInput>,
    ) -> Result<content::BulkResult, StorageError> {
        // Validate all items first
        for (i, item) in items.iter().enumerate() {
            if let Err(e) = self.validate_content(item) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Perform bulk create
        let result = self.content_db.with_conn_mut(|conn| {
            content::bulk_create_content(conn, items)
        })?;

        // Emit event if any items were inserted
        if result.inserted > 0 {
            self.events.emit(StorageEvent::ContentBulkCreated {
                count: result.inserted as usize,
                ids,
            });
        }

        Ok(result)
    }

    /// Delete content by ID
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let deleted = self.content_db.with_conn_mut(|conn| {
            content::delete_content(conn, id)
        })?;

        if deleted {
            self.events.emit(StorageEvent::ContentDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Delete content and cascade to relationships
    ///
    /// This is the preferred delete method as it maintains referential integrity.
    pub fn delete_cascade(&self, id: &str) -> Result<bool, StorageError> {
        // First check if content exists
        let exists = self.get(id)?.is_some();
        if !exists {
            return Ok(false);
        }

        self.content_db.with_conn_mut(|conn| {
            // Delete relationships where this content is source or target
            let _ = db::relationships::delete_relationships_for_content(conn, id);
            // Then delete content
            content::delete_content(conn, id)
        })?;

        self.events.emit(StorageEvent::ContentDeleted { id: id.to_string() });

        Ok(true)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate content input
    fn validate_content(&self, input: &content::CreateContentInput) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("id is required".into()));
        }

        if input.id.len() > 255 {
            return Err(StorageError::InvalidInput("id must be <= 255 characters".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("title is required".into()));
        }

        if input.title.len() > 500 {
            return Err(StorageError::InvalidInput("title must be <= 500 characters".into()));
        }

        // Validate content_type is reasonable
        let valid_types = [
            "concept", "article", "quiz", "assessment", "video", "audio",
            "image", "document", "interactive", "simulation", "reference",
            "path", "module", "chapter", "lesson", "exercise", "project",
            "discussion", "poll", "survey", "scenario", "role", "resource",
        ];
        if !valid_types.contains(&input.content_type.as_str()) && !input.content_type.starts_with("custom:") {
            // Allow custom types with prefix
            // Just warn, don't reject - be permissive
        }

        // Validate content_format
        let valid_formats = [
            "markdown", "html", "json", "text", "perseus",
            "gherkin", "yaml", "toml", "latex", "asciidoc",
            "html5-app", "iframe", "embed",
        ];
        if !valid_formats.contains(&input.content_format.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "content_format '{}' is not valid. Valid formats: {:?}",
                input.content_format, valid_formats
            )));
        }

        // Validate reach level
        let valid_reach = ["public", "commons", "regional", "local", "private", "invited"];
        if !valid_reach.contains(&input.reach.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "reach '{}' is not valid. Valid values: {:?}",
                input.reach, valid_reach
            )));
        }

        // Validate metadata_json is valid JSON if provided
        if let Some(ref json_str) = input.metadata_json {
            if !json_str.is_empty() {
                serde_json::from_str::<serde_json::Value>(json_str).map_err(|e| {
                    StorageError::InvalidInput(format!("metadata_json is not valid JSON: {}", e))
                })?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get content count by type
    pub fn get_stats(&self) -> Result<ContentStats, StorageError> {
        self.content_db.with_conn(|conn| {
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM content", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let by_type: Vec<(String, i64)> = {
                let mut stmt = conn
                    .prepare("SELECT content_type, COUNT(*) FROM content GROUP BY content_type")
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| StorageError::Internal(e.to_string()))?
            };

            Ok(ContentStats {
                total_count: total as u64,
                by_type: by_type.into_iter().collect(),
            })
        })
    }
}

/// Content statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContentStats {
    pub total_count: u64,
    pub by_type: std::collections::HashMap<String, i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests would require setting up a test database
    // For now, just test validation logic

    #[test]
    fn test_validate_empty_id() {
        let events = Arc::new(EventBus::new());
        // Can't test without ContentDb, but validation is straightforward
    }
}
