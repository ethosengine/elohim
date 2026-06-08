//! The native content-graph seam. The ONE place the content graph is realized.
//! Composes explicit (notarized, Category A) + computed (recompute-on-read,
//! Category C) edges. Read-only: this trait has no write method by design —
//! a computed edge can never be persisted through it.

use crate::db::context::AppContext;
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
