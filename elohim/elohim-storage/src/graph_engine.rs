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

    /// Pass 2: tag co-occurrence discovery (Category C).
    ///
    /// Surfaces content nodes that share at least `min_shared_tags` tags with the
    /// root but have NO authored edge to it — the edges that make this a discovery
    /// surface rather than a replay of hand-authored links. These edges are
    /// **recomputed on every read and NEVER persisted**: no `create_relationship`,
    /// no write, no `dht_anchor_hash`. `exclude` carries the explicit targets (plus
    /// the root) so an explicitly-reached node is never also emitted as a tag edge
    /// (explicit precedence). Capped at `max_computed`, depth fixed at 1.
    fn computed_tag_edges(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
        exclude: &HashSet<String>,
    ) -> Result<Vec<ResolvedEdge>, StorageError> {
        use diesel::prelude::*;
        use diesel::sql_types::{BigInt, Text};

        let mut conn = self.conn()?;
        let app = &ctx.h_app_id;

        // Root tag count — the confidence denominator. Clean COUNT query.
        #[derive(diesel::QueryableByName)]
        struct RootTagCount {
            #[diesel(sql_type = BigInt)]
            n: i64,
        }
        let root_tag_count: i64 = diesel::sql_query(
            "SELECT COUNT(*) AS n FROM content_tags WHERE h_app_id = ? AND content_id = ?",
        )
        .bind::<Text, _>(app)
        .bind::<Text, _>(query.root_id)
        .get_result::<RootTagCount>(&mut conn)?
        .n;

        // One row of the tag-overlap self-join: a co-occurring content id and the
        // number of tags it shares with the root.
        #[derive(diesel::QueryableByName)]
        struct TagOverlapRow {
            #[diesel(sql_type = diesel::sql_types::Text)]
            content_id: String,
            #[diesel(sql_type = diesel::sql_types::BigInt)]
            shared: i64,
        }

        // Self-join: count tags each OTHER content node shares with the root,
        // scoped to the same app/tenant, keep those at/above the threshold.
        // ORDER BY includes a tiebreaker (ct2.content_id ASC) for determinism.
        // LIMIT is inflated by exclude.len() so the Rust-side filter cannot starve
        // the cap; .take(max_computed) after the filter enforces the true cap.
        let rows: Vec<TagOverlapRow> = diesel::sql_query(
            "SELECT ct2.content_id AS content_id, COUNT(*) AS shared \
             FROM content_tags ct1 \
             JOIN content_tags ct2 ON ct1.tag = ct2.tag \
               AND ct1.h_app_id = ct2.h_app_id AND ct2.content_id <> ct1.content_id \
             WHERE ct1.h_app_id = ? AND ct1.content_id = ? \
             GROUP BY ct2.content_id HAVING shared >= ? \
             ORDER BY shared DESC, ct2.content_id ASC LIMIT ?",
        )
        .bind::<Text, _>(app)
        .bind::<Text, _>(query.root_id)
        .bind::<BigInt, _>(query.min_shared_tags as i64)
        .bind::<diesel::sql_types::BigInt, _>((query.max_computed + exclude.len()) as i64)
        .load(&mut conn)?;

        let denom = root_tag_count.max(1) as f64;
        Ok(rows
            .into_iter()
            .filter(|r| !exclude.contains(&r.content_id))
            .take(query.max_computed)
            .map(|r| ResolvedEdge {
                target_id: r.content_id,
                relationship_type: "RELATES_TO".to_string(),
                confidence: (r.shared as f64 / denom).clamp(0.0, 1.0),
                inference_source: "tag".to_string(),
                depth: 1,
            })
            .collect())
    }
}

