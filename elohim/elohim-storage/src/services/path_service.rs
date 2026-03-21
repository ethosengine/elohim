//! Path service - business logic for path operations
//!
//! Wraps the path repository with validation, event emission,
//! and cross-entity orchestration (paths + chapters + steps).

use std::sync::Arc;

use diesel::prelude::*;

use crate::db::models::{Chapter, NewChapter, NewStep, Path, PathWithSteps, Step};
use crate::db::{context::AppContext, paths_diesel, DbPool};
use crate::error::StorageError;
use crate::generated_enums::{ALL_PATH_VISIBILITIES, ALL_STEP_TYPES};

use super::events::{EventBus, StorageEvent};

/// Path service for business logic
pub struct PathService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
}

impl PathService {
    /// Create a new path service
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

    /// Get path by ID
    pub fn get(&self, id: &str) -> Result<Option<Path>, StorageError> {
        let mut conn = self.conn()?;
        paths_diesel::get_path(&mut conn, &self.ctx, id)
    }

    /// Get path with all chapters and steps
    pub fn get_with_steps(&self, id: &str) -> Result<Option<PathWithSteps>, StorageError> {
        let mut conn = self.conn()?;
        paths_diesel::get_path_with_steps(&mut conn, &self.ctx, id)
    }

    /// List paths with pagination
    pub fn list(&self, limit: u32, offset: u32) -> Result<Vec<Path>, StorageError> {
        let mut conn = self.conn()?;
        paths_diesel::list_paths(&mut conn, &self.ctx, limit as i64, offset as i64)
    }

