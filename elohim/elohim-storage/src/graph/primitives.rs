use crate::graph::engine::{GraphEngine, GraphError};

/// Registers core named Datalog rule fragments: neighborhood, path, reach_filtered, version_chain.
///
/// CozoDB named rules are not persisted independently — they must be inlined into the script
/// that uses them. This module provides const rule bodies that callers compose with their
/// own query heads.
///
/// IMPORTANT: CozoDB 0.7 uses plain variable names (no `?` prefix) inside rule bodies;
/// the `?` prefix is only valid in the final query head (e.g. `?[to, hops] := ...`).
/// Rule bodies use bare identifiers: `to`, `hops`, `via`, etc.
///
/// Domains MUST NOT shadow these rule names; the manifest validator enforces this at
/// registration time.
///
/// No persistent state is written by this function; it validates that the required
/// schema relations are present before accepting domain primitives that depend on them.
pub fn register_core_primitives(engine: &GraphEngine) -> Result<(), GraphError> {
    // Validate required schema relations exist by running trivially-false queries
    engine.run_script("?[x] := *epr_edge{from_cid: x}, false", &[])?;
    engine.run_script("?[x] := *epr_qahal{cid: x}, false", &[])?;
    Ok(())
}

pub mod scripts {
    /// Recursive neighbourhood walk: returns all nodes reachable from `$start` within
    /// `$max_hops` hops via any edge type.
    ///
    /// Parameters: `$start: String`, `$max_hops: Int`
    /// Query head to append: `?[to, hops] := neighborhood[to, hops]`
    ///
    /// NOTE: variable names use plain identifiers (no `?` prefix) in rule bodies —
    /// CozoDB 0.7 only accepts `?` prefix in the final query head.
    pub const NEIGHBORHOOD: &str = r#"
        neighborhood[to, hops] :=
            *epr_edge{from_cid: $start, to_cid: to},
            hops = 1
        neighborhood[to, hops] :=
            neighborhood[via, prev_hops],
            *epr_edge{from_cid: via, to_cid: to},
            hops = prev_hops + 1,
            hops <= $max_hops
    "#;

    /// Follows SUPERSEDES edges forward from `$start`, returning each successor node.
    ///
    /// Parameters: `$start: String`
    /// Query head to append: `?[node] := version_chain[node]`
    pub const VERSION_CHAIN: &str = r#"
        version_chain[node] :=
            *epr_edge{from_cid: $start, to_cid: node, rel_type: 'SUPERSEDES'}
        version_chain[node] :=
            version_chain[prev],
            *epr_edge{from_cid: prev, to_cid: node, rel_type: 'SUPERSEDES'}
    "#;

    /// Filters EPR nodes by minimum reach level (lexicographic comparison on String).
    ///
    /// Parameters: `$reach_floor: String`
    /// Query head to append: `?[node] := reach_filtered[node]`
    ///
    /// NOTE: reach comparison is lexicographic (String type). A future sprint will
    /// replace this with an ordinal index once reach is encoded as Int in epr_qahal.
    pub const REACH_FILTERED: &str = r#"
        reach_filtered[node] :=
            *epr_qahal{cid: node, reach: r},
            r >= $reach_floor
    "#;
}
