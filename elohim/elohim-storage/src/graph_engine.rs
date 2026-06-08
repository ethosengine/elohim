//! The native content-graph seam. The ONE place the content graph is realized.
//! Composes explicit (notarized, Category A) + computed (recompute-on-read,
//! Category C) edges. Read-only: this trait has no write method by design —
//! a computed edge can never be persisted through it.

use std::collections::{HashSet, VecDeque};

use crate::db::context::AppContext;
use crate::db::{relationships_diesel, DbPool, PooledConn};
use crate::error::StorageError;

/// One edge in a resolved neighborhood, discriminated by `inference_source`.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEdge {
    pub target_id: String,
    pub relationship_type: String, // RELATES_TO for both classes in this slice
    pub confidence: f64,
    pub inference_source: String, // "explicit" (A) | "tag" (C). Never persisted for C.
    pub depth: u32,               // 1 = direct; >1 = transitively-reached explicit edge
}

/// A resolved neighborhood rooted at one node.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedNeighborhood {
    pub root_id: String,
    pub edges: Vec<ResolvedEdge>,
}

/// Bounded knobs for one resolution.
#[derive(Debug, Clone)]
pub struct GraphQuery<'a> {
    pub root_id: &'a str,
    pub max_depth: u32,
    pub relationship_types: Option<&'a [String]>,
    pub include_computed: bool,
    pub max_computed: usize,
    pub min_shared_tags: usize,
}

impl<'a> GraphQuery<'a> {
    /// Defaults: depth 2 (caller hard-caps at 3), computed on, 25 cap, 1 shared tag.
    pub fn new(root_id: &'a str) -> Self {
        Self {
            root_id,
            max_depth: 2,
            relationship_types: None,
            include_computed: true,
            max_computed: 25,
            min_shared_tags: 1,
        }
    }
}

/// The seam. A future Cozo/datalog/embedding engine is just another impl.
pub trait ContentGraphResolver: Send + Sync {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError>;
}

/// The Diesel-backed resolver. Holds the pool exactly like `RelationshipService`
/// (`pool: DbPool`); `AppContext` is the per-call tenant scope, forwarded into the
/// diesel functions — it is NOT a connection source.
///
/// Pass 1 (this slice): explicit edges via depth-bounded BFS over stored
/// relationships. Pass 2 (tag-discovery, Category C) is added in A4 — that is why
/// `include_computed` / `max_computed` / `min_shared_tags` on the query are read
/// but not yet acted upon here.
pub struct NativeGraphResolver {
    pool: DbPool,
}

impl NativeGraphResolver {
    /// Build a resolver over a connection pool (mirrors `RelationshipService::new`).
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }

    /// Get a connection from the pool (mirrors `RelationshipService::conn`).
    fn conn(&self) -> Result<PooledConn, StorageError> {
        self.pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))
    }

    /// Pass 1: explicit edges via depth-bounded BFS.
    ///
    /// Walks stored relationships breadth-first from `root_id`, emitting one
    /// `ResolvedEdge` per first-discovery of a target (cycle-safe via a visited
    /// set; the root is pre-seeded so a self-cycle never re-emits it). Depth is
    /// hard-capped at 3 regardless of `query.max_depth`.
    fn explicit_edges(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<Vec<ResolvedEdge>, StorageError> {
        // Connection is held across the BFS; safe because depth is hard-capped at 3.
        let mut conn = self.conn()?;
        let depth_cap = query.max_depth.min(3); // hard cap — bounded traversal

        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(query.root_id.to_string());

        let mut out: Vec<ResolvedEdge> = Vec::new();
        let mut frontier: VecDeque<(String, u32)> = VecDeque::new();
        frontier.push_back((query.root_id.to_string(), 0));

        while let Some((node, depth)) = frontier.pop_front() {
            if depth >= depth_cap {
                continue;
            }
            let rels = relationships_diesel::get_outgoing_relationships(
                &mut conn,
                ctx,
                &node,
                query.relationship_types,
            )?;
            for r in rels {
                if visited.insert(r.target_id.clone()) {
                    let edge_depth = depth + 1;
                    out.push(ResolvedEdge {
                        target_id: r.target_id.clone(),
                        relationship_type: r.relationship_type,
                        confidence: r.confidence as f64,
                        inference_source: if r.inference_source.is_empty() {
                            "explicit".to_string()
                        } else {
                            r.inference_source
                        },
                        depth: edge_depth,
                    });
                    frontier.push_back((r.target_id, edge_depth));
                }
            }
        }

        Ok(out)
    }
}

