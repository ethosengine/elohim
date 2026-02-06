//! Knowledge service - business logic for knowledge maps and path extensions
//!
//! Wraps the knowledge_maps and path_extensions repositories with
//! validation and event emission.

use std::sync::Arc;

use crate::db::{knowledge_maps, path_extensions, ContentDb};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Knowledge service for user personalization operations
pub struct KnowledgeService {
    content_db: Arc<ContentDb>,
    events: Arc<EventBus>,
}

impl KnowledgeService {
    /// Create a new knowledge service
    pub fn new(content_db: Arc<ContentDb>, events: Arc<EventBus>) -> Self {
        Self { content_db, events }
    }

    // =========================================================================
    // Knowledge Map Operations
    // =========================================================================

    /// Get knowledge map by ID
    pub fn get_knowledge_map(&self, id: &str) -> Result<Option<knowledge_maps::KnowledgeMapRow>, StorageError> {
        self.content_db.with_conn(|conn| knowledge_maps::get_knowledge_map(conn, id))
    }

    /// List knowledge maps with filtering
    pub fn list_knowledge_maps(
        &self,
        query: &knowledge_maps::KnowledgeMapQuery,
    ) -> Result<Vec<knowledge_maps::KnowledgeMapRow>, StorageError> {
        self.content_db.with_conn(|conn| knowledge_maps::list_knowledge_maps(conn, query))
    }

    /// Get knowledge maps for an owner
    pub fn get_knowledge_maps_for_owner(
        &self,
        owner_id: &str,
    ) -> Result<Vec<knowledge_maps::KnowledgeMapRow>, StorageError> {
        self.list_knowledge_maps(&knowledge_maps::KnowledgeMapQuery {
            owner_id: Some(owner_id.to_string()),
            ..Default::default()
        })
    }

    /// Create a knowledge map
    pub fn create_knowledge_map(
        &self,
        input: knowledge_maps::CreateKnowledgeMapInput,
    ) -> Result<knowledge_maps::KnowledgeMapRow, StorageError> {
        // Validate input
        self.validate_knowledge_map(&input)?;

        // Create knowledge map
        let result = self.content_db.with_conn_mut(|conn| {
            knowledge_maps::create_knowledge_map(conn, input.clone())
        })?;

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
        input: knowledge_maps::CreateKnowledgeMapInput,
    ) -> Result<knowledge_maps::KnowledgeMapRow, StorageError> {
        // Validate input
        self.validate_knowledge_map(&input)?;

        // Check if exists
        if self.get_knowledge_map(id)?.is_none() {
            return Err(StorageError::NotFound(format!("Knowledge map '{}' not found", id)));
        }

        // Update
        let result = self.content_db.with_conn_mut(|conn| {
            knowledge_maps::update_knowledge_map(conn, id, input)
        })?;

        // Emit event
        self.events.emit(StorageEvent::KnowledgeMapUpdated { id: id.to_string() });

        Ok(result)
    }

    /// Delete a knowledge map
    pub fn delete_knowledge_map(&self, id: &str) -> Result<bool, StorageError> {
        let deleted = self.content_db.with_conn_mut(|conn| {
            knowledge_maps::delete_knowledge_map(conn, id)
        })?;

        if deleted {
            self.events.emit(StorageEvent::KnowledgeMapDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Validate knowledge map input
    fn validate_knowledge_map(&self, input: &knowledge_maps::CreateKnowledgeMapInput) -> Result<(), StorageError> {
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
            return Err(StorageError::InvalidInput("subject_name is required".into()));
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
                "overall_affinity must be between 0.0 and 1.0".into()
            ));
        }

        Ok(())
    }

    // =========================================================================
    // Path Extension Operations
    // =========================================================================

    /// Get path extension by ID
    pub fn get_path_extension(&self, id: &str) -> Result<Option<path_extensions::PathExtensionRow>, StorageError> {
        self.content_db.with_conn(|conn| path_extensions::get_path_extension(conn, id))
    }

    /// List path extensions with filtering
    pub fn list_path_extensions(
        &self,
        query: &path_extensions::PathExtensionQuery,
    ) -> Result<Vec<path_extensions::PathExtensionRow>, StorageError> {
        self.content_db.with_conn(|conn| path_extensions::list_path_extensions(conn, query))
    }

    /// Get path extensions for a base path
    pub fn get_extensions_for_path(
        &self,
        base_path_id: &str,
    ) -> Result<Vec<path_extensions::PathExtensionRow>, StorageError> {
        self.list_path_extensions(&path_extensions::PathExtensionQuery {
            base_path_id: Some(base_path_id.to_string()),
            ..Default::default()
        })
    }

    /// Create a path extension
    pub fn create_path_extension(
        &self,
        input: path_extensions::CreatePathExtensionInput,
    ) -> Result<path_extensions::PathExtensionRow, StorageError> {
        // Validate input
        self.validate_path_extension(&input)?;

        // Create path extension
        let result = self.content_db.with_conn_mut(|conn| {
            path_extensions::create_path_extension(conn, input.clone())
        })?;

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
        input: path_extensions::CreatePathExtensionInput,
    ) -> Result<path_extensions::PathExtensionRow, StorageError> {
        // Validate input
        self.validate_path_extension(&input)?;

        // Check if exists
        if self.get_path_extension(id)?.is_none() {
            return Err(StorageError::NotFound(format!("Path extension '{}' not found", id)));
        }

        // Update
        let result = self.content_db.with_conn_mut(|conn| {
            path_extensions::update_path_extension(conn, id, input)
        })?;

        // Emit event
        self.events.emit(StorageEvent::PathExtensionUpdated { id: id.to_string() });

        Ok(result)
    }

    /// Delete a path extension
    pub fn delete_path_extension(&self, id: &str) -> Result<bool, StorageError> {
        let deleted = self.content_db.with_conn_mut(|conn| {
            path_extensions::delete_path_extension(conn, id)
        })?;

        if deleted {
            self.events.emit(StorageEvent::PathExtensionDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Validate path extension input
    fn validate_path_extension(&self, input: &path_extensions::CreatePathExtensionInput) -> Result<(), StorageError> {
        if input.base_path_id.is_empty() {
            return Err(StorageError::InvalidInput("base_path_id is required".into()));
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
        self.content_db.with_conn(|conn| {
            let total_maps: i64 = conn
                .query_row("SELECT COUNT(*) FROM knowledge_maps", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let total_extensions: i64 = conn
                .query_row("SELECT COUNT(*) FROM path_extensions", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            Ok(KnowledgeStats {
                total_knowledge_maps: total_maps as u64,
                total_path_extensions: total_extensions as u64,
            })
        })
    }
}

/// Knowledge statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeStats {
    pub total_knowledge_maps: u64,
    pub total_path_extensions: u64,
}
