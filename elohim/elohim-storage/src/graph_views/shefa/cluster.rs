//! Shefa view builder — MyClusterView (graph-backed branch)
//!
//! Walks OPERATES_DEVICE edges from the agent CID using the `household_topology`
//! manifest rule to enumerate the agent's devices.
//! Source of truth: CozoDB graph projection (Operational, Category C).
//!
//! Composition note: device metrics (storage bytes, memory, online status) require
//! peer identity bindings + system metrics (relational reads). Zero-filled here as
//! composition placeholders.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::*;
use crate::views::{
    DeviceArchetype, DeviceSummary, DeviceTotals, Freshness, FreshnessState, MyClusterView,
};
use cozo::DataValue;

/// Build a graph-backed `MyClusterView` for the given `agent_cid`.
///
/// Enumerates devices via OPERATES_DEVICE edges from the agent node.
/// Device metrics are zero-filled composition placeholders — full composition with
/// system_metrics + peer_identity_bindings lands in the follow-on sprint.
pub fn build(engine: &GraphEngine, agent_cid: &str) -> Result<MyClusterView, GraphError> {
    let result = engine.run_script(
        r#"?[device_cid] :=
            *epr_edge{from_cid: $agent, to_cid: device_cid, rel_type: 'OPERATES_DEVICE'}"#,
        &[("agent", DataValue::from(agent_cid))],
    )?;

    let devices: Vec<DeviceSummary> = result
        .rows
        .iter()
        .map(|row| {
            let peer_id = str_at(row, 0);
            DeviceSummary {
                peer_id,
                archetype: DeviceArchetype::Node, // Composition placeholder — requires peer binding
                display_name: None,
                online: false, // Composition placeholder — requires libp2p heartbeat
                freshness: Freshness {
                    state: FreshnessState::Unverifiable,
                    stale_since_ms: None,
                },
                storage_used_bytes: None,
                storage_total_bytes: None,
                memory_used_bytes: None,
                memory_total_bytes: None,
                hosting_count: None,
                projecting_count: None,
                beacon_age_ms: None,
                compute: None, // Left None (A3-followup): this is the graph-native branch whose
                               // build() signature takes only &GraphEngine — no DbPool in scope.
                               // Populating stewarded requires a DbPool for the REA ledger query;
                               // that composition belongs in a follow-up that threads DbPool into
                               // this path or merges the graph and federated paths upstream.
            }
        })
        .collect();

    Ok(MyClusterView {
        agent_cid: agent_cid.to_string(),
        devices,
        totals: DeviceTotals {
            // Composition placeholders — requires relational storage metric aggregation
            storage_used_bytes: 0,
            storage_total_bytes: 0,
            external_committed_bytes: 0,
            reciprocity_net_bytes: 0,
        },
        freshness: Freshness {
            state: FreshnessState::Live,
            stale_since_ms: None,
        },
    })
}