impl ContentGraphResolver for NativeGraphResolver {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError> {
        // Pass 1: explicit edges. Pass 2 (computed/tag) is wired in A4.
        let edges = self.explicit_edges(ctx, query)?;
        Ok(ResolvedNeighborhood {
            root_id: query.root_id.to_string(),
            edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::relationships_diesel::test_harness::{insert_relationship, test_pool_ctx};

    #[test]
    fn explicit_bfs_reaches_depth_two() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        // A -> B -> C, all explicit RELATES_TO edges.
        {
            let mut conn = pool.get().expect("conn");
            insert_relationship(&mut conn, &ctx, "A", "B", "RELATES_TO", "explicit");
            insert_relationship(&mut conn, &ctx, "B", "C", "RELATES_TO", "explicit");
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: false,
            max_depth: 2,
            ..GraphQuery::new("A")
        };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        let by: std::collections::BTreeSet<_> = n
            .edges
            .iter()
            .map(|e| (e.target_id.as_str(), e.depth))
            .collect();
        assert!(by.contains(&("B", 1)), "B at depth 1");
        assert!(by.contains(&("C", 2)), "C at depth 2 (de-stubbed)");
        assert!(n.edges.iter().all(|e| e.inference_source == "explicit"));
    }

    /// BFS must terminate on cycles and never double-emit any node.
    /// A→B and B→A form a cycle. With max_depth 2 and root A pre-seeded in
    /// visited, the back-edge B→A is suppressed; only B is emitted (at depth 1).
    #[test]
    fn bfs_cycle_does_not_loop() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        {
            let mut conn = pool.get().expect("conn");
            insert_relationship(&mut conn, &ctx, "A", "B", "RELATES_TO", "explicit");
            insert_relationship(&mut conn, &ctx, "B", "A", "RELATES_TO", "explicit");
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: false,
            max_depth: 2,
            ..GraphQuery::new("A")
        };
        // Must terminate (no infinite loop).
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        // B is reachable at depth 1.
        assert!(
            n.edges.iter().any(|e| e.target_id == "B" && e.depth == 1),
            "B should be emitted at depth 1"
        );
        // Root A must never appear as a target (pre-seeded in visited).
        assert!(
            !n.edges.iter().any(|e| e.target_id == "A"),
            "root A must not appear as a target"
        );
        // No node is double-emitted.
        let targets: Vec<_> = n.edges.iter().map(|e| &e.target_id).collect();
        let unique: std::collections::HashSet<_> = targets.iter().collect();
        assert_eq!(targets.len(), unique.len(), "no double-emitted targets");
    }

    /// Diamond topology: A→B, A→C, B→D, C→D. D must appear exactly once
    /// (the visited-set dedup across the two paths to D).
    #[test]
    fn diamond_emits_target_once() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        {
            let mut conn = pool.get().expect("conn");
            insert_relationship(&mut conn, &ctx, "A", "B", "RELATES_TO", "explicit");
            insert_relationship(&mut conn, &ctx, "A", "C", "RELATES_TO", "explicit");
            insert_relationship(&mut conn, &ctx, "B", "D", "RELATES_TO", "explicit");
            insert_relationship(&mut conn, &ctx, "C", "D", "RELATES_TO", "explicit");
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: false,
            max_depth: 2,
            ..GraphQuery::new("A")
        };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        let d_edges: Vec<_> = n.edges.iter().filter(|e| e.target_id == "D").collect();
        assert_eq!(
            d_edges.len(),
            1,
            "D must appear exactly once (visited-set dedup)"
        );
        assert_eq!(d_edges[0].depth, 2, "D is at depth 2");
    }
}