impl ContentGraphResolver for NativeGraphResolver {
    fn resolve_neighborhood(
        &self,
        ctx: &AppContext,
        query: &GraphQuery<'_>,
    ) -> Result<ResolvedNeighborhood, StorageError> {
        // Pass 1: explicit edges (Category A — notarized, may be persisted).
        let explicit = self.explicit_edges(ctx, query)?;

        // Pass 2: tag-discovery edges (Category C — recompute-on-read, never
        // persisted). Short-circuit entirely when computed edges are off.
        if query.include_computed {
            // Explicit precedence: never re-emit an explicitly-reached node (or
            // the root itself) as a tag edge. Build `seen` from &explicit, then
            // MOVE explicit into edges (no clone).
            let mut seen: HashSet<String> = explicit.iter().map(|e| e.target_id.clone()).collect();
            seen.insert(query.root_id.to_string());
            let computed = self.computed_tag_edges(ctx, query, &seen)?;
            let mut edges = explicit; // move, not clone
            edges.extend(computed);
            return Ok(ResolvedNeighborhood {
                root_id: query.root_id.to_string(),
                edges,
            });
        }

        Ok(ResolvedNeighborhood {
            root_id: query.root_id.to_string(),
            edges: explicit,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::relationships_diesel::test_harness::{
        insert_content_with_tags, insert_relationship, test_pool_ctx,
    };

    /// Pass 2 core payoff: two nodes that share tags with NO authored edge
    /// between them must still be discovered, tagged `inference_source == "tag"`
    /// at `depth == 1` (same-root similarity, not a transitive walk).
    #[test]
    fn tag_overlap_discovers_unlinked_neighbor() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        // X and Y both tagged [grace, sin]; NO relationship between them.
        {
            let mut conn = pool.get().expect("conn");
            insert_content_with_tags(&mut conn, &ctx, "X", &["grace", "sin"]);
            insert_content_with_tags(&mut conn, &ctx, "Y", &["grace", "sin"]);
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: true,
            min_shared_tags: 2,
            ..GraphQuery::new("X")
        };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        let y = n
            .edges
            .iter()
            .find(|e| e.target_id == "Y")
            .expect("Y discovered via tag overlap");
        assert_eq!(y.inference_source, "tag", "discovered edge is a tag edge");
        assert_eq!(y.depth, 1, "tag-discovery edges are depth 1");
    }

    /// Explicit precedence: when X and Y share a tag AND there is an authored
    /// X→Y edge, Y must appear EXACTLY ONCE — as the explicit edge. The tag
    /// edge to Y is suppressed by the `seen`/exclude set.
    #[test]
    fn explicit_precedence_over_computed() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        {
            let mut conn = pool.get().expect("conn");
            insert_content_with_tags(&mut conn, &ctx, "X", &["grace"]);
            insert_content_with_tags(&mut conn, &ctx, "Y", &["grace"]);
            // Authored edge — Y is reachable explicitly too.
            insert_relationship(&mut conn, &ctx, "X", "Y", "RELATES_TO", "explicit");
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: true,
            min_shared_tags: 1,
            ..GraphQuery::new("X")
        };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        let y_edges: Vec<_> = n.edges.iter().filter(|e| e.target_id == "Y").collect();
        assert_eq!(y_edges.len(), 1, "Y appears exactly once (explicit wins)");
        assert_eq!(
            y_edges[0].inference_source, "explicit",
            "the surviving Y edge is the explicit one"
        );
    }

    /// Gating: with `include_computed: false`, Pass 2 must be short-circuited
    /// entirely — no edge may carry `inference_source == "tag"`, even when
    /// tag-sharing content exists.
    #[test]
    fn include_computed_false_yields_no_tag_edges() {
        let (pool, ctx, _tmp) = test_pool_ctx();

        {
            let mut conn = pool.get().expect("conn");
            insert_content_with_tags(&mut conn, &ctx, "X", &["grace"]);
            insert_content_with_tags(&mut conn, &ctx, "Y", &["grace"]);
        }

        let resolver = NativeGraphResolver::new(pool);
        let q = GraphQuery {
            include_computed: false,
            min_shared_tags: 1,
            ..GraphQuery::new("X")
        };
        let n = resolver.resolve_neighborhood(&ctx, &q).unwrap();

        assert!(
            !n.edges.iter().any(|e| e.inference_source == "tag"),
            "no tag edges when include_computed is false"
        );
    }

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
