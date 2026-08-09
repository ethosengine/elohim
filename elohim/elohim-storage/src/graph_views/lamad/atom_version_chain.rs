//! Lamad view builder — AtomVersionChain
//!
//! Walks SUPERSEDES edges forward from the given start CID using the core
//! VERSION_CHAIN_DEPTH Datalog primitive (hop-counted, so chain order and the
//! chain tip can be recovered without trusting CozoDB's row order).
//! Source of truth: CozoDB graph projection (Operational, Category C).

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph::primitives::scripts::VERSION_CHAIN_DEPTH;
use crate::graph_views::data_value::*;
use cozo::DataValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Chain of EPR atom CIDs linked by SUPERSEDES edges.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AtomVersionChain {
    pub current_cid: String,
    pub chain: Vec<VersionEntry>,
    pub canonical_cid: Option<String>,
}

/// A single successor in the version chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionEntry {
    pub cid: String,
    /// Position in the chain (2, 3, … since the start CID is implicitly v1).
    pub version: i64,
    pub superseded_at: Option<String>,
}

/// Build an `AtomVersionChain` starting from `cid`.
///
/// The chain contains all successor nodes reachable via SUPERSEDES edges.
/// `canonical_cid` is the final entry (the most recent version), or None when
/// the atom has no successors.
pub fn build(engine: &GraphEngine, cid: &str) -> Result<AtomVersionChain, GraphError> {
    // NOTE: CozoDB sorts result rows lexicographically on the returned tuple,
    // NOT in traversal order — the datalog rule carries `hops` explicitly so
    // chain order (and the chain tip) can be recovered by sorting on depth
    // instead of trusting row order or CID lexicographic order.
    let script = format!("{VERSION_CHAIN_DEPTH}\n?[node, hops] := version_chain_depth[node, hops]");
    let result = engine.run_script(&script, &[("start", DataValue::from(cid))])?;

    // A node reachable via more than one path can be derived at multiple
    // depths; keep only its maximum hop count before numbering.
    let mut max_hops_by_node: HashMap<String, i64> = HashMap::new();
    for row in &result.rows {
        let node = str_at(row, 0);
        let hops = i64_at(row, 1);
        max_hops_by_node
            .entry(node)
            .and_modify(|h| *h = (*h).max(hops))
            .or_insert(hops);
    }

    let mut entries: Vec<(String, i64)> = max_hops_by_node.into_iter().collect();
    // Order by hop depth (chain order); tiebreak lexicographically on CID for
    // a deterministic order among nodes at the same depth (a fork — an
    // acknowledged open question, not resolved here).
    entries.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));

    let chain: Vec<VersionEntry> = entries
        .into_iter()
        .map(|(node, hops)| VersionEntry {
            cid: node,
            // The start CID is implicitly version 1; a node at hop depth N is version N+1.
            version: hops + 1,
            superseded_at: None,
        })
        .collect();

    // The canonical version is the chain tip: the node with the greatest hop
    // depth (last entry, since `chain` is sorted by hops ascending).
    let canonical_cid = chain.last().map(|e| e.cid.clone());

    Ok(AtomVersionChain {
        current_cid: cid.to_string(),
        chain,
        canonical_cid,
    })
}
