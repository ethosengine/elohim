//! Shefa view builder — PeerTopologyView (graph-backed branch)
//!
//! Composes graph topology edges into the existing `PeerTopologyView` shape.
//! Source of truth: CozoDB graph projection via MEMBER_OF + RECIPROCATES_WITH edges
//! (Operational, Category C).
//!
//! Composition note: edge fields that require peer blob-inventory data
//! (`my_cids_hosted_by_them`, `their_cids_hosted_by_me`, etc.) are populated with
//! defaults here. Task 27 wires the graph-backed path behind the `graph-native` feature
//! flag; full composition with relational reads lands in a follow-on sprint.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::str_at;
use crate::views::{Freshness, FreshnessState, PeerHouseholdEdge, PeerTopologyView};
use cozo::DataValue;

/// Build a graph-backed `PeerTopologyView` for the given `agent_cid`.
///
/// Queries RECIPROCATES_WITH edges to build the edges list. CID-level reciprocation
/// counts (`my_cids_hosted_by_them`, etc.) are zero-filled at this stage — they require
/// peer blob inventory data (composition with relational reads in follow-on sprint).
pub fn build(engine: &GraphEngine, agent_cid: &str) -> Result<PeerTopologyView, GraphError> {
    // Find all contributors this agent reciprocates with (outbound RECIPROCATES_WITH edges).
    let result = engine.run_script(
        r#"?[peer_cid] :=
            *epr_edge{from_cid: $agent, to_cid: peer_cid, rel_type: 'RECIPROCATES_WITH'}"#,
        &[("agent", DataValue::from(agent_cid))],
    )?;

    let edges: Vec<PeerHouseholdEdge> = result
        .rows
        .iter()
        .map(|row| {
            let household_id = str_at(row, 0);
            PeerHouseholdEdge {
                household_id,
                display_name: None,
                online: false, // Composition placeholder — requires libp2p swarm state
                last_sync_sec: None,
                // Composition placeholder — requires peer blob inventory reads
                my_cids_hosted_by_them: 0,
                their_cids_hosted_by_me: 0,
                net_diff: None,
                is_critical_for_me: None,
                i_am_critical_for_them: None,
            }
        })
        .collect();

    let reciprocation_count = edges.len() as u32;

    Ok(PeerTopologyView {
        agent_cid: agent_cid.to_string(),
        edges,
        reciprocation_count,
        resilience_cliffs: vec![], // Composition placeholder — requires blob sole-replica query
        freshness: Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        },
    })
}
