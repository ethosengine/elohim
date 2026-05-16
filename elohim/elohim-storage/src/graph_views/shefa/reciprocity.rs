//! Shefa view builder — ReciprocityView (graph-backed branch)
//!
//! Composes RECIPROCATES_WITH edges using the `reciprocity_flow_to` manifest rule
//! into the existing `ReciprocityView` shape.
//! Source of truth: CozoDB graph projection (Operational, Category C).
//!
//! Composition note: `committed_bytes`, `delivered_bytes`, `honored_percent` require
//! REA economic event data (relational reads). These are zero-filled here as composition
//! placeholders for the follow-on sprint that wires full relational composition.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::*;
use crate::views::{ReciprocityRow, ReciprocityView};
use cozo::DataValue;

/// Inline rule body — matches shefa manifest `reciprocity_flow_to`.
const RECIPROCITY_FLOW_TO: &str = r#"
    reciprocity_flow_to[from] :=
        *epr_edge{from_cid: from, to_cid: $contributor, rel_type: 'RECIPROCATES_WITH'}
"#;

/// Build a graph-backed `ReciprocityView` for the given `agent_cid`.
///
/// `inflow` lists contributors sending RECIPROCATES_WITH to this agent.
/// `outflow` lists contributors this agent sends RECIPROCATES_WITH to.
/// Byte-level fields are zero-filled (composition with REA economic events in follow-on sprint).
pub fn build(engine: &GraphEngine, agent_cid: &str) -> Result<ReciprocityView, GraphError> {
    // Inbound: who reciprocates toward this agent (reciprocity_flow_to pattern).
    let inflow_script = format!("{RECIPROCITY_FLOW_TO}\n?[from] := reciprocity_flow_to[from]");
    let inflow_result = engine.run_script(
        &inflow_script,
        &[("contributor", DataValue::from(agent_cid))],
    )?;

    let inflow: Vec<ReciprocityRow> = inflow_result
        .rows
        .iter()
        .map(|row| ReciprocityRow {
            counterparty_household_id: str_at(row, 0),
            display_name: None,
            // Composition placeholders — requires REA economic event relational reads
            committed_bytes: 0,
            delivered_bytes: 0,
            honored_percent: 0.0,
            online: None,
        })
        .collect();

    // Outbound: contributors this agent sends RECIPROCATES_WITH to.
    let outflow_result = engine.run_script(
        r#"?[to_cid] :=
            *epr_edge{from_cid: $agent, to_cid, rel_type: 'RECIPROCATES_WITH'}"#,
        &[("agent", DataValue::from(agent_cid))],
    )?;

    let outflow: Vec<ReciprocityRow> = outflow_result
        .rows
        .iter()
        .map(|row| ReciprocityRow {
            counterparty_household_id: str_at(row, 0),
            display_name: None,
            committed_bytes: 0,
            delivered_bytes: 0,
            honored_percent: 0.0,
            online: None,
        })
        .collect();

    Ok(ReciprocityView {
        agent_cid: agent_cid.to_string(),
        inflow,
        outflow,
        net_hosted_bytes: 0, // Composition placeholder — requires REA byte accounting
        capacity_available_bytes: 0, // Composition placeholder — requires storage metrics
    })
}
