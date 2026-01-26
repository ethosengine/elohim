//! Path service - business logic for path operations
//!
//! Wraps the path repository with validation, event emission,
//! and cross-entity orchestration (paths + chapters + steps).

use std::sync::Arc;

use crate::db::{paths, ContentDb};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Path service for business logic
pub struct PathService {
    content_db: Arc<ContentDb>,
    events: Arc<EventBus>,
}

impl PathService {
    /// Create a new path service
    pub fn new(content_db: Arc<ContentDb>, events: Arc<EventBus>) -> Self {
        Self { content_db, events }
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get path by ID
    pub fn get(&self, id: &str) -> Result<Option<paths::PathRow>, StorageError> {
        self.content_db.with_conn(|conn| paths::get_path(conn, id))
    }

    /// Get path with all chapters and steps
    pub fn get_with_steps(&self, id: &str) -> Result<Option<paths::PathWithSteps>, StorageError> {
        self.content_db.with_conn(|conn| paths::get_path_with_steps(conn, id))
    }

    /// List paths with pagination
    pub fn list(&self, limit: u32, offset: u32) -> Result<Vec<paths::PathRow>, StorageError> {
        self.content_db.with_conn(|conn| paths::list_paths(conn, limit, offset))
    }

    /// Search paths by tag
    pub fn search_by_tag(&self, tag: &str, limit: u32) -> Result<Vec<paths::PathRow>, StorageError> {
        // Use list and filter by tag (could be optimized with a dedicated query)
        let all_paths = self.list(1000, 0)?;
        let filtered: Vec<_> = all_paths
            .into_iter()
            .filter(|p| p.tags.iter().any(|t| t == tag))
            .take(limit as usize)
            .collect();
        Ok(filtered)
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a path with chapters and steps
    pub fn create(&self, input: paths::CreatePathInput) -> Result<paths::PathRow, StorageError> {
        // Validate input
        self.validate_path(&input)?;

        // Create path
        let result = self.content_db.with_conn_mut(|conn| {
            paths::create_path(conn, input.clone())
        })?;

        // Emit event
        self.events.emit(StorageEvent::PathCreated {
            id: result.id.clone(),
            title: result.title.clone(),
        });

        Ok(result)
    }

    /// Bulk create paths (for seeding)
    pub fn bulk_create(
        &self,
        items: Vec<paths::CreatePathInput>,
    ) -> Result<paths::BulkPathResult, StorageError> {
        // Validate all items first
        for (i, item) in items.iter().enumerate() {
            if let Err(e) = self.validate_path(item) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Perform bulk create
        let result = self.content_db.with_conn_mut(|conn| {
            paths::bulk_create_paths(conn, items)
        })?;

        // Emit event if any items were inserted
        if result.inserted > 0 {
            self.events.emit(StorageEvent::PathBulkCreated {
                count: result.inserted as usize,
                ids,
            });
        }

        Ok(result)
    }

    /// Delete path by ID (cascades to chapters and steps)
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        // First check if path exists
        let exists = self.get(id)?.is_some();
        if !exists {
            return Ok(false);
        }

        let deleted = self.content_db.with_conn_mut(|conn| {
            paths::delete_path(conn, id)
        })?;

        if deleted {
            self.events.emit(StorageEvent::PathDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    // =========================================================================
    // Step Operations
    // =========================================================================

    /// Get steps for a path
    pub fn get_steps(&self, path_id: &str) -> Result<Vec<paths::StepRow>, StorageError> {
        self.content_db.with_conn(|conn| paths::get_steps_for_path(conn, path_id))
    }

    /// Create a step
    pub fn create_step(&self, input: paths::CreateStepInput) -> Result<paths::StepRow, StorageError> {
        self.validate_step(&input)?;

        self.content_db.with_conn_mut(|conn| {
            paths::create_step(conn, input)
        })
    }

    /// Delete a step
    pub fn delete_step(&self, step_id: &str) -> Result<bool, StorageError> {
        self.content_db.with_conn_mut(|conn| {
            paths::delete_step(conn, step_id)
        })
    }

    // =========================================================================
    // Chapter Operations
    // =========================================================================

    /// Get chapters for a path
    pub fn get_chapters(&self, path_id: &str) -> Result<Vec<paths::ChapterRow>, StorageError> {
        self.content_db.with_conn(|conn| paths::get_chapters(conn, path_id))
    }

    /// Create a chapter
    pub fn create_chapter(&self, input: paths::CreateChapterInput, path_id: &str) -> Result<paths::ChapterRow, StorageError> {
        self.validate_chapter(&input)?;

        self.content_db.with_conn_mut(|conn| {
            paths::create_chapter(conn, path_id, input)
        })
    }

    /// Delete a chapter (cascades to steps)
    pub fn delete_chapter(&self, chapter_id: &str) -> Result<bool, StorageError> {
        self.content_db.with_conn_mut(|conn| {
            paths::delete_chapter(conn, chapter_id)
        })
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate path input
    fn validate_path(&self, input: &paths::CreatePathInput) -> Result<(), StorageError> {
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

        // Validate path_type
        let valid_types = ["guided", "self-paced", "challenge", "assessment", "exploration", "certification"];
        if !valid_types.contains(&input.path_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "path_type '{}' is not valid. Valid types: {:?}",
                input.path_type, valid_types
            )));
        }

        // Validate visibility
        let valid_visibility = ["public", "private", "unlisted", "draft"];
        if !valid_visibility.contains(&input.visibility.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "visibility '{}' is not valid. Valid values: {:?}",
                input.visibility, valid_visibility
            )));
        }

        // Validate difficulty if provided
        if let Some(ref difficulty) = input.difficulty {
            let valid_difficulties = ["beginner", "intermediate", "advanced", "expert"];
            if !valid_difficulties.contains(&difficulty.as_str()) {
                return Err(StorageError::InvalidInput(format!(
                    "difficulty '{}' is not valid. Valid values: {:?}",
                    difficulty, valid_difficulties
                )));
            }
        }

        // Validate metadata_json is valid JSON if provided
        if let Some(ref json_str) = input.metadata_json {
            if !json_str.is_empty() {
                serde_json::from_str::<serde_json::Value>(json_str).map_err(|e| {
                    StorageError::InvalidInput(format!("metadata_json is not valid JSON: {}", e))
                })?;
            }
        }

        // Validate chapters
        for (i, chapter) in input.chapters.iter().enumerate() {
            if let Err(e) = self.validate_chapter(chapter) {
                return Err(StorageError::InvalidInput(format!(
                    "chapters[{}]: {}", i, e
                )));
            }
        }

        Ok(())
    }

