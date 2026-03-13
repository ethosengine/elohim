//! Relationship service - business logic for content graph operations
//!
//! Wraps the relationship repository with validation, event emission,
//! and graph traversal logic.

use std::sync::Arc;

use crate::db::{content_diesel, context::AppContext, relationships_diesel, DbPool};
use crate::error::StorageError;

use super::events::{EventBus, StorageEvent};

/// Relationship service for content graph operations
pub struct RelationshipService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
}

impl RelationshipService {
    /// Create a new relationship service
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

    /// Get relationship by ID
    pub fn get(
        &self,
        id: &str,
    ) -> Result<Option<crate::db::models::Relationship>, StorageError> {
        let mut conn = self.conn()?;
        relationships_diesel::get_relationship(&mut conn, &self.ctx, id)
    }

    /// List relationships with filtering
    pub fn list(
        &self,
        query: &relationships_diesel::RelationshipQuery,
    ) -> Result<Vec<crate::db::models::Relationship>, StorageError> {
        let mut conn = self.conn()?;
        relationships_diesel::list_relationships(&mut conn, &self.ctx, query)
    }

    /// Get relationships for a content item
    pub fn get_for_content(
        &self,
        content_id: &str,
        direction: Option<&str>,
    ) -> Result<Vec<crate::db::models::Relationship>, StorageError> {
        self.list(&relationships_diesel::RelationshipQuery {
            content_id: Some(content_id.to_string()),
            direction: direction.map(|s| s.to_string()),
            ..Default::default()
        })
    }

    /// Get content graph starting from a root node
    ///
    /// Note: The Diesel module doesn't have a graph traversal function.
    /// This returns a flat list of relationships for the content node,
    /// structured as a simple graph with depth=1.
    pub fn get_graph(
        &self,
        content_id: &str,
        relationship_types: Option<&[String]>,
    ) -> Result<ContentGraph, StorageError> {
        let mut conn = self.conn()?;
        let rels = relationships_diesel::get_outgoing_relationships(
            &mut conn,
            &self.ctx,
            content_id,
            relationship_types,
        )?;

        let related: Vec<ContentGraphNode> = rels
            .iter()
            .map(|r| ContentGraphNode {
                content_id: r.target_id.clone(),
                relationship_type: r.relationship_type.clone(),
                confidence: r.confidence as f64,
                children: vec![],
            })
            .collect();

        let total_nodes = related.len();
        Ok(ContentGraph {
            root_id: content_id.to_string(),
            related,
            total_nodes,
        })
    }

    /// Get graph with depth limiting (multi-level traversal)
    pub fn get_graph_with_depth(
        &self,
        content_id: &str,
        max_depth: u32,
        relationship_types: Option<&[String]>,
    ) -> Result<ContentGraph, StorageError> {
        if max_depth == 0 {
            return Ok(ContentGraph {
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
        input: relationships_diesel::CreateRelationshipInput,
    ) -> Result<crate::db::models::Relationship, StorageError> {
        // Validate input
        self.validate_relationship(&input)?;

        // Validate source and target content exist
        self.validate_content_exists(&input.source_id, "source")?;
        self.validate_content_exists(&input.target_id, "target")?;

        // Check for self-referential relationship
        if input.source_id == input.target_id {
            return Err(StorageError::InvalidInput(
                "Cannot create relationship from content to itself".into(),
            ));
        }

        // Check for cycles if this is a hierarchical relationship
        if self.is_hierarchical(&input.relationship_type)
            && self.would_create_cycle(&input.source_id, &input.target_id)?
        {
            return Err(StorageError::InvalidInput(
                "This relationship would create a cycle in the content graph".into(),
            ));
        }

        // Create relationship
        let mut conn = self.conn()?;
        let result =
            relationships_diesel::create_relationship(&mut conn, &self.ctx, input)?;

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
        inputs: Vec<relationships_diesel::CreateRelationshipInput>,
    ) -> Result<relationships_diesel::BulkRelationshipResult, StorageError> {
        // Validate all inputs (skip content existence check for bulk operations)
        for (i, input) in inputs.iter().enumerate() {
            if let Err(e) = self.validate_relationship(input) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        // Perform bulk create
        let mut conn = self.conn()?;
        let result =
            relationships_diesel::bulk_create_relationships(&mut conn, &self.ctx, inputs)?;

        // Emit event
        if result.created > 0 {
            self.events.emit(StorageEvent::RelationshipBulkCreated {
                count: result.created as usize,
            });
        }

        Ok(result)
    }

    /// Delete a relationship by ID
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let mut conn = self.conn()?;
        let deleted = relationships_diesel::delete_relationship(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::RelationshipDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Delete all relationships for a content item
    pub fn delete_for_content(&self, content_id: &str) -> Result<usize, StorageError> {
        let mut conn = self.conn()?;
        relationships_diesel::delete_relationships_for_content(&mut conn, &self.ctx, content_id)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate relationship input
    fn validate_relationship(
        &self,
        input: &relationships_diesel::CreateRelationshipInput,
    ) -> Result<(), StorageError> {
        if input.source_id.is_empty() {
            return Err(StorageError::InvalidInput("source_id is required".into()));
        }

        if input.target_id.is_empty() {
            return Err(StorageError::InvalidInput("target_id is required".into()));
        }

        if input.relationship_type.is_empty() {
            return Err(StorageError::InvalidInput(
                "relationship_type is required".into(),
            ));
        }

        // Validate relationship_type
        let valid_types = [
            "RELATES_TO",
            "CONTAINS",
            "DEPENDS_ON",
            "IMPLEMENTS",
            "REFERENCES",
            "PREREQUISITE",
            "FOLLOWUP",
            "SIBLING",
            "PARENT",
            "CHILD",
            "SIMILAR_TO",
            "CONTRASTS_WITH",
            "ELABORATES",
            "SUMMARIZES",
            "EXAMPLE_OF",
            "DEFINITION_OF",
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
                "confidence must be between 0.0 and 1.0".into(),
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
        let mut conn = self.conn()?;
        let exists = content_diesel::get_content(&mut conn, &self.ctx, id)?.is_some();

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
        matches!(
            rel_type,
            "CONTAINS" | "PARENT" | "CHILD" | "DEPENDS_ON" | "PREREQUISITE"
        )
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
        let mut conn = self.conn()?;
        let total = relationships_diesel::relationship_count(&mut conn, &self.ctx)? as u64;
        let by_type_vec = relationships_diesel::relationship_stats_by_type(&mut conn, &self.ctx)?;

        Ok(RelationshipStats {
            total_count: total,
            by_type: by_type_vec.into_iter().collect(),
        })
    }
}

/// Content graph node for tree traversal
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContentGraphNode {
    pub content_id: String,
    pub relationship_type: String,
    pub confidence: f64,
    pub children: Vec<ContentGraphNode>,
}

/// Content graph output
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContentGraph {
    pub root_id: String,
    pub related: Vec<ContentGraphNode>,
    pub total_nodes: usize,
}

/// Relationship statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationshipStats {
    pub total_count: u64,
    pub by_type: std::collections::HashMap<String, i64>,
}
