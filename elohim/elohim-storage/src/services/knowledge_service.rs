//! Knowledge service - business logic for knowledge maps and path extensions
//!
//! Wraps the knowledge_maps and path_extensions repositories with
//! validation and event emission.

use std::sync::Arc;

use crate::db::{context::AppContext, knowledge_maps_diesel, path_extensions_diesel, DbPool};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Knowledge service for user personalization operations
pub struct KnowledgeService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
}

impl KnowledgeService {
    /// Create a new knowledge service
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
    // Knowledge Map Operations
    // =========================================================================

    /// Get knowledge map by ID
    pub fn get_knowledge_map(
        &self,
        id: &str,
    ) -> Result<Option<crate::db::models::KnowledgeMap>, StorageError> {
        let mut conn = self.conn()?;
        knowledge_maps_diesel::get_knowledge_map(&mut conn, &self.ctx, id)
    }

    /// List knowledge maps with filtering
    pub fn list_knowledge_maps(
        &self,
        query: &knowledge_maps_diesel::KnowledgeMapQuery,
    ) -> Result<Vec<crate::db::models::KnowledgeMap>, StorageError> {
        let mut conn = self.conn()?;
        knowledge_maps_diesel::list_knowledge_maps(&mut conn, &self.ctx, query)
    }

    /// Get knowledge maps for an owner
    pub fn get_knowledge_maps_for_owner(
        &self,
        owner_id: &str,
    ) -> Result<Vec<crate::db::models::KnowledgeMap>, StorageError> {
        self.list_knowledge_maps(&knowledge_maps_diesel::KnowledgeMapQuery {
            owner_id: Some(owner_id.to_string()),
            ..Default::default()
        })
    }

    /// Create a knowledge map
    pub fn create_knowledge_map(
        &self,
        input: knowledge_maps_diesel::CreateKnowledgeMapInput,
    ) -> Result<crate::db::models::KnowledgeMap, StorageError> {
        // Validate input
        self.validate_knowledge_map(&input)?;

        // Create knowledge map
        let mut conn = self.conn()?;
        let result =
            knowledge_maps_diesel::create_knowledge_map(&mut conn, &self.ctx, input)?;

        // Emit event
        self.events.emit(StorageEvent::KnowledgeMapCreated {
            id: result.id.clone(),
            map_type: result.map_type.clone(),
            owner_id: result.owner_id.clone(),
        });

        Ok(result)
    }

    /// Update a knowledge map
    pub fn update_knowledge_map(
        &self,
        id: &str,
        input: knowledge_maps_diesel::CreateKnowledgeMapInput,
    ) -> Result<crate::db::models::KnowledgeMap, StorageError> {
        // Validate input
        self.validate_knowledge_map(&input)?;

        // Check if exists
        if self.get_knowledge_map(id)?.is_none() {
            return Err(StorageError::NotFound(format!(
                "Knowledge map '{}' not found",
                id
            )));
        }

        // Update
        let mut conn = self.conn()?;
        let result =
            knowledge_maps_diesel::update_knowledge_map(&mut conn, &self.ctx, id, input)?;

        // Emit event
        self.events
            .emit(StorageEvent::KnowledgeMapUpdated { id: id.to_string() });

        Ok(result)
    }