    /// Validate chapter input
    fn validate_chapter(&self, input: &paths::CreateChapterInput) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("chapter id is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("chapter title is required".into()));
        }

        // Validate steps
        for (i, step) in input.steps.iter().enumerate() {
            if let Err(e) = self.validate_step(step) {
                return Err(StorageError::InvalidInput(format!(
                    "steps[{}]: {}", i, e
                )));
            }
        }

        Ok(())
    }

    /// Validate step input
    fn validate_step(&self, input: &paths::CreateStepInput) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("step id is required".into()));
        }

        if input.path_id.is_empty() {
            return Err(StorageError::InvalidInput("step path_id is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("step title is required".into()));
        }

        // Validate step_type
        let valid_types = [
            "learn", "practice", "quiz", "assessment", "discussion",
            "project", "resource", "video", "reading", "checkpoint",
        ];
        if !valid_types.contains(&input.step_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "step_type '{}' is not valid. Valid types: {:?}",
                input.step_type, valid_types
            )));
        }

        Ok(())
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get path statistics
    pub fn get_stats(&self) -> Result<PathStats, StorageError> {
        self.content_db.with_conn(|conn| {
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM paths", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let total_steps: i64 = conn
                .query_row("SELECT COUNT(*) FROM steps", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let total_chapters: i64 = conn
                .query_row("SELECT COUNT(*) FROM chapters", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            Ok(PathStats {
                total_paths: total as u64,
                total_steps: total_steps as u64,
                total_chapters: total_chapters as u64,
            })
        })
    }
}

/// Path statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct PathStats {
    pub total_paths: u64,
    pub total_steps: u64,
    pub total_chapters: u64,
}