    /// Search paths by tag
    pub fn search_by_tag(&self, tag: &str, limit: u32) -> Result<Vec<Path>, StorageError> {
        // Use list and filter by tag (could be optimized with a dedicated query)
        let all_paths = self.list(1000, 0)?;
        // Paths don't have tags directly in the Diesel model; we'd need to join path_tags.
        // For now, fetch tags per path and filter.
        let mut conn = self.conn()?;
        let filtered: Vec<_> = all_paths
            .into_iter()
            .filter(|p| {
                use crate::db::diesel_schema::path_tags;
                let tags: Vec<String> = path_tags::table
                    .filter(path_tags::app_id.eq(&self.ctx.app_id))
                    .filter(path_tags::path_id.eq(&p.id))
                    .select(path_tags::tag)
                    .load(&mut *conn)
                    .unwrap_or_default();
                tags.iter().any(|t| t == tag)
            })
            .take(limit as usize)
            .collect();
        Ok(filtered)
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a path with chapters and steps
    pub fn create(&self, input: paths_diesel::CreatePathInput) -> Result<Path, StorageError> {
        // Validate input
        self.validate_path(&input)?;

        // Create path
        let mut conn = self.conn()?;
        let result = paths_diesel::create_path(&mut conn, &self.ctx, input)?;

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
        items: Vec<paths_diesel::CreatePathInput>,
    ) -> Result<paths_diesel::BulkPathResult, StorageError> {
        // Validate all items first
        for (i, item) in items.iter().enumerate() {
            if let Err(e) = self.validate_path(item) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Perform bulk create
        let mut conn = self.conn()?;
        let result = paths_diesel::bulk_create_paths(&mut conn, &self.ctx, items)?;

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

        let mut conn = self.conn()?;
        let deleted = paths_diesel::delete_path(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::PathDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    // =========================================================================
    // Step Operations
    // =========================================================================

    /// Get steps for a path
    pub fn get_steps(&self, path_id: &str) -> Result<Vec<Step>, StorageError> {
        use crate::db::diesel_schema::steps;
        let mut conn = self.conn()?;
        steps::table
            .filter(steps::app_id.eq(&self.ctx.app_id))
            .filter(steps::path_id.eq(path_id))
            .order(steps::order_index.asc())
            .load(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Steps query failed: {}", e)))
    }

    /// Create a step
    pub fn create_step(
        &self,
        input: paths_diesel::CreateStepInput,
        path_id: &str,
    ) -> Result<Step, StorageError> {
        self.validate_step(&input, path_id)?;

        use crate::db::diesel_schema::steps;
        let mut conn = self.conn()?;

        let new_step = NewStep {
            id: &input.id,
            app_id: &self.ctx.app_id,
            path_id,
            chapter_id: input.chapter_id.as_deref(),
            title: &input.title,
            description: input.description.as_deref(),
            step_type: &input.step_type,
            resource_id: input.resource_id.as_deref(),
            resource_type: input.resource_type.as_deref(),
            order_index: input.order_index,
            estimated_duration: input.estimated_duration.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
        };

        diesel::insert_into(steps::table)
            .values(&new_step)
            .execute(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Step insert failed: {}", e)))?;

        steps::table
            .filter(steps::app_id.eq(&self.ctx.app_id))
            .filter(steps::id.eq(&input.id))
            .first(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Step fetch failed: {}", e)))
    }

    /// Delete a step
    pub fn delete_step(&self, step_id: &str) -> Result<bool, StorageError> {
        use crate::db::diesel_schema::steps;
        let mut conn = self.conn()?;
        let deleted = diesel::delete(
            steps::table
                .filter(steps::app_id.eq(&self.ctx.app_id))
                .filter(steps::id.eq(step_id)),
        )
        .execute(&mut *conn)
        .map_err(|e| StorageError::Internal(format!("Step delete failed: {}", e)))?;
        Ok(deleted > 0)
    }

    // =========================================================================
    // Chapter Operations
    // =========================================================================

    /// Get chapters for a path
    pub fn get_chapters(&self, path_id: &str) -> Result<Vec<Chapter>, StorageError> {
        use crate::db::diesel_schema::chapters;
        let mut conn = self.conn()?;
        chapters::table
            .filter(chapters::app_id.eq(&self.ctx.app_id))
            .filter(chapters::path_id.eq(path_id))
            .order(chapters::order_index.asc())
            .load(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Chapters query failed: {}", e)))
    }

    /// Create a chapter
    pub fn create_chapter(
        &self,
        input: paths_diesel::CreateChapterInput,
        path_id: &str,
    ) -> Result<Chapter, StorageError> {
        self.validate_chapter(&input)?;

        use crate::db::diesel_schema::chapters;
        let mut conn = self.conn()?;

        let new_chapter = NewChapter {
            id: &input.id,
            app_id: &self.ctx.app_id,
            path_id,
            title: &input.title,
            description: input.description.as_deref(),
            order_index: input.order_index,
            estimated_duration: input.estimated_duration.as_deref(),
        };

        diesel::insert_into(chapters::table)
            .values(&new_chapter)
            .execute(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Chapter insert failed: {}", e)))?;

        chapters::table
            .filter(chapters::app_id.eq(&self.ctx.app_id))
            .filter(chapters::id.eq(&input.id))
            .first(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Chapter fetch failed: {}", e)))
    }

    /// Delete a chapter (cascades to steps)
    pub fn delete_chapter(&self, chapter_id: &str) -> Result<bool, StorageError> {
        use crate::db::diesel_schema::chapters;
        let mut conn = self.conn()?;
        let deleted = diesel::delete(
            chapters::table
                .filter(chapters::app_id.eq(&self.ctx.app_id))
                .filter(chapters::id.eq(chapter_id)),
        )
        .execute(&mut *conn)
        .map_err(|e| StorageError::Internal(format!("Chapter delete failed: {}", e)))?;
        Ok(deleted > 0)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate path input
    fn validate_path(&self, input: &paths_diesel::CreatePathInput) -> Result<(), StorageError> {
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

        // Validate path_type
        let valid_types = [
            "guided",
            "self-paced",
            "challenge",
            "assessment",
            "exploration",
            "certification",
        ];
        if !valid_types.contains(&input.path_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "path_type '{}' is not valid. Valid types: {:?}",
                input.path_type, valid_types
            )));
        }

        // Validate visibility — from protocol schema (generated_enums)
        if !ALL_PATH_VISIBILITIES.contains(&input.visibility.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "visibility '{}' is not valid. Valid values: {:?}",
                input.visibility, ALL_PATH_VISIBILITIES
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
                    "chapters[{}]: {}",
                    i, e
                )));
            }
        }

        Ok(())
    }

    /// Validate chapter input
    fn validate_chapter(
        &self,
        input: &paths_diesel::CreateChapterInput,
    ) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("chapter id is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput(
                "chapter title is required".into(),
            ));
        }

        // Validate steps
        for (i, step) in input.steps.iter().enumerate() {
            if let Err(e) = self.validate_step_input(step) {
                return Err(StorageError::InvalidInput(format!("steps[{}]: {}", i, e)));
            }
        }

        Ok(())
    }

    /// Validate step input (for standalone step creation)
    fn validate_step(
        &self,
        input: &paths_diesel::CreateStepInput,
        path_id: &str,
    ) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("step id is required".into()));
        }

        if path_id.is_empty() {
            return Err(StorageError::InvalidInput(
                "step path_id is required".into(),
            ));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("step title is required".into()));
        }

        self.validate_step_type(&input.step_type)
    }

    /// Validate step input (for embedded step in chapter)
    fn validate_step_input(
        &self,
        input: &paths_diesel::CreateStepInput,
    ) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("step id is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("step title is required".into()));
        }

        self.validate_step_type(&input.step_type)
    }

    /// Validate step_type value — from protocol schema (generated_enums)
    fn validate_step_type(&self, step_type: &str) -> Result<(), StorageError> {
        if !ALL_STEP_TYPES.contains(&step_type) {
            return Err(StorageError::InvalidInput(format!(
                "step_type '{}' is not valid. Valid types: {:?}",
                step_type, ALL_STEP_TYPES
            )));
        }
        Ok(())
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get path statistics
    pub fn get_stats(&self) -> Result<PathStats, StorageError> {
        let mut conn = self.conn()?;
        let total = paths_diesel::path_count(&mut conn, &self.ctx)? as u64;
        let total_steps = paths_diesel::total_step_count(&mut conn, &self.ctx)? as u64;

        // Chapter count via Diesel
        use crate::db::diesel_schema::chapters;
        let total_chapters: i64 = chapters::table
            .filter(chapters::app_id.eq(&self.ctx.app_id))
            .count()
            .get_result(&mut *conn)
            .map_err(|e| StorageError::Internal(format!("Chapter count failed: {}", e)))?;

        Ok(PathStats {
            total_paths: total,
            total_steps,
            total_chapters: total_chapters as u64,
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
