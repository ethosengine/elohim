//! Relationship service - business logic for content graph operations
//!
//! Wraps the relationship repository with validation, event emission,
//! and graph traversal logic.

use std::sync::Arc;

use crate::db::{content_diesel, context::AppContext, relationships_diesel, DbPool};
use crate::error::StorageError;
use crate::views::ContentGraphView;

use super::events::{EventBus, StorageEvent};

/// Relationship types this service accepts.
///
/// The lamad manifest (`elohim/sdk/domains/lamad/manifest/relationships.json`)
/// is the SOURCE OF TRUTH for this vocabulary — the seeder's extractor emits
/// exactly its ids (generated `manifest-types.ts`). This list is the manifest's
/// ids (twelve as of the `STEP` addition, see
/// prologue-seed-step-relationship-type-invalid.md) PLUS the legacy ids rows
/// already carry (kept so stored rows stay readable); this list is
/// HAND-SYNCED to the manifest, not generated — the
/// `manifest_relationship_vocabulary_is_accepted` test pins
/// "manifest ⊆ accepted" so the two cannot drift apart silently again — until
/// 2026-08-28 they had (5 of 11 in common) and every local `seed apply`
/// dropped its whole relationship graph on `HTTP 400 relationship_type
/// 'EXTENDS' is not valid`.
pub const VALID_RELATIONSHIP_TYPES: &[&str] = &[
    // manifest/relationships.json (canonical)
    "CONTAINS",
    "BELONGS_TO",
    "DESCRIBES",
    "IMPLEMENTS",
    "VALIDATES",
    "RELATES_TO",
    "REFERENCES",
    "DEPENDS_ON",
    "REQUIRES",
    "FOLLOWS",
    "ATTACHED_TO",
    "STEP",
    // legacy ids already persisted by earlier seeders / migrations
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

/// The reach an edge whose SOURCE atom has no local row is written at: the
/// most restrictive tier (the read gate serves it to nobody anonymous — and,
/// with the source unknown, the `self` authorizer finds no creator to admit).
/// A fabricated edge from an id this peer cannot check never serves openly.
pub const UNKNOWN_SOURCE_EDGE_REACH: &str = "self";

/// The narrower (more restrictive) of two recognized reach tiers; on a tie,
/// `a`.
fn narrower_reach<'a>(a: &'a str, b: &'a str) -> &'a str {
    if crate::epr_service::reach_level_index(b) > crate::epr_service::reach_level_index(a) {
        b
    } else {
        a
    }
}

/// The reach a write route stores an edge at — derived server-side, never
/// taken from the caller (invariant: an edge is never more open than the atom
/// that authored it, `db::authored_edges`).
///
/// - Source row held: the SOURCE's reach, or the caller's stated reach when
///   that is NARROWER (a caller may narrow an edge, never widen it).
/// - Source row absent: [`UNKNOWN_SOURCE_EDGE_REACH`], whatever was stated.
/// - A stated reach that is not a recognized tier is refused.
pub(crate) fn written_edge_reach(
    conn: &mut diesel::SqliteConnection,
    ctx: &AppContext,
    input: &relationships_diesel::CreateRelationshipInput,
) -> Result<String, StorageError> {
    let stated = input.reach.as_deref().map(str::trim);
    if let Some(stated) = stated {
        if crate::epr_service::recognized_reach_level_index(stated).is_none() {
            return Err(StorageError::InvalidInput(format!(
                "reach '{stated}' is not a recognized tier"
            )));
        }
    }
    let Some(source_reach) = content_diesel::reach_for(conn, ctx, &input.source_id)? else {
        return Ok(UNKNOWN_SOURCE_EDGE_REACH.to_string());
    };
    Ok(match stated {
        Some(stated) => narrower_reach(&source_reach, stated).to_string(),
        None => source_reach,
    })
}

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
            admission: None,
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
            admission: None,
        };
        Ok(self.resolver.resolve_neighborhood(&self.ctx, &q)?.into())
    }

    /// Richer content-graph entrypoint: explicit BFS + optional tag-discovery.
    ///
    /// The HTTP route (`GET /db/relationships/graph/{id}`) delegates here. All
    /// caps are the caller's responsibility to clamp BEFORE calling — the route
    /// bounds `depth`, `max_computed`, and `min_shared_tags` so an attacker can
    /// never pass a huge/negative cap into the SQL `LIMIT`. The route also
    /// supplies `query.admission` (the caller's reach filter), so the walk
    /// prunes nodes the caller may not read.
    pub fn get_graph_query(
        &self,
        query: &crate::graph_engine::GraphQuery<'_>,
    ) -> Result<ContentGraphView, StorageError> {
        debug_assert!(
            query.max_depth <= 3 && query.max_computed <= 100 && query.min_shared_tags >= 1,
            "get_graph_query expects clamped params (depth<=3, max_computed<=100, min_shared_tags>=1)"
        );
        Ok(self.resolver.resolve_neighborhood(&self.ctx, query)?.into())
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

        // Create relationship at the reach its SOURCE row allows.
        let mut conn = self.conn()?;
        let mut input = input;
        input.reach = Some(written_edge_reach(&mut conn, &self.ctx, &input)?);
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

        // Perform bulk create, each edge at the reach its SOURCE row allows.
        let mut conn = self.conn()?;
        let mut inputs = inputs;
        for (i, input) in inputs.iter_mut().enumerate() {
            input.reach = Some(
                written_edge_reach(&mut conn, &self.ctx, input)
                    .map_err(|e| StorageError::InvalidInput(format!("item[{}]: {}", i, e)))?,
            );
        }
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
        if !VALID_RELATIONSHIP_TYPES.contains(&input.relationship_type.as_str()) {
            return Err(StorageError::InvalidInput(format!(
                "relationship_type '{}' is not valid. Valid types: {:?}",
                input.relationship_type, VALID_RELATIONSHIP_TYPES
            )));
        }

        // Validate confidence range
        if input.confidence < 0.0 || input.confidence > 1.0 {
            return Err(StorageError::InvalidInput(
                "confidence must be between 0.0 and 1.0".into(),
            ));
        }

        // The verified-projection marker is written by `authored_edges` alone;
        // a request that claims it is claiming a signed head it does not carry.
        if crate::db::authored_edges::is_authored_projection(input.provenance_chain_json.as_deref())
        {
            return Err(StorageError::InvalidInput(
                "provenance_chain_json names the verified-projection marker, which only a \
                 conductor-verified source head can write"
                    .into(),
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
        let exists = content_diesel::get_content(
            &mut conn,
            &self.ctx,
            id,
            content_diesel::MinTrust::Invisible,
        )?
        .is_some();

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
    ///
    /// - Z via an authored X→Z edge (`inferenceSource == "explicit"`), and
    /// - Y via tag overlap with X, no authored edge (`inferenceSource == "tag"`).
    ///
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
            .get_graph_query(&crate::graph_engine::GraphQuery {
                max_depth: 2,
                include_computed: true,
                min_shared_tags: 1,
                max_computed: 25,
                ..crate::graph_engine::GraphQuery::new("X")
            })
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

#[cfg(test)]
mod relationship_vocabulary_tests {
    use super::VALID_RELATIONSHIP_TYPES;

    /// The lamad manifest is the source of truth for relationship ids; this
    /// service must accept every id it declares. Reads the manifest at compile
    /// time so a new manifest id without a matching entry here fails the build's
    /// tests instead of failing the next `seed apply` with an HTTP 400.
    #[test]
    fn manifest_relationship_vocabulary_is_accepted() {
        let manifest = include_str!("../../../sdk/domains/lamad/manifest/relationships.json");
        let parsed: serde_json::Value =
            serde_json::from_str(manifest).expect("relationships.json parses");
        let ids: Vec<&str> = parsed
            .as_object()
            .expect("relationships.json is an object keyed by relationship id")
            .keys()
            .map(String::as_str)
            .collect();
        assert!(ids.len() >= 11, "manifest lost relationship ids: {ids:?}");
        let missing: Vec<&&str> = ids
            .iter()
            .filter(|id| !VALID_RELATIONSHIP_TYPES.contains(id))
            .collect();
        assert!(
            missing.is_empty(),
            "manifest relationship ids not accepted by relationship_service: {missing:?}"
        );
    }
}

/// The write routes' edge-reach derivation (an edge is never more open than
/// its source atom).
#[cfg(test)]
mod written_edge_reach_tests {
    use super::*;

    fn edge_input(
        source: &str,
        target: &str,
        reach: Option<&str>,
    ) -> relationships_diesel::CreateRelationshipInput {
        relationships_diesel::CreateRelationshipInput {
            id: None,
            source_id: source.into(),
            target_id: target.into(),
            relationship_type: "RELATES_TO".into(),
            confidence: 1.0,
            inference_source: "explicit".into(),
            is_bidirectional: false,
            provenance_chain_json: None,
            governance_layer: None,
            reach: reach.map(str::to_owned),
            metadata_json: None,
        }
    }

    fn seed_atom(conn: &mut diesel::SqliteConnection, ctx: &AppContext, id: &str, reach: &str) {
        content_diesel::create_content(
            conn,
            ctx,
            content_diesel::CreateContentInput {
                id: id.into(),
                title: id.into(),
                description: None,
                content_type: "concept".into(),
                content_format: "markdown".into(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.into(),
                created_by: None,
                tags: Vec::new(),
                content_body: Some("body".into()),
                dht_anchor_hash: None,
            },
        )
        .unwrap();
    }

    /// The write routes derive an edge's reach from its SOURCE row: a stated
    /// reach may only narrow it, an unknown source writes the most restrictive
    /// tier, an unrecognized tier is refused.
    #[test]
    fn a_written_edge_is_never_more_open_than_its_source() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let mut conn = pool.get().unwrap();
        seed_atom(&mut conn, &ctx, "open", "commons");
        seed_atom(&mut conn, &ctx, "closed", "intimate");
        let reach = |conn: &mut diesel::SqliteConnection, s: &str, r: Option<&str>| {
            written_edge_reach(conn, &ctx, &edge_input(s, "t", r))
        };

        assert_eq!(reach(&mut conn, "open", None).unwrap(), "commons");
        assert_eq!(
            reach(&mut conn, "open", Some("community")).unwrap(),
            "community",
            "narrowing is the caller's to ask"
        );
        assert_eq!(
            reach(&mut conn, "closed", Some("commons")).unwrap(),
            "intimate",
            "widening is refused by taking the source's tier"
        );
        assert_eq!(reach(&mut conn, "closed", None).unwrap(), "intimate");
        assert_eq!(
            reach(&mut conn, "unknown-source", Some("commons")).unwrap(),
            UNKNOWN_SOURCE_EDGE_REACH
        );
        assert!(reach(&mut conn, "open", Some("wide-open")).is_err());
    }

    /// Through the service the POST routes call: bulk rows land at the derived
    /// reach (including an upsert of an existing row), and a request claiming
    /// the verified-projection marker is refused whole.
    #[test]
    fn bulk_create_writes_the_derived_reach_and_refuses_the_marker() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        {
            let mut conn = pool.get().unwrap();
            seed_atom(&mut conn, &ctx, "closed", "intimate");
            seed_atom(&mut conn, &ctx, "open", "commons");
        }
        let svc = RelationshipService::new(pool.clone(), ctx.clone(), Arc::new(EventBus::new()));
        svc.bulk_create(vec![
            edge_input("closed", "open", Some("commons")),
            edge_input("ghost", "open", Some("commons")),
        ])
        .unwrap();
        // Re-POST the same triple asking for commons: still intimate.
        svc.bulk_create(vec![edge_input("closed", "open", Some("public"))])
            .unwrap();
        let edges = svc
            .list(&relationships_diesel::RelationshipQuery {
                limit: 100,
                ..Default::default()
            })
            .unwrap();
        let reach_of = |s: &str| {
            edges
                .iter()
                .find(|e| e.source_id == s)
                .unwrap()
                .reach
                .clone()
        };
        assert_eq!(reach_of("closed"), "intimate");
        assert_eq!(reach_of("ghost"), UNKNOWN_SOURCE_EDGE_REACH);

        let mut forged = edge_input("open", "closed", None);
        forged.provenance_chain_json =
            Some(crate::db::authored_edges::AUTHORED_PROVENANCE_JSON.to_string());
        assert!(svc.bulk_create(vec![forged]).is_err());
    }
}
