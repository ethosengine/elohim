//! Node-shape publish surface.
//!
//! `register_node_shape` is the coordinator fn invoked by elohim-node at boot
//! to commit its durable shape (hostname, archetype, committed resources,
//! household binding) as a `NodeRegistration` DHT entry. Source of truth is
//! the DHT entry; elohim-storage's `stewarded_nodes` is the operational
//! projection.
//!
//! This is a thin wrapper over the existing `NodeRegistration` entry type —
//! no new DHT entry types introduced. It maps the camelCase NodeShapeInput
//! (mirrors node-shape-view.schema.json) onto the existing integrity struct.

use hdk::prelude::*;
use node_registry_integrity::{EntryTypes, NodeRegistration};

/// Input for `register_node_shape`.
///
/// Fields mirror `elohim/sdk/schemas/v1/views/node-shape-view.schema.json` —
/// the wire view for node shape. Committed fields are flattened here for
/// DNA-side simplicity; elohim-storage re-nests them in the view type.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeShapeInput {
    pub node_id: String,
    pub hostname: String,
    pub device_archetype_id: String,
    pub household_id: String,
    pub role: String,
    pub capability_level: u8,
    pub cpu_cores: u32,
    pub memory_gb: u32,
    pub storage_tb: f64,
    pub bandwidth_mbps: Option<u32>,
    pub max_custody_gb: Option<f64>,
    pub can_steward: bool,
    pub can_infer: bool,
    pub can_doorway: bool,
    pub steward_tier: Option<String>,
    pub custodian_opt_in: bool,
    pub region: Option<String>,
    pub signature: String,
    pub signed_at: String,
}

/// Commit a node shape as a `NodeRegistration` DHT entry authored by the
/// boot peer. Returns the ActionHash for storage-side `dht_anchor_hash`.
#[hdk_extern]
pub fn register_node_shape(input: NodeShapeInput) -> ExternResult<ActionHash> {
    let agent = agent_info()?.agent_initial_pubkey.to_string();
    let reg = NodeRegistration {
        node_id: input.node_id,
        agent_pub_key: agent,
        display_name: input.hostname.clone(),
        cpu_cores: input.cpu_cores,
        memory_gb: input.memory_gb,
        storage_tb: input.storage_tb,
        bandwidth_mbps: input.bandwidth_mbps.unwrap_or(0),
        region: input.region.clone().unwrap_or_default(),
        latitude: None,
        longitude: None,
        zomes_hosted: vec![],
        steward_tier: input.steward_tier.unwrap_or_else(|| "caretaker".into()),
        custodian_opt_in: input.custodian_opt_in,
        max_custody_gb: input.max_custody_gb,
        max_bandwidth_mbps: input.bandwidth_mbps,
        max_cpu_percent: None,
        uptime_percent: 0.0,
        last_heartbeat: input.signed_at.clone(),
        registered_at: input.signed_at.clone(),
        updated_at: input.signed_at,
        claim_status: "claimed".into(),
        context_epr_id: None,
        signature: input.signature,
    };
    create_entry(EntryTypes::NodeRegistration(reg))
}
