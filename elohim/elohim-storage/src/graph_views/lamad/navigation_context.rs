//! Lamad view builder — NavigationContextView
//!
//! Returns the resolved atom + its navigation origin + the 2-hop neighbourhood
//! using the core NEIGHBORHOOD Datalog primitive.
//! Source of truth: CozoDB graph projection (Operational, Category C).

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph::primitives::scripts::NEIGHBORHOOD;
use crate::graph_views::data_value::*;
use crate::graph_views::lamad::resolved_atom::{self, ResolvedAtomView};
use cozo::DataValue;
use serde::{Deserialize, Serialize};

/// Graph-traversal contextual view for an EPR atom.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationContextView {
    pub atom: ResolvedAtomView,
    pub origin_context: OriginContext,
    pub neighborhood: Vec<NeighborhoodEntry>,
}

/// Navigation origin metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginContext {
    pub origin_cid: String,
    pub origin_type: String,
    pub origin_path: Option<String>,
}

/// A single neighbour atom reachable via the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NeighborhoodEntry {
    pub cid: String,
    /// The edge type from the start atom (or 'multi-hop' for indirect neighbours).
    pub rel_type: String,
    pub hops: i64,
}

/// Build a `NavigationContextView` for `cid`, navigated from `origin_cid`.
///
/// Neighbourhood is computed up to `max_hops` (default 2) using the NEIGHBORHOOD primitive.
/// Direct edges carry the rel_type from the graph; indirect edges carry "multi-hop".
pub fn build(
    engine: &GraphEngine,
    cid: &str,
    origin_cid: &str,
) -> Result<NavigationContextView, GraphError> {
    let atom = resolved_atom::build(engine, cid)?;

    // Fetch 1-hop edges with their rel_type for richer output.
    let direct_result = engine.run_script(
        r#"?[to_cid, rel_type, hops] :=
            *epr_edge{from_cid: $start, to_cid, rel_type},
            hops = 1"#,
        &[("start", DataValue::from(cid))],
    )?;

    // Fetch 2-hop neighbourhood (all reachable within 2 hops).
    let script =
        format!("{NEIGHBORHOOD}\n?[to, hops] := neighborhood[to, hops], hops <= $max_hops");
    let nb_result = engine.run_script(
        &script,
        &[
            ("start", DataValue::from(cid)),
            ("max_hops", DataValue::from(2_i64)),
        ],
    )?;

    // Build a set of 1-hop CIDs so we can annotate rel_type correctly.
    let mut direct_map: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for row in &direct_result.rows {
        let to = str_at(row, 0);
        let rel = str_at(row, 1);
        direct_map.insert(to, rel);
    }

    let neighborhood = nb_result
        .rows
        .iter()
        .map(|row| {
            let neighbour_cid = str_at(row, 0);
            let hops = i64_at(row, 1);
            let rel_type = direct_map
                .get(&neighbour_cid)
                .cloned()
                .unwrap_or_else(|| "multi-hop".to_string());
            NeighborhoodEntry {
                cid: neighbour_cid,
                rel_type,
                hops,
            }
        })
        .collect();

    Ok(NavigationContextView {
        atom,
        origin_context: OriginContext {
            origin_cid: origin_cid.to_string(),
            origin_type: "epr".to_string(),
            origin_path: None,
        },
        neighborhood,
    })
}
