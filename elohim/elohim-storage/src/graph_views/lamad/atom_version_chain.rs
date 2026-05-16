//! Lamad view builder — AtomVersionChain
//!
//! Walks SUPERSEDES edges forward from the given start CID using the core
//! VERSION_CHAIN Datalog primitive.
//! Source of truth: CozoDB graph projection (Operational, Category C).

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph::primitives::scripts::VERSION_CHAIN;
use crate::graph_views::data_value::*;
use cozo::DataValue;
use serde::{Deserialize, Serialize};

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
    let script = format!("{VERSION_CHAIN}\n?[node] := version_chain[node]");
    let result = engine.run_script(&script, &[("start", DataValue::from(cid))])?;

    let chain: Vec<VersionEntry> = result
        .rows
        .iter()
        .enumerate()
        .map(|(idx, row)| VersionEntry {
            cid: str_at(row, 0),
            // Chain entries start at version 2 (the start CID is implicitly version 1).
            version: (idx + 2) as i64,
            superseded_at: None,
        })
        .collect();

    let canonical_cid = chain.last().map(|e| e.cid.clone());

    Ok(AtomVersionChain {
        current_cid: cid.to_string(),
        chain,
        canonical_cid,
    })
}