    /// Delete a knowledge map
    pub fn delete_knowledge_map(&self, id: &str) -> Result<bool, StorageError> {
        let mut conn = self.conn()?;
        let deleted =
            knowledge_maps_diesel::delete_knowledge_map(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::KnowledgeMapDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Validate knowledge map input
    fn validate_knowledge_map(
        &self,
        input: &knowledge_maps_diesel::CreateKnowledgeMapInput,
    ) -> Result<(), StorageError> {
        if input.owner_id.is_empty() {
            return Err(StorageError::InvalidInput("owner_id is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("title is required".into()));
        }

        if input.subject_id.is_empty() {
            return Err(StorageError::InvalidInput("subject_id is required".into()));
        }

        if input.subject_name.is_empty() {
            return Err(StorageError::InvalidInput(
                "subject_name is required".into(),
            ));
        }

        // Validate map_type
        let valid_types = ["domain", "self", "person", "collective"];
        if !valid_types.contains(&input.map_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "map_type '{}' is not valid. Valid types: {:?}",
                input.map_type, valid_types
            )));
        }

        // Validate visibility
        let valid_visibility = ["private", "shared", "public"];
        if !valid_visibility.contains(&input.visibility.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "visibility '{}' is not valid. Valid values: {:?}",
                input.visibility, valid_visibility
            )));
        }

        // Validate nodes_json is valid JSON
        if !input.nodes_json.is_empty() {
            serde_json::from_str::<serde_json::Value>(&input.nodes_json).map_err(|e| {
                StorageError::InvalidInput(format!("nodes_json is not valid JSON: {}", e))
            })?;
        }

        // Validate affinity range
        if input.overall_affinity < 0.0 || input.overall_affinity > 1.0 {
            return Err(StorageError::InvalidInput(
                "overall_affinity must be between 0.0 and 1.0".into(),
            ));
        }

        Ok(())
    }

    // =========================================================================
    // Path Extension Operations
    // =========================================================================

    /// Get path extension by ID
    pub fn get_path_extension(
        &self,
        id: &str,
    ) -> Result<Option<crate::db::models::PathExtension>, StorageError> {
        let mut conn = self.conn()?;
        path_extensions_diesel::get_path_extension(&mut conn, &self.ctx, id)
    }

    /// List path extensions with filtering
    pub fn list_path_extensions(
        &self,
        query: &path_extensions_diesel::PathExtensionQuery,
    ) -> Result<Vec<crate::db::models::PathExtension>, StorageError> {
        let mut conn = self.conn()?;
        path_extensions_diesel::list_path_extensions(&mut conn, &self.ctx, query)
    }

    /// Get path extensions for a base path
    pub fn get_extensions_for_path(
        &self,
        base_path_id: &str,
    ) -> Result<Vec<crate::db::models::PathExtension>, StorageError> {
        self.list_path_extensions(&path_extensions_diesel::PathExtensionQuery {
            base_path_id: Some(base_path_id.to_string()),
            ..Default::default()
        })
    }

    /// Create a path extension
    pub fn create_path_extension(
        &self,
        input: path_extensions_diesel::CreatePathExtensionInput,
    ) -> Result<crate::db::models::PathExtension, StorageError> {
        // Validate input
        self.validate_path_extension(&input)?;

        // Create path extension
        let mut conn = self.conn()?;
        let result =
            path_extensions_diesel::create_path_extension(&mut conn, &self.ctx, input)?;

        // Emit event
        self.events.emit(StorageEvent::PathExtensionCreated {
            id: result.id.clone(),
            base_path_id: result.base_path_id.clone(),
            extended_by: result.extended_by.clone(),
        });

        Ok(result)
    }

    /// Update a path extension
    pub fn update_path_extension(
        &self,
        id: &str,
        input: path_extensions_diesel::CreatePathExtensionInput,
    ) -> Result<crate::db::models::PathExtension, StorageError> {
        // Validate input
        self.validate_path_extension(&input)?;

        // Check if exists
        if self.get_path_extension(id)?.is_none() {
            return Err(StorageError::NotFound(format!(
                "Path extension '{}' not found",
                id
            )));
        }

        // Update
        let mut conn = self.conn()?;
        let result =
            path_extensions_diesel::update_path_extension(&mut conn, &self.ctx, id, input)?;

        // Emit event
        self.events
            .emit(StorageEvent::PathExtensionUpdated { id: id.to_string() });

        Ok(result)
    }

    /// Delete a path extension
    pub fn delete_path_extension(&self, id: &str) -> Result<bool, StorageError> {
        let mut conn = self.conn()?;
        let deleted =
            path_extensions_diesel::delete_path_extension(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::PathExtensionDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Validate path extension input
    fn validate_path_extension(
        &self,
        input: &path_extensions_diesel::CreatePathExtensionInput,
    ) -> Result<(), StorageError> {
        if input.base_path_id.is_empty() {
            return Err(StorageError::InvalidInput(
                "base_path_id is required".into(),
            ));
        }

        if input.extended_by.is_empty() {
            return Err(StorageError::InvalidInput("extended_by is required".into()));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("title is required".into()));
        }

        // Validate visibility
        let valid_visibility = ["private", "shared", "public"];
        if !valid_visibility.contains(&input.visibility.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "visibility '{}' is not valid. Valid values: {:?}",
                input.visibility, valid_visibility
            )));
        }

        // Validate insertions_json is valid JSON if provided
        if let Some(ref json_str) = input.insertions_json {
            if !json_str.is_empty() {
                serde_json::from_str::<serde_json::Value>(json_str).map_err(|e| {
                    StorageError::InvalidInput(format!("insertions_json is not valid JSON: {}", e))
                })?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get knowledge statistics
    pub fn get_stats(&self) -> Result<KnowledgeStats, StorageError> {
        let mut conn = self.conn()?;
        let total_maps =
            knowledge_maps_diesel::knowledge_map_count(&mut conn, &self.ctx)? as u64;
        let total_extensions =
            path_extensions_diesel::path_extension_count(&mut conn, &self.ctx)? as u64;

        Ok(KnowledgeStats {
            total_knowledge_maps: total_maps,
            total_path_extensions: total_extensions,
        })
    }
}

/// Knowledge statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeStats {
    pub total_knowledge_maps: u64,
    pub total_path_extensions: u64,
}
