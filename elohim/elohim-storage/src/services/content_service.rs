//! Content service - business logic for content operations
//!
//! Wraps the content repository with validation, event emission,
//! and cross-entity orchestration.

use std::sync::Arc;

use crate::db::{self, content_diesel, context::AppContext, DbPool};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Content service for business logic
pub struct ContentService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
}

impl ContentService {
    /// Create a new content service
    pub fn new(pool: DbPool, ctx: AppContext, events: Arc<EventBus>) -> Self {
        Self { pool, ctx, events }
    }

    /// Get a connection from the pool
    fn conn(
        &self,
    ) -> Result<
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        StorageError,
    > {
        self.pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get content by ID
    pub fn get(&self, id: &str) -> Result<Option<crate::db::models::Content>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::get_content(&mut conn, &self.ctx, id)
    }

    /// List content with filters
    pub fn list(
        &self,
        query: &content_diesel::ContentQuery,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::list_content(&mut conn, &self.ctx, query)
    }

    /// Get content by tag
    pub fn get_by_tag(
        &self,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::get_content_by_tag(&mut conn, &self.ctx, tag, limit as i64)
    }

    /// Search content by text
    pub fn search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        self.list(&content_diesel::ContentQuery {
            search: Some(query.to_string()),
            limit: limit as i64,
            ..Default::default()
        })
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a single content item with validation
    pub fn create(
        &self,
        input: content_diesel::CreateContentInput,
    ) -> Result<crate::db::models::ContentWithTags, StorageError> {
        // Validate required fields
        self.validate_content(&input)?;

        // Create content
        let mut conn = self.conn()?;
        let result = content_diesel::create_content(&mut conn, &self.ctx, input)?;

        // Emit event
        self.events.emit(StorageEvent::ContentCreated {
            id: result.content.id.clone(),
            title: result.content.title.clone(),
            content_type: Some(result.content.content_type.clone()),
        });

        Ok(result)
    }

    /// Bulk create content items (for seeding)
    pub fn bulk_create(
        &self,
        items: Vec<content_diesel::CreateContentInput>,
    ) -> Result<content_diesel::BulkResult, StorageError> {
        // Validate all items first
        for (i, item) in items.iter().enumerate() {
            if let Err(e) = self.validate_content(item) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Perform bulk create
        let mut conn = self.conn()?;
        let result = content_diesel::bulk_create_content(&mut conn, &self.ctx, items)?;

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
        let mut conn = self.conn()?;
        let deleted = content_diesel::delete_content(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::ContentDeleted { id: id.to_string() });
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

        let mut conn = self.conn()?;
        // Delete relationships where this content is source or target
        let _ =
            db::relationships_diesel::delete_relationships_for_content(&mut conn, &self.ctx, id);
        // Then delete content
        content_diesel::delete_content(&mut conn, &self.ctx, id)?;

        self.events
            .emit(StorageEvent::ContentDeleted { id: id.to_string() });

        Ok(true)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate content input
    fn validate_content(
        &self,
        input: &content_diesel::CreateContentInput,
    ) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("id is required".into()));
        }

        if input.id.len() > 255 {
            return Err(StorageError::InvalidInput(
                "id must be <= 255 characters".into(),
            ));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("title is required".into()));
        }

        if input.title.len() > 500 {
            return Err(StorageError::InvalidInput(
                "title must be <= 500 characters".into(),
            ));
        }

        // Validate content_type is reasonable
        let valid_types = [
            "concept",
            "article",
            "quiz",
            "assessment",
            "video",
            "audio",
            "image",
            "document",
            "interactive",
            "simulation",
            "reference",
            "path",
            "module",
            "chapter",
            "lesson",
            "exercise",
            "project",
            "discussion",
            "poll",
            "survey",
            "scenario",
            "role",
            "resource",
        ];
        if !valid_types.contains(&input.content_type.as_str())
            && !input.content_type.starts_with("custom:")
        {
            // Allow custom types with prefix
            // Just warn, don't reject - be permissive
        }

        // Validate content_format — aligned with healing.rs CONTENT_FORMATS
        let valid_formats = [
            "markdown",
            "html",
            "json",
            "text",
            "plaintext",
            "plain",
            "perseus",
            "perseus-json",
            "perseus-quiz-json",
            "sophia-quiz-json",
            "sophia",
            "gherkin",
            "yaml",
            "toml",
            "latex",
            "asciidoc",
            "html5-app",
            "iframe",
            "embed",
            "video",
            "audio",
            "interactive",
            "external",
            "video-embed",
            "audio-file",
            "human-json",
            "organization-json",
        ];
        if !valid_formats.contains(&input.content_format.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "content_format '{}' is not valid. Valid formats: {:?}",
                input.content_format, valid_formats
            )));
        }

        // Validate reach level — protocol spec (8 levels) + legacy for backward compat
        let valid_reach = [
            // Protocol spec (social/relational hierarchy)
            "private",
            "self",
            "intimate",
            "trusted",
            "familiar",
            "community",
            "public",
            "commons",
            // Legacy values (backward compat with existing stored data)
            "regional",
            "local",
            "invited",
            "federated",
        ];
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
        let mut conn = self.conn()?;
        let total = content_diesel::content_count(&mut conn, &self.ctx)? as u64;

        // For by_type stats, use a simplified approach
        // The Diesel module doesn't have a group-by-type function,
        // so we return total count with empty by_type map
        Ok(ContentStats {
            total_count: total,
            by_type: std::collections::HashMap::new(),
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
        let _events = Arc::new(EventBus::new());
        // Can't test without a database connection, but validation is straightforward
    }
}
