//! Relationship service - business logic for content graph operations
//!
//! Wraps the relationship repository with validation, event emission,
//! and graph traversal logic.

use std::sync::Arc;

use crate::db::{relationships, content, ContentDb};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Relationship service for content graph operations
pub struct RelationshipService {
    content_db: Arc<ContentDb>,
    events: Arc<EventBus>,
}

impl RelationshipService {
    /// Create a new relationship service
    pub fn new(content_db: Arc<ContentDb>, events: Arc<EventBus>) -> Self {
        Self { content_db, events }
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get relationship by ID
    pub fn get(&self, id: &str) -> Result<Option<relationships::RelationshipRow>, StorageError> {
        self.content_db.with_conn(|conn| relationships::get_relationship(conn, id))
    }

    /// List relationships with filtering
    pub fn list(&self, query: &relationships::RelationshipQuery) -> Result<Vec<relationships::RelationshipRow>, StorageError> {
        self.content_db.with_conn(|conn| relationships::list_relationships(conn, query))
    }

    /// Get relationships for a content item
    pub fn get_for_content(
        &self,
        content_id: &str,
        direction: Option<&str>,
    ) -> Result<Vec<relationships::RelationshipRow>, StorageError> {
        self.list(&relationships::RelationshipQuery {
            content_id: Some(content_id.to_string()),
            direction: direction.map(|s| s.to_string()),
            ..Default::default()
        })
    }

    /// Get content graph starting from a root node
    pub fn get_graph(
        &self,
        content_id: &str,
        relationship_types: Option<&[String]>,
    ) -> Result<relationships::ContentGraph, StorageError> {
        self.content_db.with_conn(|conn| {
            relationships::get_content_graph(conn, content_id, relationship_types)
        })
    }

    /// Get graph with depth limiting (multi-level traversal)
    pub fn get_graph_with_depth(
        &self,
        content_id: &str,
        max_depth: u32,
        relationship_types: Option<&[String]>,
    ) -> Result<relationships::ContentGraph, StorageError> {
        if max_depth == 0 {
            return Ok(relationships::ContentGraph {
                root_id: content_id.to_string(),
                related: vec![],
                total_nodes: 0,
            });
        }

        // For now, just return depth=1 graph
        // TODO: Implement recursive traversal with visited set to prevent cycles
        self.get_graph(content_id, relationship_types)
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a relationship with validation
    pub fn create(
        &self,
        input: relationships::CreateRelationshipInput,
    ) -> Result<relationships::RelationshipRow, StorageError> {
        // Validate input
        self.validate_relationship(&input)?;

        // Validate source and target content exist
        self.validate_content_exists(&input.source_id, "source")?;
        self.validate_content_exists(&input.target_id, "target")?;

        // Check for self-referential relationship
        if input.source_id == input.target_id {
            return Err(StorageError::InvalidInput(
                "Cannot create relationship from content to itself".into()
            ));
        }

        // Check for cycles if this is a hierarchical relationship
        if self.is_hierarchical(&input.relationship_type) {
            if self.would_create_cycle(&input.source_id, &input.target_id)? {
                return Err(StorageError::InvalidInput(
                    "This relationship would create a cycle in the content graph".into()
                ));
            }
        }

        // Create relationship
        let result = self.content_db.with_conn_mut(|conn| {
            relationships::create_relationship(conn, input.clone())
        })?;

        // Emit event
        self.events.emit(StorageEvent::RelationshipCreated {
            id: result.id.clone(),
            source_id: result.source_id.clone(),
            target_id: result.target_id.clone(),
            relationship_type: result.relationship_type.clone(),
        });

        Ok(result)
    }

    /// Bulk create relationships (for seeding/import)
    pub fn bulk_create(
        &self,
        inputs: Vec<relationships::CreateRelationshipInput>,
    ) -> Result<relationships::BulkRelationshipResult, StorageError> {
        // Validate all inputs (skip content existence check for bulk operations)
        for (i, input) in inputs.iter().enumerate() {
            if let Err(e) = self.validate_relationship(input) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        // Perform bulk create
        let result = self.content_db.with_conn_mut(|conn| {
            relationships::bulk_create_relationships(conn, inputs)
        })?;

        // Emit event
        if result.created > 0 {
            self.events.emit(StorageEvent::RelationshipBulkCreated {
                count: result.created,
            });
        }

        Ok(result)
    }

    /// Delete a relationship by ID
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let deleted = self.content_db.with_conn_mut(|conn| {
            relationships::delete_relationship(conn, id)
        })?;

        if deleted {
            self.events.emit(StorageEvent::RelationshipDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Delete all relationships for a content item
    pub fn delete_for_content(&self, content_id: &str) -> Result<usize, StorageError> {
        self.content_db.with_conn_mut(|conn| {
            relationships::delete_relationships_for_content(conn, content_id)
        })
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate relationship input
    fn validate_relationship(&self, input: &relationships::CreateRelationshipInput) -> Result<(), StorageError> {
        if input.source_id.is_empty() {
            return Err(StorageError::InvalidInput("source_id is required".into()));
        }

        if input.target_id.is_empty() {
            return Err(StorageError::InvalidInput("target_id is required".into()));
        }

        if input.relationship_type.is_empty() {
            return Err(StorageError::InvalidInput("relationship_type is required".into()));
        }

        // Validate relationship_type
        let valid_types = [
            "RELATES_TO", "CONTAINS", "DEPENDS_ON", "IMPLEMENTS", "REFERENCES",
            "PREREQUISITE", "FOLLOWUP", "SIBLING", "PARENT", "CHILD",
            "SIMILAR_TO", "CONTRASTS_WITH", "ELABORATES", "SUMMARIZES",
            "EXAMPLE_OF", "DEFINITION_OF",
        ];
        if !valid_types.contains(&input.relationship_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "relationship_type '{}' is not valid. Valid types: {:?}",
                input.relationship_type, valid_types
            )));
        }

        // Validate confidence range
        if input.confidence < 0.0 || input.confidence > 1.0 {
            return Err(StorageError::InvalidInput(
                "confidence must be between 0.0 and 1.0".into()
            ));
        }

        // Validate inference_source
        let valid_sources = ["explicit", "path", "tag", "semantic", "system"];
        if !valid_sources.contains(&input.inference_source.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "inference_source '{}' is not valid. Valid sources: {:?}",
                input.inference_source, valid_sources
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

    /// Validate that content exists
    fn validate_content_exists(&self, id: &str, field_name: &str) -> Result<(), StorageError> {
        let exists = self.content_db.with_conn(|conn| {
            content::get_content(conn, id)
        })?.is_some();

        if !exists {
            return Err(StorageError::InvalidInput(format!(
                "{} content '{}' does not exist",
                field_name, id
            )));
        }

        Ok(())
    }

    /// Check if a relationship type is hierarchical (could form cycles)
    fn is_hierarchical(&self, rel_type: &str) -> bool {
        matches!(rel_type, "CONTAINS" | "PARENT" | "CHILD" | "DEPENDS_ON" | "PREREQUISITE")
    }

    /// Check if creating this relationship would create a cycle
    fn would_create_cycle(&self, source_id: &str, target_id: &str) -> Result<bool, StorageError> {
        // Simple check: see if target already has a path back to source
        // This is a basic DFS/BFS - for large graphs, consider a more efficient algorithm
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![target_id.to_string()];

        while let Some(current) = stack.pop() {
            if current == source_id {
                return Ok(true); // Found a path back to source = cycle
            }

            if visited.contains(&current) {
                continue;
            }
            visited.insert(current.clone());

            // Get outgoing relationships from current node
            let relations = self.get_for_content(&current, Some("outgoing"))?;
            for rel in relations {
                if self.is_hierarchical(&rel.relationship_type) {
                    stack.push(rel.target_id);
                }
            }
        }

        Ok(false)
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get relationship statistics
    pub fn get_stats(&self) -> Result<RelationshipStats, StorageError> {
        self.content_db.with_conn(|conn| {
            let total: i64 = conn
                .query_row("SELECT COUNT(*) FROM relationships", [], |row| row.get(0))
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let by_type: Vec<(String, i64)> = {
                let mut stmt = conn
                    .prepare("SELECT relationship_type, COUNT(*) FROM relationships GROUP BY relationship_type")
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                let rows = stmt
                    .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
                    .map_err(|e| StorageError::Internal(e.to_string()))?;
                rows.collect::<Result<Vec<_>, _>>()
                    .map_err(|e| StorageError::Internal(e.to_string()))?
            };

            Ok(RelationshipStats {
                total_count: total as u64,
                by_type: by_type.into_iter().collect(),
            })
        })
    }
}

/// Relationship statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationshipStats {
    pub total_count: u64,
    pub by_type: std::collections::HashMap<String, i64>,
}
