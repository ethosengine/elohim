//! Relationship service - business logic for content graph operations
//!
//! Wraps the relationship repository with validation, event emission,
//! and graph traversal logic.

use std::sync::Arc;

use crate::db::{content_diesel, context::AppContext, relationships_diesel, DbPool};
use crate::error::StorageError;
use crate::views::ContentGraphView;

use super::events::{EventBus, StorageEvent};

/// Relationship service for content graph operations
pub struct RelationshipService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
    /// The content-graph seam. Diesel-backed today (`NativeGraphResolver`); a
    /// future Cozo/datalog/embedding engine is just another `dyn` impl. Holding
    /// it behind the trait keeps `get_graph*` transport-/engine-neutral.
    resolver: Arc<dyn crate::graph_engine::ContentGraphResolver>,
}

impl RelationshipService {
    /// Create a new relationship service
    pub fn new(pool: DbPool, ctx: AppContext, events: Arc<EventBus>) -> Self {
        let resolver: Arc<dyn crate::graph_engine::ContentGraphResolver> =
            Arc::new(crate::graph_engine::NativeGraphResolver::new(pool.clone()));
        Self {
            pool,
            ctx,
            events,
            resolver,
        }
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
    pub fn get(&self, id: &str) -> Result<Option<crate::db::models::Relationship>, StorageError> {
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

    /// Get content graph starting from a root node (direct neighbors only).
    ///
    /// Delegates to the `ContentGraphResolver` seam: a flat list of depth-1
    /// explicit neighbors, projected to the provenance-honest `ContentGraphView`
    /// (each node carries its own `inferenceSource` + `depth`). Computed/tag
    /// discovery is OFF here — see `get_graph_query` for the richer entrypoint.
    pub fn get_graph(
        &self,
        content_id: &str,
        relationship_types: Option<&[String]>,
    ) -> Result<ContentGraphView, StorageError> {
        let q = crate::graph_engine::GraphQuery {
            root_id: content_id,
            max_depth: 1,
            relationship_types,
            include_computed: false,
            max_computed: 25,
            min_shared_tags: 1,
        };
        Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
    }

    /// Get graph with depth limiting (multi-level explicit traversal).
    ///
    /// Depth-bounded BFS over stored relationships (the resolver hard-caps depth
    /// at 3 internally). Computed/tag discovery is OFF; use `get_graph_query` to
    /// opt into it.
    ///
    /// Passing `max_depth = 0` returns an empty graph (root only, no traversal).
    pub fn get_graph_with_depth(
        &self,
        content_id: &str,
        max_depth: u32,
        relationship_types: Option<&[String]>,
    ) -> Result<ContentGraphView, StorageError> {
        let q = crate::graph_engine::GraphQuery {
            root_id: content_id,
            max_depth,
            relationship_types,
            include_computed: false,
            max_computed: 25,
            min_shared_tags: 1,
        };
        Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
    }

    /// Richer content-graph entrypoint: explicit BFS + optional tag-discovery.
    ///
    /// The HTTP route (`GET /db/relationships/graph/{id}`) delegates here. All
    /// caps are the caller's responsibility to clamp BEFORE calling — the route
    /// bounds `depth`, `max_computed`, and `min_shared_tags` so an attacker can
    /// never pass a huge/negative cap into the SQL `LIMIT`.
    pub fn get_graph_query(
        &self,
        content_id: &str,
        depth: u32,
        include_computed: bool,
        min_shared_tags: usize,
        max_computed: usize,
        relationship_types: Option<&[String]>,
    ) -> Result<ContentGraphView, StorageError> {
        debug_assert!(
            depth <= 3 && max_computed <= 100 && min_shared_tags >= 1,
            "get_graph_query expects clamped params (depth<=3, max_computed<=100, min_shared_tags>=1)"
        );
        let q = crate::graph_engine::GraphQuery {
            root_id: content_id,
            max_depth: depth,
            relationship_types,
            include_computed,
            max_computed,
            min_shared_tags,
        };
        Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
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
        let result = relationships_diesel::create_relationship(&mut conn, &self.ctx, input)?;

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
        let result = relationships_diesel::bulk_create_relationships(&mut conn, &self.ctx, inputs)?;

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
        // Internal existence check for relationship validation; pre-drain rows
        // must still count as existing, so provenance gate is off.
        let exists = content_diesel::get_content(&mut conn, &self.ctx, id, false)?.is_some();

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

/// Relationship statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct RelationshipStats {
    pub total_count: u64,
    pub by_type: std::collections::HashMap<String, i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::relationships_diesel::test_harness::{
        insert_content_with_tags, insert_relationship, test_pool_ctx,
    };

    /// End-to-end composition through the service seam: `get_graph_query` must
    /// fold BOTH provenance classes into one `ContentGraphView` —
    /// - Z via an authored X→Z edge (`inferenceSource == "explicit"`), and
    /// - Y via tag overlap with X, no authored edge (`inferenceSource == "tag"`).
    /// This proves `RelationshipService` delegates to the resolver and returns
    /// the promoted ts-rs view, not the retired plain-serde struct.
    #[test]
    fn get_graph_query_folds_explicit_and_tag_provenance() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        {
            let mut conn = pool.get().expect("conn");
            // X shares tags with Y (no authored edge -> tag discovery).
            insert_content_with_tags(&mut conn, &ctx, "X", &["grace", "sin"]);
            insert_content_with_tags(&mut conn, &ctx, "Y", &["grace", "sin"]);
            // X has an authored edge to Z (explicit).
            insert_relationship(&mut conn, &ctx, "X", "Z", "RELATES_TO", "explicit");
        }

        let svc = RelationshipService::new(pool, ctx, Arc::new(EventBus::new()));
        let graph = svc
            .get_graph_query("X", 2, true, 1, 25, None)
            .expect("resolve neighborhood");

        assert_eq!(graph.root_id, "X");

        let z = graph
            .related
            .iter()
            .find(|n| n.content_id == "Z")
            .expect("Z reached via explicit edge");
        assert_eq!(z.inference_source, "explicit");
        assert_eq!(z.depth, 1);

        let y = graph
            .related
            .iter()
            .find(|n| n.content_id == "Y")
            .expect("Y discovered via tag overlap");
        assert_eq!(y.inference_source, "tag");
        assert_eq!(y.depth, 1);

        // Flat read: total_nodes is the neighbour count, children stays empty.
        assert_eq!(graph.total_nodes, graph.related.len());
        assert!(graph.related.iter().all(|n| n.children.is_empty()));
    }
}
