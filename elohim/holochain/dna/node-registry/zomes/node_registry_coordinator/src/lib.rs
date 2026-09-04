use hdk::prelude::*;
use node_registry_integrity::*;
use sha2::{Digest, Sha256};

// Re-export integrity types for convenience
pub use node_registry_integrity::{
    NodeRegistration, NodeHeartbeat, HealthAttestation, CustodianAssignment,
    ShardAssignment, ShardStatus, ShardingStrategy,
    EntryTypes, LinkTypes,
};

// Gated behind `lineage-witness` — see node_registry_integrity's Cargo.toml.
#[cfg(feature = "lineage-witness")]
pub use node_registry_integrity::{NotarizationWitness, CarriedProof, LineageProperties, WITNESS_BATCH};

pub mod shape;
pub use shape::{NodeShapeInput, register_node_shape};

// Bootstrap-steward pattern (node-registry copy; reference lives in imagodei).
pub mod bootstrap_steward;
pub use bootstrap_steward::{
    am_i_bootstrap_steward, bootstrap_steward, maybe_bootstrap_steward, BootstrapStewardError,
    DnaProperties,
};

/// Retired: frontend should call `/api/v1/households/{id}/devices` via
/// elohim-storage, which projects `NodeRegistration` DHT entries filtered by
/// household. This stub returns an empty Vec so legacy callers (frontend
/// `NodeRegistryAnchor`) don't crash during the cut-over. Remove once no
/// caller remains.
#[hdk_extern]
pub fn get_my_nodes(_: ()) -> ExternResult<Vec<NodeRegistration>> {
    Ok(vec![])
}

// ============================================================================
// NODE LIFECYCLE FUNCTIONS
// ============================================================================

/// Register a new node in the network
/// Creates index links for efficient discovery by region, status, tier
#[hdk_extern]
pub fn register_node(registration: NodeRegistration) -> ExternResult<ActionHash> {
    // Create the NodeRegistration entry
    let hash = create_entry(EntryTypes::NodeRegistration(registration.clone()))?;

    // Create index links for efficient querying

    // 1. Link from region anchor to this node
    let region_anchor = StringAnchor {
        anchor_type: "region".to_string(),
        anchor_value: registration.region.clone(),
    };
    let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;
    create_link(
        region_anchor_hash,
        hash.clone(),
        LinkTypes::RegionToNode,
        (),
    )?;

    // 2. Link from status anchor to this node
    let status_anchor = StringAnchor {
        anchor_type: "status".to_string(),
        anchor_value: "online".to_string(), // New nodes start as online
    };
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;
    create_link(
        status_anchor_hash,
        hash.clone(),
        LinkTypes::StatusToNode,
        (),
    )?;

    // 3. Link from tier anchor to this node
    let tier_anchor = StringAnchor {
        anchor_type: "tier".to_string(),
        anchor_value: registration.steward_tier.clone(),
    };
    let tier_anchor_hash = hash_entry(&EntryTypes::StringAnchor(tier_anchor))?;
    create_link(
        tier_anchor_hash,
        hash.clone(),
        LinkTypes::TierToNode,
        (),
    )?;

    // 4. Link from node ID anchor to this registration (for lookups by ID)
    let id_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: registration.node_id.clone(),
    };
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        hash.clone(),
        LinkTypes::IdToNodeRegistration,
        (),
    )?;

    // 5. If custodian opt-in is enabled, link from custodian anchor
    if registration.custodian_opt_in {
        let custodian_anchor = StringAnchor {
            anchor_type: "custodian".to_string(),
            anchor_value: "available".to_string(),
        };
        let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;
        create_link(
            custodian_anchor_hash,
            hash.clone(),
            LinkTypes::CustodianToNode,
            (),
        )?;
    }

    Ok(hash)
}

/// Update node capacity information
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CapacityUpdates {
    pub cpu_cores: Option<u32>,
    pub memory_gb: Option<u32>,
    pub storage_tb: Option<f64>,
    pub bandwidth_mbps: Option<u32>,
    pub max_custody_gb: Option<f64>,
    pub max_bandwidth_mbps: Option<u32>,
    pub max_cpu_percent: Option<f64>,
}

#[hdk_extern]
pub fn update_node_capacity(input: UpdateCapacityInput) -> ExternResult<ActionHash> {
    // Get the current registration
    let mut registration = get_node_registration_by_id(input.node_id.clone())?;

    // Apply updates
    if let Some(cpu_cores) = input.updates.cpu_cores {
        registration.cpu_cores = cpu_cores;
    }
    if let Some(memory_gb) = input.updates.memory_gb {
        registration.memory_gb = memory_gb;
    }
    if let Some(storage_tb) = input.updates.storage_tb {
        registration.storage_tb = storage_tb;
    }
    if let Some(bandwidth_mbps) = input.updates.bandwidth_mbps {
        registration.bandwidth_mbps = bandwidth_mbps;
    }
    if let Some(max_custody_gb) = input.updates.max_custody_gb {
        registration.max_custody_gb = Some(max_custody_gb);
    }
    if let Some(max_bandwidth_mbps) = input.updates.max_bandwidth_mbps {
        registration.max_bandwidth_mbps = Some(max_bandwidth_mbps);
    }
    if let Some(max_cpu_percent) = input.updates.max_cpu_percent {
        registration.max_cpu_percent = Some(max_cpu_percent);
    }

    registration.updated_at = timestamp_now()?;

    // Update the entry
    let original_hash = get_node_registration_hash(&input.node_id)?;
    update_entry(original_hash, &EntryTypes::NodeRegistration(registration))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCapacityInput {
    pub node_id: String,
    pub updates: CapacityUpdates,
}

/// Deregister a node from the network
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DeregisterInput {
    pub node_id: String,
    pub reason: String,
}

#[hdk_extern]
pub fn deregister_node(input: DeregisterInput) -> ExternResult<()> {
    // Get the registration
    let _registration = get_node_registration_by_id(input.node_id.clone())?;
    let registration_hash = get_node_registration_hash(&input.node_id)?;

    // Delete all links associated with this node
    // (In production, you might want to keep historical data and just mark as deregistered)
    delete_entry(registration_hash)?;

    // TODO: Trigger disaster recovery for any content this node was custodying

    Ok(())
}

// ============================================================================
// STEWARDSHIP CLAIM FUNCTIONS
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimNodeInput {
    pub node_id: String,
    pub agent_pub_key: String,
}

/// Claim an unclaimed node — transitions claim_status to "claimed"
#[hdk_extern]
pub fn claim_node(_input: ClaimNodeInput) -> ExternResult<ActionHash> {
    // Look up existing registration by node_id
    // Validate claim_status == "unclaimed"
    // Create updated entry with claim_status = "claimed"
    // Return new action hash
    todo!("Will be implemented when conductor integration is wired")
}

/// Release a claimed node — transitions claim_status to "released"
#[hdk_extern]
pub fn release_node(_node_id: String) -> ExternResult<ActionHash> {
    // Look up existing registration
    // Create updated entry with claim_status = "released"
    // Return new action hash
    todo!("Will be implemented when conductor integration is wired")
}

// ============================================================================
// HEALTH TRACKING FUNCTIONS
// ============================================================================

/// Submit a heartbeat to signal node is still alive
#[hdk_extern]
pub fn heartbeat(heartbeat_data: NodeHeartbeat) -> ExternResult<ActionHash> {
    // Create the heartbeat entry
    let hash = create_entry(EntryTypes::NodeHeartbeat(heartbeat_data.clone()))?;

    // Link from node to heartbeat
    let node_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: heartbeat_data.node_id.clone(),
    };
    let node_anchor_hash = hash_entry(&EntryTypes::StringAnchor(node_anchor))?;
    create_link(
        node_anchor_hash,
        hash.clone(),
        LinkTypes::NodeToHeartbeat,
        (),
    )?;

    Ok(hash)
}

/// Attest to the health of a peer node
#[hdk_extern]
pub fn attest_health(attestation: HealthAttestation) -> ExternResult<ActionHash> {
    // Prevent self-attestation
    let my_agent_info = agent_info()?;
    let my_node = get_node_by_agent(my_agent_info.agent_initial_pubkey)?;

    if my_node.node_id == attestation.subject_node_id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Cannot attest to own health".to_string()
        )));
    }

    // Create the attestation entry
    let hash = create_entry(EntryTypes::HealthAttestation(attestation.clone()))?;

    // Link from subject node to attestation
    let subject_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: attestation.subject_node_id.clone(),
    };
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;
    create_link(
        subject_anchor_hash,
        hash.clone(),
        LinkTypes::NodeToAttestations,
        (),
    )?;

    Ok(hash)
}

/// Get health summary for a node based on recent attestations
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct NodeHealthSummary {
    pub node_id: String,
    pub status: String,
    pub confidence: f64,
    pub avg_response_time_ms: Option<u32>,
    pub last_heartbeat: Option<String>,
    pub successful_pings: u32,
    pub failed_pings: u32,
}

#[hdk_extern]
pub fn get_node_health(node_id: String) -> ExternResult<NodeHealthSummary> {
    // Get latest heartbeat
    let last_heartbeat = get_latest_heartbeat(&node_id).ok();

    // Get recent attestations (last 5 minutes)
    let attestations = get_recent_attestations(&node_id, 300)?;

    let mut successful_pings = 0;
    let mut failed_pings = 0;
    let mut total_response_time: u64 = 0;

    for attestation in attestations.iter() {
        if attestation.success {
            successful_pings += 1;
            total_response_time += attestation.response_time_ms as u64;
        } else {
            failed_pings += 1;
        }
    }

    let total_attestations = successful_pings + failed_pings;

    let (status, confidence) = if total_attestations == 0 {
        ("unknown".to_string(), 0.0)
    } else {
        let health_ratio = successful_pings as f64 / total_attestations as f64;
        let status = if health_ratio >= 0.90 {
            "healthy"
        } else if health_ratio >= 0.60 {
            "degraded"
        } else {
            "unhealthy"
        };
        (status.to_string(), health_ratio)
    };

    let avg_response_time_ms = if successful_pings > 0 {
        Some((total_response_time / successful_pings as u64) as u32)
    } else {
        None
    };

    Ok(NodeHealthSummary {
        node_id,
        status,
        confidence,
        avg_response_time_ms,
        last_heartbeat: last_heartbeat.map(|hb| hb.timestamp),
        successful_pings,
        failed_pings,
    })
}

// ============================================================================
// DISCOVERY FUNCTIONS
// ============================================================================

/// Get all nodes in a specific region
#[hdk_extern]
pub fn get_nodes_by_region(region: String) -> ExternResult<Vec<NodeRegistration>> {
    let region_anchor = StringAnchor {
        anchor_type: "region".to_string(),
        anchor_value: region,
    };
    let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;

    let query = LinkQuery::try_new(region_anchor_hash, LinkTypes::RegionToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut nodes = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(registration) = deserialize_node_registration(&record)? {
                    nodes.push(registration);
                }
            }
        }
    }

    Ok(nodes)
}

/// Filter criteria for finding custodian nodes
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustodianFilters {
    pub region: Option<String>,
    pub min_storage_gb: Option<f64>,
    pub min_bandwidth_mbps: Option<u32>,
    pub min_tier: Option<String>,
    pub exclude_nodes: Option<Vec<String>>,
    pub status: Option<String>,
}

/// Get available custodian nodes matching filters
#[hdk_extern]
pub fn get_available_custodians(filters: CustodianFilters) -> ExternResult<Vec<NodeRegistration>> {
    // Start with all nodes that opted in to custodianship
    let custodian_anchor = StringAnchor {
        anchor_type: "custodian".to_string(),
        anchor_value: "available".to_string(),
    };
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;

    let query = LinkQuery::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut candidates = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(registration) = deserialize_node_registration(&record)? {

                    // Apply filters
                    if let Some(ref region) = filters.region {
                        if &registration.region != region {
                            continue;
                        }
                    }

                    if let Some(min_storage) = filters.min_storage_gb {
                        let available_storage = registration.max_custody_gb.unwrap_or(0.0);
                        if available_storage < min_storage {
                            continue;
                        }
                    }

                    if let Some(min_bandwidth) = filters.min_bandwidth_mbps {
                        let available_bandwidth = registration.max_bandwidth_mbps.unwrap_or(0);
                        if available_bandwidth < min_bandwidth {
                            continue;
                        }
                    }

                    if let Some(ref min_tier) = filters.min_tier {
                        // Tier hierarchy: caretaker < guardian < steward < pioneer
                        if !meets_tier_requirement(&registration.steward_tier, min_tier) {
                            continue;
                        }
                    }

                    if let Some(ref exclude_list) = filters.exclude_nodes {
                        if exclude_list.contains(&registration.node_id) {
                            continue;
                        }
                    }

                    // TODO: Check status from recent heartbeats

                    candidates.push(registration);
                }
            }
        }
    }

    Ok(candidates)
}

/// Get all nodes at a specific steward tier
#[hdk_extern]
pub fn get_nodes_by_tier(tier: String) -> ExternResult<Vec<NodeRegistration>> {
    let tier_anchor = StringAnchor {
        anchor_type: "tier".to_string(),
        anchor_value: tier,
    };
    let tier_anchor_hash = hash_entry(&EntryTypes::StringAnchor(tier_anchor))?;

    let query = LinkQuery::try_new(tier_anchor_hash, LinkTypes::TierToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut nodes = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(registration) = deserialize_node_registration(&record)? {
                    nodes.push(registration);
                }
            }
        }
    }

    Ok(nodes)
}

// ============================================================================
// CUSTODIAN ASSIGNMENT FUNCTIONS
// ============================================================================

/// Assign a custodian to content
#[hdk_extern]
pub fn assign_custodian(assignment: CustodianAssignment) -> ExternResult<ActionHash> {
    // Create the assignment entry
    let hash = create_entry(EntryTypes::CustodianAssignment(assignment.clone()))?;

    // Link from content to assignment
    let content_anchor = StringAnchor {
        anchor_type: "content_id".to_string(),
        anchor_value: assignment.content_id.clone(),
    };
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(
        content_anchor_hash,
        hash.clone(),
        LinkTypes::ContentToAssignment,
        (),
    )?;

    // Link from custodian node to assignment
    let node_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: assignment.custodian_node_id.clone(),
    };
    let node_anchor_hash = hash_entry(&EntryTypes::StringAnchor(node_anchor))?;
    create_link(
        node_anchor_hash,
        hash.clone(),
        LinkTypes::NodeToAssignment,
        (),
    )?;

    Ok(hash)
}

/// Get all custodian assignments for a piece of content
#[hdk_extern]
pub fn get_assignments_for_content(content_id: String) -> ExternResult<Vec<CustodianAssignment>> {
    let content_anchor = StringAnchor {
        anchor_type: "content_id".to_string(),
        anchor_value: content_id,
    };
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

    let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToAssignment)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut assignments = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(assignment) = deserialize_custodian_assignment(&record)? {
                    assignments.push(assignment);
                }
            }
        }
    }

    Ok(assignments)
}

/// Get all custodian assignments for a node
#[hdk_extern]
pub fn get_assignments_for_node(node_id: String) -> ExternResult<Vec<CustodianAssignment>> {
    let node_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id,
    };
    let node_anchor_hash = hash_entry(&EntryTypes::StringAnchor(node_anchor))?;

    let query = LinkQuery::try_new(node_anchor_hash, LinkTypes::NodeToAssignment)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut assignments = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(assignment) = deserialize_custodian_assignment(&record)? {
                    assignments.push(assignment);
                }
            }
        }
    }

    Ok(assignments)
}

/// Input for auto-assigning custodians to a newly registered node
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AutoAssignInput {
    /// Node ID to assign custodian responsibilities to
    pub node_id: String,
    /// Maximum total GB to assign
    pub max_total_gb: Option<f64>,
    /// Preferred content reach levels (0-7, where 0=private, 7=commons)
    pub preferred_reach_levels: Option<Vec<u8>>,
    /// Maximum number of assignments to create
    pub max_assignments: Option<u32>,
}

/// Result of auto-assignment operation
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AutoAssignResult {
    pub node_id: String,
    pub assignments_created: u32,
    pub total_gb_assigned: f64,
    pub assignment_hashes: Vec<ActionHash>,
    pub skipped_content: Vec<String>,
}

/// Auto-assign custodian responsibilities to a newly registered node
///
/// This function:
/// 1. Finds content needing additional custodians in the node's region
/// 2. Prioritizes content by reach level (commons first) and under-replicated items
/// 3. Creates assignments up to the node's capacity limits
/// 4. Emits signals for orchestrator to coordinate blob transfer
#[hdk_extern]
pub fn auto_assign_custodians(input: AutoAssignInput) -> ExternResult<AutoAssignResult> {
    // Get the node registration to know capacity and region
    let registration = get_node_registration_by_id(input.node_id.clone())?;

    // Determine maximum capacity
    let max_gb = input.max_total_gb
        .or(registration.max_custody_gb)
        .unwrap_or(100.0); // Default 100GB

    let max_assignments = input.max_assignments.unwrap_or(50);

    // Get content needing custodians in this region
    let content_needing_custodians = find_content_needing_custodians(
        &registration.region,
        input.preferred_reach_levels.clone(),
    )?;

    let mut result = AutoAssignResult {
        node_id: input.node_id.clone(),
        assignments_created: 0,
        total_gb_assigned: 0.0,
        assignment_hashes: Vec::new(),
        skipped_content: Vec::new(),
    };

    // Create assignments until we hit limits
    for content in content_needing_custodians {
        // Check if we've hit limits
        if result.assignments_created >= max_assignments {
            result.skipped_content.push(content.content_id.clone());
            continue;
        }

        let content_gb = content.size_gb.unwrap_or(0.1);
        if result.total_gb_assigned + content_gb > max_gb {
            result.skipped_content.push(content.content_id.clone());
            continue;
        }

        // Create the assignment
        let assignment = CustodianAssignment {
            assignment_id: format!("auto-{}-{}", input.node_id, sys_time()?.as_micros()),
            content_id: content.content_id.clone(),
            content_hash: content.content_hash.clone(),
            custodian_node_id: input.node_id.clone(),
            strategy: content.strategy.clone().unwrap_or("full_replica".to_string()),
            shard_index: None,
            preferred_region: Some(registration.region.clone()),
            required_tier: Some(registration.steward_tier.clone()),
            content_size_gb: Some(content_gb),
            decided_by: "auto_assign_custodians".to_string(),
            decision_round: Some(1),
            votes_json: "".to_string(),
            created_at: timestamp_now()?,
            expires_at: calculate_expiration(365)?, // 1 year
        };

        match assign_custodian(assignment) {
            Ok(hash) => {
                result.assignments_created += 1;
                result.total_gb_assigned += content_gb;
                result.assignment_hashes.push(hash);

                // Emit signal for orchestrator to transfer content
                emit_signal(Signal::ReplicateContent {
                    content_id: content.content_id.clone(),
                    content_hash: content.content_hash,
                    from_custodians: content.existing_custodians,
                    to_custodian: input.node_id.clone(),
                    strategy: content.strategy.unwrap_or("full_replica".to_string()),
                })?;
            }
            Err(_) => {
                // Log error but continue with other assignments
                result.skipped_content.push(content.content_id);
            }
        }
    }

    Ok(result)
}

/// Content that needs additional custodians
#[derive(Debug, Clone)]
struct ContentNeedingCustodian {
    content_id: String,
    content_hash: String,
    size_gb: Option<f64>,
    reach_level: u8,
    current_replicas: u32,
    target_replicas: u32,
    existing_custodians: Vec<String>,
    strategy: Option<String>,
}

/// Find content that needs additional custodians in a region
fn find_content_needing_custodians(
    _region: &str,
    _preferred_reach_levels: Option<Vec<u8>>,
) -> ExternResult<Vec<ContentNeedingCustodian>> {
    // In a full implementation, this would:
    // 1. Query a content index anchor by region
    // 2. Check each content's current replica count vs target
    // 3. Filter by reach level if specified
    // 4. Sort by priority (under-replicated first, then by reach level)

    // For now, return empty - orchestrator handles actual content discovery
    // This would be populated by content DNA or projection

    // Placeholder: In production, query content needing replication
    // let content_anchor = StringAnchor {
    //     anchor_type: "content_region".to_string(),
    //     anchor_value: region.to_string(),
    // };
    // ... query and filter content ...

    Ok(Vec::new())
}

/// Get target replica count based on reach level
fn target_replicas_for_reach(reach_level: u8) -> u32 {
    // Higher reach = more replicas needed
    match reach_level {
        0 => 3,      // Private: 3 replicas (family cluster)
        1 => 3,      // Invited: 3 replicas
        2 => 5,      // Local: 5 replicas
        3 => 7,      // Neighborhood: 7 replicas
        4 => 10,     // Municipal: 10 replicas
        5 => 15,     // Bioregional: 15 replicas
        6 => 20,     // Regional: 20 replicas
        7 => 30,     // Commons: 30 replicas (widely available)
        _ => 5,      // Default
    }
}

// ============================================================================
// DISASTER RECOVERY FUNCTIONS
// ============================================================================

/// Detect nodes that have failed (no heartbeat in 60 seconds)
#[hdk_extern]
pub fn detect_failed_nodes(_: ()) -> ExternResult<Vec<String>> {
    // Get all registered nodes
    let custodian_anchor = StringAnchor {
        anchor_type: "custodian".to_string(),
        anchor_value: "available".to_string(),
    };
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;

    let query = LinkQuery::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut failed_nodes = Vec::new();
    let now = sys_time()?;

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(registration) = deserialize_node_registration(&record)? {

                    // Check latest heartbeat
                    match get_latest_heartbeat(&registration.node_id) {
                        Ok(heartbeat) => {
                            let heartbeat_time = parse_timestamp(&heartbeat.timestamp)?;
                            let elapsed = now.as_seconds_and_nanos().0 - heartbeat_time.as_seconds_and_nanos().0;

                            // If no heartbeat in 60 seconds, mark as failed
                            if elapsed > 60 {
                                failed_nodes.push(registration.node_id);
                            }
                        }
                        Err(_) => {
                            // No heartbeat found at all - mark as failed
                            failed_nodes.push(registration.node_id);
                        }
                    }
                }
            }
        }
    }

    Ok(failed_nodes)
}

/// Trigger disaster recovery for a failed node
#[hdk_extern]
pub fn trigger_disaster_recovery(failed_node_id: String) -> ExternResult<Vec<ActionHash>> {
    // Get all content custodied by this node
    let assignments = get_assignments_for_node(failed_node_id.clone())?;

    let mut new_assignments = Vec::new();

    for assignment in assignments {
        // Find replacement custodians
        let filters = CustodianFilters {
            region: Some(assignment.preferred_region.clone().unwrap_or_default()),
            min_storage_gb: Some(assignment.content_size_gb.unwrap_or(1.0)),
            min_bandwidth_mbps: None, // No minimum bandwidth for disaster recovery
            min_tier: Some(assignment.required_tier.clone().unwrap_or("caretaker".to_string())),
            exclude_nodes: Some(vec![failed_node_id.clone()]),
            status: Some("online".to_string()),
        };

        let available_custodians = get_available_custodians(filters)?;

        if available_custodians.is_empty() {
            // Emit signal that recovery failed
            emit_signal(Signal::DisasterRecoveryFailed {
                content_id: assignment.content_id.clone(),
                failed_node_id: failed_node_id.clone(),
                reason: "No available custodians".to_string(),
            })?;
            continue;
        }

        // Store content_id before moving into new assignment
        let content_id_for_lookup = assignment.content_id.clone();

        // Create new assignment with first available custodian
        let new_assignment = CustodianAssignment {
            assignment_id: format!("recovery-{}-{}", assignment.content_id, sys_time()?.as_micros()),
            content_id: assignment.content_id,
            content_hash: assignment.content_hash,
            custodian_node_id: available_custodians[0].node_id.clone(),
            strategy: assignment.strategy,
            shard_index: assignment.shard_index,
            preferred_region: assignment.preferred_region,
            required_tier: assignment.required_tier,
            content_size_gb: assignment.content_size_gb,
            decided_by: "disaster_recovery_daemon".to_string(),
            decision_round: assignment.decision_round.map(|r| r + 1).or(Some(1)),
            votes_json: "".to_string(),
            created_at: timestamp_now()?,
            expires_at: calculate_expiration(30)?, // 30 days
        };

        let hash = assign_custodian(new_assignment.clone())?;
        new_assignments.push(hash);

        // Emit signal to trigger actual content transfer
        emit_signal(Signal::ReplicateContent {
            content_id: new_assignment.content_id,
            content_hash: new_assignment.content_hash,
            from_custodians: find_other_custodians(&content_id_for_lookup, &failed_node_id)?,
            to_custodian: new_assignment.custodian_node_id,
            strategy: new_assignment.strategy,
        })?;
    }

    Ok(new_assignments)
}

// ============================================================================
// SHARD ASSIGNMENT FUNCTIONS
// ============================================================================

#[hdk_extern]
pub fn create_shard_assignment(assignment: ShardAssignment) -> ExternResult<ActionHash> {
    let mut assignment = assignment;
    assignment.created_at = timestamp_now()?;
    assignment.updated_at = assignment.created_at.clone();
    
    // Create the assignment entry
    let hash = create_entry(EntryTypes::ShardAssignment(assignment.clone()))?;
    
    // Update self-reference hash
    assignment.assignment_hash = Some(hash.to_string());
    let _ = update_entry(hash.clone(), &EntryTypes::ShardAssignment(assignment.clone()))?;

    // Link from content to assignment
    let content_anchor = StringAnchor {
        anchor_type: "content_hash".to_string(),
        anchor_value: assignment.content_hash.clone(),
    };
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;
    create_link(
        content_anchor_hash,
        hash.clone(),
        LinkTypes::ContentToShardAssignment,
        (),
    )?;

    // Link from custodian to assignment
    let custodian_anchor = StringAnchor {
        anchor_type: "custodian_did".to_string(),
        anchor_value: assignment.custodian_did.clone(),
    };
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;
    create_link(
        custodian_anchor_hash,
        hash.clone(),
        LinkTypes::CustodianToShardAssignment,
        (),
    )?;

    // Link from content:shard_index to assignment
    let shard_index_anchor = StringAnchor {
        anchor_type: "content_hash:shard_index".to_string(),
        anchor_value: format!("{}:{}", assignment.content_hash, assignment.shard_index),
    };
    let shard_index_anchor_hash = hash_entry(&EntryTypes::StringAnchor(shard_index_anchor))?;
    create_link(
        shard_index_anchor_hash,
        hash.clone(),
        LinkTypes::ShardIndexToAssignment,
        (),
    )?;

    // Emit signal for projection
    emit_signal(Signal::ShardAssignmentCommitted {
        content_hash: assignment.content_hash,
        shard_index: assignment.shard_index,
        custodian_did: assignment.custodian_did,
        status: assignment.status,
    })?;

    Ok(hash)
}

#[hdk_extern]
pub fn get_shard_assignments_for_content(content_hash: String) -> ExternResult<Vec<ShardAssignment>> {
    let content_anchor = StringAnchor {
        anchor_type: "content_hash".to_string(),
        anchor_value: content_hash,
    };
    let content_anchor_hash = hash_entry(&EntryTypes::StringAnchor(content_anchor))?;

    let query = LinkQuery::try_new(content_anchor_hash, LinkTypes::ContentToShardAssignment)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut assignments = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(assignment) = deserialize_shard_assignment(&record)? {
                    assignments.push(assignment);
                }
            }
        }
    }

    Ok(assignments)
}

#[hdk_extern]
pub fn get_shard_assignments_for_custodian(custodian_did: String) -> ExternResult<Vec<ShardAssignment>> {
    let custodian_anchor = StringAnchor {
        anchor_type: "custodian_did".to_string(),
        anchor_value: custodian_did,
    };
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;

    let query = LinkQuery::try_new(custodian_anchor_hash, LinkTypes::CustodianToShardAssignment)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut assignments = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(assignment) = deserialize_shard_assignment(&record)? {
                    assignments.push(assignment);
                }
            }
        }
    }

    Ok(assignments)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateShardStatusInput {
    pub assignment_hash: ActionHash,
    pub new_status: ShardStatus,
}

#[hdk_extern]
pub fn update_shard_status(input: UpdateShardStatusInput) -> ExternResult<ActionHash> {
    if let Some(record) = get(input.assignment_hash.clone(), GetOptions::default())? {
        if let Some(mut assignment) = deserialize_shard_assignment(&record)? {
            assignment.status = input.new_status;
            assignment.updated_at = timestamp_now()?;
            return update_entry(input.assignment_hash, &EntryTypes::ShardAssignment(assignment));
        }
    }
    Err(wasm_error!(WasmErrorInner::Guest("Assignment not found".to_string())))
}

#[hdk_extern]
pub fn update_shard_verified_at(assignment_hash: ActionHash) -> ExternResult<ActionHash> {
    if let Some(record) = get(assignment_hash.clone(), GetOptions::default())? {
        if let Some(mut assignment) = deserialize_shard_assignment(&record)? {
            assignment.verified_at = Some(timestamp_now()?);
            assignment.updated_at = timestamp_now()?;
            return update_entry(assignment_hash, &EntryTypes::ShardAssignment(assignment));
        }
    }
    Err(wasm_error!(WasmErrorInner::Guest("Assignment not found".to_string())))
}

// ============================================================================
// SIGNAL DEFINITIONS
// ============================================================================

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    DisasterRecoveryFailed {
        content_id: String,
        failed_node_id: String,
        reason: String,
    },
    ReplicateContent {
        content_id: String,
        content_hash: String,
        from_custodians: Vec<String>,
        to_custodian: String,
        strategy: String,
    },
    ShardAssignmentCommitted {
        content_hash: String,
        shard_index: u32,
        custodian_did: String,
        status: ShardStatus,
    },
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// HDK 0.6 entry deserialization: `record.entry().to_app_option::<T>()` unwraps
// the `Entry::App(AppEntryBytes)` variant and msgpack-decodes the inner bytes
// as `T`. The previous `entry.try_into() -> SerializedBytes -> T` round-trip
// serialized the whole `Entry` enum (including the `App(...)` variant tag),
// producing bytes that deserialized as a tuple/enum, not as `T` — which
// surfaced as `missing field `node_id`` once node_registry sweettests began
// exercising `get_nodes_by_region`. Other DNAs (mishpat, imagodei) already
// use this pattern.

/// Deserialize a NodeRegistration from a Record
fn deserialize_node_registration(record: &Record) -> ExternResult<Option<NodeRegistration>> {
    record.entry().to_app_option::<NodeRegistration>().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
    })
}

/// Deserialize a NodeHeartbeat from a Record
fn deserialize_node_heartbeat(record: &Record) -> ExternResult<Option<NodeHeartbeat>> {
    record.entry().to_app_option::<NodeHeartbeat>().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
    })
}

/// Deserialize a HealthAttestation from a Record
fn deserialize_health_attestation(record: &Record) -> ExternResult<Option<HealthAttestation>> {
    record.entry().to_app_option::<HealthAttestation>().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
    })
}

/// Deserialize a CustodianAssignment from a Record
fn deserialize_custodian_assignment(record: &Record) -> ExternResult<Option<CustodianAssignment>> {
    record.entry().to_app_option::<CustodianAssignment>().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
    })
}

/// Deserialize a ShardAssignment from a Record
fn deserialize_shard_assignment(record: &Record) -> ExternResult<Option<ShardAssignment>> {
    record.entry().to_app_option::<ShardAssignment>().map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
    })
}

fn get_node_registration_by_id(node_id: String) -> ExternResult<NodeRegistration> {
    let id_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id.clone(),
    };
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToNodeRegistration)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Node registration not found: {}", node_id)
        )));
    }

    if let Some(action_hash) = links[0].target.clone().into_action_hash() {
        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Some(registration) = deserialize_node_registration(&record)? {
                return Ok(registration);
            }
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(
        format!("Failed to retrieve node registration: {}", node_id)
    )))
}

fn get_node_registration_hash(node_id: &str) -> ExternResult<ActionHash> {
    let id_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id.to_string(),
    };
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToNodeRegistration)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Node registration not found: {}", node_id)
        )));
    }

    links[0].target.clone().into_action_hash().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string()))
    })
}

fn get_node_by_agent(agent_key: AgentPubKey) -> ExternResult<NodeRegistration> {
    // Search through all nodes to find one with matching agent_pub_key
    let custodian_anchor = StringAnchor {
        anchor_type: "custodian".to_string(),
        anchor_value: "available".to_string(),
    };
    let custodian_anchor_hash = hash_entry(&EntryTypes::StringAnchor(custodian_anchor))?;

    let query = LinkQuery::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(registration) = deserialize_node_registration(&record)? {
                    if registration.agent_pub_key == agent_key.to_string() {
                        return Ok(registration);
                    }
                }
            }
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(
        "Node not found for this agent".to_string()
    )))
}

fn get_latest_heartbeat(node_id: &str) -> ExternResult<NodeHeartbeat> {
    let node_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id.to_string(),
    };
    let node_anchor_hash = hash_entry(&EntryTypes::StringAnchor(node_anchor))?;

    let query = LinkQuery::try_new(node_anchor_hash, LinkTypes::NodeToHeartbeat)?;
    let links = get_links(query, GetStrategy::default())?;

    if links.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("No heartbeats found for node: {}", node_id)
        )));
    }

    // Get the most recent heartbeat (links are ordered by creation time)
    let latest_link = &links[links.len() - 1];

    if let Some(action_hash) = latest_link.target.clone().into_action_hash() {
        if let Some(record) = get(action_hash, GetOptions::default())? {
            if let Some(heartbeat) = deserialize_node_heartbeat(&record)? {
                return Ok(heartbeat);
            }
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(
        format!("Failed to retrieve heartbeat for node: {}", node_id)
    )))
}

fn get_recent_attestations(node_id: &str, max_age_seconds: i64) -> ExternResult<Vec<HealthAttestation>> {
    let subject_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id.to_string(),
    };
    let subject_anchor_hash = hash_entry(&EntryTypes::StringAnchor(subject_anchor))?;

    let query = LinkQuery::try_new(subject_anchor_hash, LinkTypes::NodeToAttestations)?;
    let links = get_links(query, GetStrategy::default())?;

    let now = sys_time()?;
    let mut recent_attestations = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(attestation) = deserialize_health_attestation(&record)? {

                    let attestation_time = parse_timestamp(&attestation.timestamp)?;
                    let elapsed = now.as_seconds_and_nanos().0 - attestation_time.as_seconds_and_nanos().0;

                    if elapsed <= max_age_seconds {
                        recent_attestations.push(attestation);
                    }
                }
            }
        }
    }

    Ok(recent_attestations)
}

fn meets_tier_requirement(node_tier: &str, min_tier: &str) -> bool {
    let tier_levels = vec!["caretaker", "guardian", "steward", "pioneer"];
    let node_level = tier_levels.iter().position(|&t| t == node_tier).unwrap_or(0);
    let min_level = tier_levels.iter().position(|&t| t == min_tier).unwrap_or(0);
    node_level >= min_level
}

fn find_other_custodians(content_id: &str, exclude_node: &str) -> ExternResult<Vec<String>> {
    let assignments = get_assignments_for_content(content_id.to_string())?;
    Ok(assignments
        .into_iter()
        .filter(|a| a.custodian_node_id != exclude_node)
        .map(|a| a.custodian_node_id)
        .collect())
}

fn timestamp_now() -> ExternResult<String> {
    let now = sys_time()?;
    Ok(format!("{}", now.as_micros()))
}

fn parse_timestamp(timestamp_str: &str) -> ExternResult<Timestamp> {
    let micros: i64 = timestamp_str.parse().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest("Invalid timestamp format".to_string()))
    })?;
    Ok(Timestamp::from_micros(micros))
}

fn calculate_expiration(days: i64) -> ExternResult<String> {
    let now = sys_time()?;
    let expiration = now.as_micros() + (days * 24 * 60 * 60 * 1_000_000);
    Ok(format!("{}", expiration))
}

// ============================================================================
// LINEAGE / MIGRATION HELPERS (Holochain Evolution Epic — probes A & B)
//
// Spec: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md §2
//
// These three externs are DNA-hash-NEUTRAL: they live in the coordinator zome,
// which is not covered by the DNA hash (integrity zomes + modifiers only).
// They exist so a v1 cell can hand its own notarizations (action + signature)
// to a v2 cell, and so a chain-switch can be driven from a test/agent.
// ============================================================================

/// Read one of THIS agent's own actions off the local source chain, signed.
///
/// Local-only by construction (`query`), so it never waits on the network and
/// is deterministic in a single-conductor test. Returns `None` when the hash
/// is not on this chain.
#[hdk_extern]
pub fn get_signed_action(action_hash: ActionHash) -> ExternResult<Option<SignedActionHashed>> {
    let records = query(ChainQueryFilter::new().include_entries(false))?;
    Ok(records
        .into_iter()
        .find(|r| r.action_address() == &action_hash)
        .map(|r| r.signed_action))
}

/// Input to [`export_records`]: an opaque page cursor (`None` starts at the
/// beginning of the app-entry portion of the chain) and a page size, capped
/// at [`EXPORT_CAP`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportInput {
    pub cursor: Option<u32>,
    pub limit: u32,
}

/// One bounded, cursor-resumable page of app-entry records, plus a chain digest
/// that is the SAME on every page (computed once over the whole walk, not the
/// page) — Task 7 compares it to the carry receipt.
///
/// **WHOSE chain, and how completely, depends on which export produced it.**
/// [`export_records`] walks the calling agent's OWN source chain with a local
/// `query()`: the walk is complete by construction, so `digest`/`total` describe
/// the whole chain. [`export_held_records`] walks a NEIGHBOUR's chain as this
/// peer's own agent-activity store has it — the COURIER'S VIEW, which is a
/// subset of the neighbour's real chain and may be gapped, because gossip is
/// asynchronous and this peer holds only what it has validated and integrated.
/// Every field below is scoped to that walk, never to a truth the courier
/// cannot see. See [`ExportPage::observed_head`] for the one field that reaches
/// past the courier's view.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportPage {
    pub records: Vec<SignedActionHashed>,
    pub entries: Vec<Option<Entry>>,
    /// Where to resume, or `None` when THIS WALK is exhausted.
    ///
    /// On the own-chain path (`export_records`) that is end-of-chain. On the
    /// held path (`export_held_records`) it is only END-OF-LOCAL-VIEW: the
    /// courier has no more records it holds, which is not the same claim as
    /// "the neighbour has no more records". A driver that reads `None` as
    /// chain-complete on a held page will silently truncate a neighbour whose
    /// tail has not gossiped here yet — compare `total` against
    /// `observed_head`, or against the neighbour's own export, before
    /// concluding completeness.
    pub next_cursor: Option<u32>,
    pub digest: String,
    /// The count of app-entry records THIS WALK covers — the same on every
    /// page, like `digest`. A carry receipt reports it verbatim so Station 3's
    /// `carried == v1_count` equality is falsifiable; deriving it from
    /// `carried` would make the check tautological.
    ///
    /// On the own-chain path this is the whole chain. On the held path it is
    /// the courier's own integrated view of the neighbour's chain — a subset,
    /// possibly gapped. **A held page is therefore never self-evidencing**:
    /// `carried == total` on a held receipt says the courier carried everything
    /// it had, not everything the neighbour had.
    ///
    /// `Option` + `#[serde(default)]` is additive by construction: a caller
    /// holding an older `ExportPage` shape still decodes, and a bundle packed
    /// before this field existed decodes to `None` rather than a fabricated 0.
    #[serde(default)]
    pub total: Option<u32>,
    /// **Additive.** The highest action SEQUENCE observed for this chain — the
    /// one field that reaches past the courier's own view, and so the only way
    /// a held page can be checked for truncation from inside itself.
    ///
    /// On the held path this is `AgentActivityStatus::highest_observed`
    /// (holochain 0.7 exposes it as `Option<HighestObserved>` and computes it
    /// BEFORE the entry-type filter, from actions the authority has *seen* but
    /// not necessarily validated or integrated). On the own-chain path it is
    /// this agent's own chain-head sequence.
    ///
    /// It is a SEQUENCE, not a count, and it spans every action including
    /// genesis, `InitZomesComplete` and links — so `observed_head >= total - 1`
    /// always, and the two are equal only on a chain that is nothing but app
    /// entries. The gap is normally LARGE and means nothing on its own:
    /// MEASURED on this DNA, five `register_node` calls give `total: 5` against
    /// `observed_head: 33`, because each call commits six actions (one Create
    /// plus five CreateLinks) above three genesis actions and an
    /// `InitZomesComplete`.
    ///
    /// So a driver must not read the distance as staleness. What this field IS
    /// good for is comparison ACROSS peers: two views of the same chain that
    /// report different `observed_head`s disagree about how far that chain has
    /// got, and the lower one is a courier that has not caught up. Compare
    /// against the neighbour's own export, or across successive sweeps — never
    /// against `total`.
    ///
    /// `Option` + `#[serde(default)]`: `None` from a bundle packed before this
    /// field existed, and `None` when the authority reported no observation at
    /// all — never a fabricated 0, which would read as "chain head at genesis".
    #[serde(default)]
    pub observed_head: Option<u32>,
}

const EXPORT_CAP: u32 = 64;

/// The chain digest every export reports: a hex sha256 over the concatenated
/// raw action hashes, in chain order.
///
/// Factored out ON PURPOSE. `export_records` (own chain, local `query`) and
/// `export_held_records` (a neighbour's chain, via the agent-activity
/// authority) MUST hash the same way, because the carry receipt's `v1_digest`
/// is a like-for-like comparison against whichever of the two the sweep called.
/// Two copies of this loop would let the two exports drift silently and turn
/// that comparison into a tautology.
fn chain_digest<'a, I>(hashes: I) -> String
where
    I: IntoIterator<Item = &'a ActionHash>,
{
    let mut hasher = Sha256::new();
    for h in hashes {
        hasher.update(h.get_raw_39());
    }
    hex::encode(hasher.finalize())
}

/// v1 bounded export (Holochain Evolution Epic Task 1). Exports THIS agent's
/// own chain actions of app entry types — genesis actions, capability
/// grants, and agent-validation packages are conductor bookkeeping, not
/// facts to carry — ordered by `action_seq`, paged by an opaque numeric
/// cursor, at most `EXPORT_CAP` per page. `digest` is a hex sha256 over the
/// concatenated action hashes of the WHOLE chain (page-independent: every
/// page of the same chain returns the same digest).
///
/// Local-only by construction (`query`), so it is deterministic and never
/// waits on the network — the same property `get_signed_action` relies on.
#[hdk_extern]
pub fn export_records(input: ExportInput) -> ExternResult<ExportPage> {
    let limit = input.limit.clamp(1, EXPORT_CAP) as usize;
    let all = query(ChainQueryFilter::new().include_entries(true))?;
    // Read the chain head BEFORE the app-entry filter: `observed_head` is a
    // sequence over every action, so that the held path (which reads it from
    // the authority's pre-filter `highest_observed`) means the same thing.
    let observed_head = all.iter().map(|r| r.action().action_seq()).max();
    let mut app: Vec<Record> = all
        .into_iter()
        .filter(|r| matches!(r.action().entry_type(), Some(EntryType::App(_))))
        .collect();
    app.sort_by_key(|r| r.action().action_seq());

    let digest = chain_digest(app.iter().map(|r| r.action_address()));
    // Cheap: the walk above already read the whole chain.
    let total = Some(app.len() as u32);

    let start = input.cursor.unwrap_or(0) as usize;
    let page: Vec<Record> = app.into_iter().skip(start).take(limit).collect();
    let next_cursor = if page.len() == limit {
        Some((start + limit) as u32)
    } else {
        None
    };

    Ok(ExportPage {
        entries: page.iter().map(|r| r.entry().as_option().cloned()).collect(),
        records: page.into_iter().map(|r| r.signed_action).collect(),
        next_cursor,
        digest,
        total,
        observed_head,
    })
}

/// Input to [`export_held_records`]: WHOSE chain to read, plus the same page
/// cursor/limit discipline as [`ExportInput`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportHeldInput {
    /// The neighbour whose chain this cell should hand over. Not necessarily
    /// (and usually not) the calling agent.
    pub agent: AgentPubKey,
    pub cursor: Option<u32>,
    pub limit: u32,
}

/// The app entry types in scope for THIS zome, rendered as an exact
/// [`ChainQueryFilter`] entry-type set.
///
/// `export_records` selects app entries with `matches!(entry_type, Some(App(_)))`
/// — a predicate it can evaluate because it holds whole `Record`s.
/// `export_held_records` holds only `(seq, ActionHash)` pairs, so the same
/// selection has to be expressed as a filter the agent-activity AUTHORITY
/// applies on its side (`ChainQueryFilter::filter_actions`, exact `EntryType`
/// equality). Enumerating every scoped `(zome_index, entry_index)` at both
/// visibilities reproduces the predicate exactly for this DNA, whose one
/// integrity zome defines every app entry type there is.
///
/// Derived at runtime from `zome_info()`, so the `lineage-witness` build's extra
/// entry type is covered without this function knowing about it.
fn app_entry_types_in_scope() -> ExternResult<Vec<EntryType>> {
    let scoped = zome_info()?.zome_types.entries;
    let mut types = Vec::new();
    for (zome_index, entry_indexes) in scoped.0.iter() {
        for entry_index in entry_indexes {
            for visibility in [EntryVisibility::Public, EntryVisibility::Private] {
                types.push(EntryType::App(AppEntryDef::new(
                    *entry_index,
                    *zome_index,
                    visibility,
                )));
            }
        }
    }
    Ok(types)
}

/// v1 bounded export of a NEIGHBOUR's chain (Holochain Evolution Epic Task 18).
///
/// Why this exists. [`export_records`] is a local `query()` — it can only ever
/// hand back the calling agent's OWN chain, so every record a sweep pulled
/// through it was a self-carry and v2's held-carry branch (§2.2) was
/// unreachable in practice (Station 5's live measurement). This extern is the
/// missing HELD view: it reads `agent`'s chain through the agent-activity
/// authority and fetches each record of the page window from the DHT, returning
/// the SAME [`ExportPage`] shape so `carry_from` can consume either without
/// caring which it asked for.
///
/// Bounded by construction: `get_agent_activity` returns hashes only (cheap,
/// one round trip), and at most `limit.clamp(1, EXPORT_CAP)` records are then
/// fetched. `digest` and `total` cover the whole filtered valid activity and
/// are page-independent, exactly as in `export_records` — and are computed with
/// the same [`chain_digest`], so a `v1_digest` comparison is like-for-like
/// whichever export produced it.
///
/// **What "whole" means here is the COURIER'S VIEW, not the neighbour's chain.**
/// This reads THIS peer's own agent-activity store, so `total`, `digest` and
/// `next_cursor` all describe a subset of `agent`'s real chain — possibly a
/// gapped one, since gossip is asynchronous. A page from this extern is
/// therefore never self-evidencing: `next_cursor: None` means end-of-local-view,
/// and a receipt's `carried == total` says the courier carried everything it
/// had. [`ExportPage::observed_head`] is the one field that reaches past that
/// view and lets a driver notice it is behind.
///
/// A record the authority named but that cannot be fetched is a LOUD Guest
/// error naming the action hash. Silently skipping it would produce a page that
/// disagrees with its own digest and total, which the driver would read as a
/// short chain rather than as a fetch failure.
///
/// Coordinator-only, so DNA-hash-NEUTRAL: it hot-swaps onto a running v1
/// conductor with no reinstall and no re-key.
#[hdk_extern]
pub fn export_held_records(input: ExportHeldInput) -> ExternResult<ExportPage> {
    let limit = input.limit.clamp(1, EXPORT_CAP) as usize;

    // Same selection as `export_records`, expressed as an authority-side filter.
    let mut filter = ChainQueryFilter::new();
    for entry_type in app_entry_types_in_scope()? {
        filter = filter.entry_type(entry_type);
    }

    // `GetOptions::local()` — THIS conductor's own authority store, i.e. what
    // this peer validated and integrated. Two reasons, one principled and one
    // measured:
    //
    //   * a courier should carry only what IT holds, never what a remote peer
    //     told it it holds — the same verify-locally-then-serve invariant the
    //     rest of the dataplane keeps;
    //   * MEASURED (holochain 0.7.0, 2026-09-04): a lone conductor reading its
    //     OWN key with `GetOptions::network()` returns an EMPTY activity list
    //     (`total: Some(0)`, digest of the empty string) even 60 s after five
    //     `register_node` calls, while the same read with `local()` returns all
    //     five. `network()` is therefore not a superset of `local()` here, and
    //     `export_held_records(self)` would silently report an empty chain.
    //
    // A peer that has not yet gossiped the neighbour's chain reports a SHORT
    // view rather than a wrong one: `total` and `digest` cover exactly the
    // records this page can also hand over, so a driver comparing them against
    // the neighbour's own export sees the disagreement instead of inheriting it.
    let activity = get_agent_activity(
        input.agent.clone(),
        filter,
        ActivityRequest::Full,
        GetOptions::local(),
    )?;

    // MEASURED (holochain 0.7.0): `highest_observed` is computed by
    // `build_agent_activity_response` from the classified lists BEFORE
    // `filter.filter_actions` runs, so it spans the whole chain the authority
    // has seen — genesis, links and all — not just the app entries this walk
    // returns. That is what makes it a truncation check the page cannot fake:
    // it reaches past the filtered view the rest of this response describes.
    let observed_head = activity.highest_observed.map(|h| h.action_seq);

    // `valid_activity` arrives ascending by sequence (the authority sorts before
    // filtering), which is the order `export_records` establishes with its
    // explicit `sort_by_key(action_seq)`.
    let hashes: Vec<ActionHash> = activity
        .valid_activity
        .into_iter()
        .map(|(_seq, hash)| hash)
        .collect();

    let digest = chain_digest(hashes.iter());
    let total = Some(hashes.len() as u32);

    let start = input.cursor.unwrap_or(0) as usize;
    let window: Vec<ActionHash> = hashes.into_iter().skip(start).take(limit).collect();
    let next_cursor = if window.len() == limit {
        Some((start + limit) as u32)
    } else {
        None
    };

    let mut records = Vec::with_capacity(window.len());
    let mut entries = Vec::with_capacity(window.len());
    for action_hash in window {
        let record = get(action_hash.clone(), GetOptions::network())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "export_held_records: the agent-activity authority named action {action_hash} \
                 for {}, but it could not be fetched — refusing to return a page that \
                 disagrees with its own digest and total",
                input.agent
            )))
        })?;
        entries.push(record.entry().as_option().cloned());
        records.push(record.signed_action);
    }

    Ok(ExportPage {
        records,
        entries,
        next_cursor,
        digest,
        total,
        observed_head,
    })
}

/// The agents whose registrations this cell can see (Holochain Evolution Epic
/// Task 18) — the sweep's enumeration of WHOM to ask for a held export.
///
/// Walks the same index the discovery externs walk: `register_node` links every
/// new registration from the `("status", "online")` anchor, so that anchor is
/// the one enumeration that covers every registration regardless of region or
/// tier.
///
/// The author is read from the LINK, not from a fetched record. `register_node`
/// creates the entry and this link in one zome call, so the link's author IS
/// the registration's author — and taking it from there keeps this extern a
/// single `get_links` instead of one network `get` per registration, which is
/// what makes it usable as the first step of a sweep.
///
/// **THE CALLER'S OWN KEY IS INCLUDED** when this cell has registered a node,
/// and deliberately so: this extern reports who is on the DHT, not who is
/// foreign. A sweep must therefore filter itself out, or route its own key
/// through [`CarrySource::Own`] — passing it to `CarrySource::Held` is refused
/// by `carry_from`, which is the honest signal rather than a silent
/// mis-labelled self-carry.
///
/// Coordinator-only, so DNA-hash-NEUTRAL.
#[hdk_extern]
pub fn known_agents(_: ()) -> ExternResult<Vec<AgentPubKey>> {
    let status_anchor = StringAnchor {
        anchor_type: "status".to_string(),
        anchor_value: "online".to_string(),
    };
    let status_anchor_hash = hash_entry(&EntryTypes::StringAnchor(status_anchor))?;

    let query = LinkQuery::try_new(status_anchor_hash, LinkTypes::StatusToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut agents: Vec<AgentPubKey> = Vec::new();
    for link in links {
        if !agents.contains(&link.author) {
            agents.push(link.author);
        }
    }

    Ok(agents)
}

/// Close this chain toward a successor DNA. Returns the real `CloseChain`
/// action hash, which the successor's `open_chain` must name.
#[hdk_extern]
pub fn close_chain_for(dna_hash: DnaHash) -> ExternResult<ActionHash> {
    close_chain(Some(MigrationTarget::Dna(dna_hash)))
}

/// Record on THIS chain which DNA it migrated from, naming the predecessor's
/// `CloseChain` action hash. Holochain does not constrain when this is
/// committed — probe B measures whether a LATE open_chain is accepted.
#[hdk_extern]
pub fn open_chain_from(input: (DnaHash, ActionHash)) -> ExternResult<ActionHash> {
    let (prev_dna_hash, close_hash) = input;
    open_chain(MigrationTarget::Dna(prev_dna_hash), close_hash)
}

// ============================================================================
// NOTARIZATION CARRYING (Holochain Evolution Epic §2)
//
// Gated behind `lineage-witness` — these externs create/read the gated
// NotarizationWitness entry and EntryToWitness link, so they cannot compile
// (let alone run) when the integrity zome was built without the feature.
// ============================================================================

/// Commit a batch of predecessor notarizations into THIS DNA.
///
/// The integrity zome re-verifies every carried signature against the carried
/// action's `signer()`, and refuses the witness if `lineage_dna_hash` is not
/// declared in this DNA's `lineage` property. On acceptance one link is created
/// per proof, from the carried entry hash to the witness, so a v2 reader can
/// ask "what predecessor notarizations exist for this entry hash?".
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn commit_witness(witness: NotarizationWitness) -> ExternResult<ActionHash> {
    let entry_hashes: Vec<EntryHash> = witness
        .proofs
        .iter()
        .filter_map(|p| p.action.entry_hash().cloned())
        .collect();

    let action_hash = create_entry(EntryTypes::NotarizationWitness(witness))?;

    for entry_hash in entry_hashes {
        create_link(
            entry_hash,
            action_hash.clone(),
            LinkTypes::EntryToWitness,
            (),
        )?;
    }

    Ok(action_hash)
}

/// Shared `EntryToWitness` link lookup — the extern below and the Task 20
/// carry-idempotency check (`entry_already_witnessed`) both read the same
/// index of "which witnesses carry this entry hash".
#[cfg(feature = "lineage-witness")]
fn witnesses_for(entry_hash: EntryHash) -> ExternResult<Vec<Link>> {
    let query = LinkQuery::try_new(entry_hash, LinkTypes::EntryToWitness)?;
    get_links(query, GetStrategy::default())
}

/// Every witness carrying a predecessor notarization for this entry hash.
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn get_witnesses_for(entry_hash: EntryHash) -> ExternResult<Vec<Link>> {
    witnesses_for(entry_hash)
}

/// Whether a [`NotarizationWitness`] already carries `entry_hash` FROM
/// `lineage_dna_hash` — the held-carry idempotency check (Task 20, epic §7
/// C6b). A retried held page must not push a second proof (and thus create a
/// second `EntryToWitness` link) for an entry a previous page already
/// witnessed from the same predecessor DNA; a different lineage witnessing
/// the same entry hash is a distinct claim and must not be treated as a
/// duplicate.
///
/// Reads each candidate witness locally (`GetOptions::local()`) — this chain
/// is checking its OWN prior commits, the same verify-locally-then-serve
/// discipline `export_held_records` documents above.
#[cfg(feature = "lineage-witness")]
fn entry_already_witnessed(
    entry_hash: &EntryHash,
    lineage_dna_hash: &DnaHash,
) -> ExternResult<bool> {
    for link in witnesses_for(entry_hash.clone())? {
        let Some(action_hash) = link.target.into_action_hash() else {
            continue;
        };
        let Some(record) = get(action_hash, GetOptions::local())? else {
            continue;
        };
        let witness = record.entry().to_app_option::<NotarizationWitness>().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "entry_already_witnessed: could not decode NotarizationWitness at {}: {e:?}",
                record.action_address()
            )))
        })?;
        if let Some(witness) = witness {
            if &witness.lineage_dna_hash == lineage_dna_hash {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// This agent's own chain activity as the local authority sees it.
///
/// `close_chain` is NOT an authoring-time guard: `ActionAfterChainClose` is
/// enforced in the sys-validation workflow (`register_agent_activity`), i.e. by
/// the agent-activity AUTHORITY, not by the source chain. So a post-close
/// commit still returns an `ActionHash` locally; the refusal shows up here, in
/// `rejected_activity` / `status`. Probe B measures exactly that.
#[hdk_extern]
pub fn my_chain_activity(_: ()) -> ExternResult<AgentActivityStatus> {
    let agent = agent_info()?.agent_initial_pubkey;
    get_agent_activity(
        agent,
        ChainQueryFilter::new(),
        ActivityRequest::Full,
        GetOptions::local(),
    )
}

/// ANOTHER agent's chain activity, as THIS conductor sees it.
///
/// Probe B measured the single-conductor case, where the author is its own
/// agent-activity authority. Probe B2 asks the two-conductor question: does a
/// REMOTE authority apply `ActionAfterChainClose` to post-close activity?
/// `my_chain_activity` cannot express it — it always targets `agent_info()`.
///
/// `local_only` selects WHOSE verdict is read:
///   * `true`  → `GetOptions::local()` — this conductor's own authority store,
///     i.e. what THIS peer validated and integrated. That is the measurement.
///   * `false` → `GetOptions::network()` — ask the network, which may answer
///     from the author's own (self-authoring) authority. Reported alongside so
///     the two views can be told apart.
#[hdk_extern]
pub fn agent_activity_of(input: (AgentPubKey, bool)) -> ExternResult<AgentActivityStatus> {
    let (agent, local_only) = input;
    let options = if local_only {
        GetOptions::local()
    } else {
        GetOptions::network()
    };
    get_agent_activity(agent, ChainQueryFilter::new(), ActivityRequest::Full, options)
}

/// `get` one record by action hash, with the local/network choice explicit.
///
/// Probe B2's second question: after the author's chain is closed and a
/// post-close action is authored anyway, can a peer still RETRIEVE it? A
/// network `get` may be answered by the author itself, so the local read is
/// reported beside it.
#[hdk_extern]
pub fn get_record_at(input: (ActionHash, bool)) -> ExternResult<Option<Record>> {
    let (action_hash, local_only) = input;
    let options = if local_only {
        GetOptions::local()
    } else {
        GetOptions::network()
    };
    get(action_hash, options)
}

// ============================================================================
// BOUNDED CROSS-CELL CARRY (Holochain Evolution Epic Task 9, spec §2/§8)
//
// Gated behind `lineage-witness`, and APPENDED at the end of the file rather
// than interleaved with the section above, for the same reason the integrity
// zome appends its gated section: the default build's compiled output must not
// be perturbed by this section's mere physical presence in the source text.
//
// `carry_from` is the whole crossing in one bounded step: it asks the
// PREDECESSOR cell for one page of its own export, re-creates this agent's own
// records natively on THIS chain (so the entry hash is preserved and v2 holds
// the content as its own commit, not merely as bytes inside a witness), and
// commits ONE witness carrying the page's predecessor notarizations. Records
// authored by somebody else cannot be re-created natively — they are carried
// as held-carries (§2.2), entry bytes included, so v2's validator can still
// check that the carried entry is the one the carried action commits to.
// ============================================================================

/// WHOSE predecessor records a [`carry_from`] page should pull.
///
/// `Own` reads the predecessor cell's own chain through `export_records` — the
/// self-carry path (§2.1), where the carrier is the author and re-creates the
/// content natively.
///
/// `Held(agent)` reads a NEIGHBOUR's chain through `export_held_records` — the
/// held-carry path (§2.2), where the carrier is a COURIER: it authors the
/// witness, but never the carried content.
#[cfg(feature = "lineage-witness")]
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub enum CarrySource {
    /// The predecessor cell's own chain (the pre-Task-18 behaviour, and the
    /// `#[serde(default)]` a request that omits `source` decodes to).
    #[default]
    Own,
    /// A neighbour's chain, as the predecessor cell can see it.
    Held(AgentPubKey),
}

/// Input to [`carry_from`]: which predecessor cell to pull from, where to
/// resume, and how many records to take. `limit` is clamped to
/// [`WITNESS_BATCH`] because one page becomes exactly one witness, and a
/// witness may carry at most that many proofs.
#[cfg(feature = "lineage-witness")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarryInput {
    pub v1_cell: CellId,
    pub cursor: Option<u32>,
    pub limit: u32,
    /// **Additive.** Whose records to carry, defaulting to [`CarrySource::Own`].
    ///
    /// `#[serde(default)]` is what keeps this byte-compatible: the landed
    /// storage decoder (`elohim-storage/.../release_adoption/apply.rs`) emits no
    /// `source` key at all, and `holochain_serialized_bytes` encodes structs as
    /// NAMED msgpack maps, so an absent key reads as `Own` — exactly the
    /// behaviour that driver already relies on.
    #[serde(default)]
    pub source: CarrySource,
}

/// What one page of carriage produced.
///
/// **Scope follows [`CarryInput::source`].** On [`CarrySource::Own`] every field
/// below describes the predecessor's whole chain, because `export_records`
/// walked it locally and completely. On [`CarrySource::Held`] they describe the
/// COURIER'S VIEW of the neighbour's chain — what the predecessor cell had
/// validated and integrated at the moment it answered, which is a subset and may
/// be gapped. A held receipt is therefore **never self-evidencing**: no
/// combination of its own fields proves the neighbour's chain was carried whole.
/// [`CarryReceipt::v1_observed_head`] is the one number that reaches past that
/// view.
#[cfg(feature = "lineage-witness")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CarryReceipt {
    /// How many predecessor records this page carried. On a first pass this
    /// equals the witness's proof count; on a RETRY it also counts records
    /// this chain already carried on an earlier pass (Task 20, epic §7 C6b) —
    /// those are still `carried` (the content IS here), but pushed no proof
    /// and authored no witness a second time. `carried == self.already_carried
    /// + <the witness's proof count>` always holds.
    pub carried: u32,
    /// Resume token for the next page, or `None` when the export THIS PAGE
    /// DREW FROM is exhausted. The caller drives the loop.
    ///
    /// On the `Own` path that is end-of-chain. On the `Held` path it is only
    /// END-OF-LOCAL-VIEW — the predecessor cell has no further records of the
    /// neighbour that it holds, which is not a claim about the neighbour's
    /// chain. A driver must not read `None` on a held page as "carried whole".
    pub next_cursor: Option<u32>,
    /// The digest of the chain walk this page drew from, reported verbatim —
    /// the same on every page, so a driver can check that it carried from ONE
    /// chain. On the `Held` path it digests the courier's view, so two peers
    /// carrying the same neighbour at different catch-up points legitimately
    /// report different digests; that is a staleness signal, not a fork.
    pub v1_digest: String,
    /// The witness committed for this page, rendered as canonical base64
    /// (`HoloHash`'s `Display`), or the EMPTY STRING for a page that authored
    /// no witness.
    ///
    /// A `String` and not an `ActionHash` on purpose. `HoloHash` serialises to
    /// a msgpack BYTE ARRAY, and the landed consumer
    /// (`elohim-storage/src/services/release_adoption/apply.rs`, Task 8)
    /// decodes this field as a `String` — returning the native hash here fails
    /// with "invalid type: byte array, expected a string", the signal-decode
    /// class that has bitten this codebase repeatedly. The zome renders; the
    /// storage side never re-derives a hash.
    pub witness_hash: String,
    /// The app-record count of the walk this page drew from, READ from its
    /// [`ExportPage::total`] — never derived from `carried`, so the driver's
    /// `sum(carried) == v1_total` check can actually fail. `None` when the
    /// predecessor bundle predates that field.
    ///
    /// On the `Own` path this is the predecessor's whole chain. On the `Held`
    /// path it is the courier's own integrated view of the neighbour's chain —
    /// so `sum(carried) == v1_total` there says everything the courier HAD of
    /// the neighbour was carried, never everything the neighbour had. Station
    /// 5/6 assertions on a held sweep must be worded that way.
    pub v1_total: Option<u32>,
    /// **Additive.** How many of `carried` were re-created NATIVELY on this
    /// chain (self-carry, §2.1) — held-carries are excluded. `carried` alone
    /// cannot tell the two apart, and only the self-carried share is content
    /// this DNA now holds as its own commits rather than as bytes inside a
    /// witness.
    ///
    /// `#[serde(default)]` so a consumer built against the pre-hardening
    /// receipt still decodes, and a page from a zome that predates the field
    /// reads as 0 rather than failing.
    #[serde(default)]
    pub self_carried: u32,
    /// **Additive.** The highest action SEQUENCE the predecessor observed for
    /// the chain it exported, READ from [`ExportPage::observed_head`] — the one
    /// number in this receipt that reaches past the courier's own view, and so
    /// the only way a held receipt can be checked for truncation at all.
    ///
    /// It is a sequence spanning every action (genesis, `InitZomesComplete`,
    /// links and app entries alike), so `v1_observed_head >= v1_total - 1`
    /// always and the gap is normally large — see [`ExportPage::observed_head`]
    /// for the measured 33-against-5. **Do not read the distance from
    /// `v1_total` as staleness.** The usable comparison is across VIEWS: a held
    /// sweep whose `v1_observed_head` sits below what another peer reports for
    /// the same chain is a courier that has not caught up, and should be
    /// re-swept rather than recorded as a complete crossing.
    ///
    /// `#[serde(default)]` and `None` from a page that predates the field, or
    /// from an authority that reported no observation — never a fabricated 0,
    /// which would read as "chain head at genesis".
    ///
    /// A multi-page sweep keeps the LAST NON-`None` value: an intermediate page
    /// answered by a momentarily blind authority must not erase what an earlier
    /// page established.
    #[serde(default)]
    pub v1_observed_head: Option<u32>,
    /// **Additive.** How many of `carried` were ALREADY carried before this
    /// page ran — a self-carry whose entry hash was already found on this
    /// chain (one `query`, checked before `create_entry`), or a held-carry
    /// whose entry hash already had a `NotarizationWitness` from this
    /// lineage (`get_witnesses_for`). Task 20, epic §7 C6b: a retried page
    /// re-creates nothing and authors no second witness for these — they
    /// contribute to `carried` but not to `self_carried` and get no proof in
    /// the page's witness. `already_carried == carried` on a page whose
    /// every record was already carried means the page authored NO witness
    /// this time (`witness_hash == ""`).
    ///
    /// `#[serde(default)]` so a consumer built against the pre-idempotency
    /// receipt still decodes, reading 0 for a page from a zome that predates
    /// the field.
    #[serde(default)]
    pub already_carried: u32,
}

/// Pull ONE bounded page from a predecessor cell and witness it here.
///
/// The predecessor DNA is read from THIS DNA's own `lineage` property (which
/// folds into the DNA hash, so every peer agrees on it) — `v1_cell` must name
/// that DNA, otherwise the carriage is refused here rather than deeper in
/// validation. MVP scope: the FIRST declared parent; a multi-parent lineage is
/// a later station.
///
/// [`CarryInput::source`] chooses WHOSE records the page holds:
/// [`CarrySource::Own`] (the default, and what a request omitting the field
/// decodes to) takes the predecessor cell's own chain and re-creates this
/// agent's records natively; [`CarrySource::Held`] takes a neighbour's chain
/// through `export_held_records` and carries every record as a held-carry —
/// entry bytes inside the witness, `self_carried` untouched, this agent as
/// courier.
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn carry_from(input: CarryInput) -> ExternResult<CarryReceipt> {
    // (1) the declared predecessor, from this DNA's identity-bearing properties.
    let properties: LineageProperties =
        dna_info()?.modifiers.properties.try_into().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "carry_from: could not deserialize DNA properties: {e:?}"
            )))
        })?;
    let lineage_dna_hash = properties.lineage.first().cloned().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "carry_from: this DNA declares no lineage — there is no predecessor to carry from"
                .to_string(),
        ))
    })?;
    if input.v1_cell.dna_hash() != &lineage_dna_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "carry_from: v1_cell names DNA {}, but this DNA declares {} as its predecessor",
            input.v1_cell.dna_hash(),
            lineage_dna_hash
        ))));
    }

    // (2) one bounded page of the predecessor's export, across the cell
    //     boundary. Same agent, so no capability secret is presented.
    //
    //     WHICH export depends on `source`: the predecessor's own chain
    //     (`export_records`, self-carry) or a neighbour's chain as the
    //     predecessor can see it (`export_held_records`, held-carry). Both
    //     return the same `ExportPage` shape, so only the call differs.
    let me = agent_info()?.agent_initial_pubkey;
    let limit = input.limit.clamp(1, WITNESS_BATCH as u32);
    // Held pages are ALWAYS held-carries: the courier is not the author, so no
    // record on the page may be re-created natively on this chain.
    let held = matches!(input.source, CarrySource::Held(_));
    let extern_name = if held {
        "export_held_records"
    } else {
        "export_records"
    };

    let response = match &input.source {
        CarrySource::Own => call(
            CallTargetCell::OtherCell(input.v1_cell.clone()),
            "node_registry_coordinator",
            "export_records".into(),
            None,
            ExportInput {
                cursor: input.cursor,
                limit,
            },
        )?,
        CarrySource::Held(agent) => {
            // A held-carry of one's OWN chain would be a self-carry wearing the
            // wrong label: the loop below would ship the entry bytes inside the
            // witness and report `self_carried: 0` for content this agent could
            // have re-created natively. Refuse rather than quietly downgrade.
            if agent == &me {
                return Err(wasm_error!(WasmErrorInner::Guest(
                    "carry_from: Held(self) is not a held-carry — use CarrySource::Own to \
                     re-create this agent's own records natively"
                        .to_string(),
                )));
            }
            call(
                CallTargetCell::OtherCell(input.v1_cell.clone()),
                "node_registry_coordinator",
                "export_held_records".into(),
                None,
                ExportHeldInput {
                    agent: agent.clone(),
                    cursor: input.cursor,
                    limit,
                },
            )?
        }
    };
    let page: ExportPage = match response {
        ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "carry_from: could not decode the predecessor's ExportPage: {e:?}"
            )))
        })?,
        other => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "carry_from: the predecessor cell refused {extern_name}: {other:?}"
            ))))
        }
    };

    // `records` and `entries` are paired POSITIONALLY, so a page whose two
    // vectors disagree in length cannot be paired at all — index i would mean a
    // different record in each. Refuse the page rather than silently carrying
    // the wrong bytes (or `None`) alongside an action.
    if page.records.len() != page.entries.len() {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "carry_from: the predecessor returned {} records but {} entries — the page cannot \
             be paired positionally",
            page.records.len(),
            page.entries.len()
        ))));
    }

    // (3) self-carry vs held-carry, per §2.1/§2.2.
    //
    // Idempotency (Task 20, epic §7 C6b). A retried page must not re-create a
    // self-carry that already landed, nor commit a second witness proof for
    // an entry that is already carried — either would leave TWO actions
    // sharing one entry hash on this chain, or TWO `EntryToWitness` links for
    // the same predecessor proof.
    //
    // Self-carry candidates are checked with ONE chain `query` over the
    // whole page's entry hashes, not one query per record — a page's records
    // are a page-bounded batch and the check should cost one round trip, not
    // `page.records.len()` of them.
    let self_carry_candidates: HashSet<EntryHash> = if held {
        HashSet::new()
    } else {
        page.records
            .iter()
            .filter(|signed| signed.action().author() == &me)
            .filter_map(|signed| signed.action().entry_hash().cloned())
            .collect()
    };
    let already_on_chain: HashSet<EntryHash> = if self_carry_candidates.is_empty() {
        HashSet::new()
    } else {
        query(
            ChainQueryFilter::new()
                .entry_hashes(self_carry_candidates)
                .include_entries(false),
        )?
        .iter()
        .filter_map(|r| r.action().entry_hash().cloned())
        .collect()
    };

    let mut proofs: Vec<CarriedProof> = Vec::with_capacity(page.records.len());
    let mut self_carried: u32 = 0;
    let mut already_carried: u32 = 0;

    for (i, signed) in page.records.iter().enumerate() {
        let action = signed.action().clone();
        let carried_entry: Option<Entry> = page.entries.get(i).cloned().flatten();

        // Self-carry, already re-created on this chain by an earlier page: it
        // IS carried, but `create_entry`ing it again would mint a second
        // action over the same entry hash, and pushing a proof for it would
        // commit a second witness. Skip both; count it as already carried.
        if !held && action.author() == &me {
            if let Some(entry_hash) = action.entry_hash() {
                if already_on_chain.contains(entry_hash) {
                    already_carried = already_carried.saturating_add(1);
                    continue;
                }
            }
        }

        // Held-carry, already witnessed FROM THIS LINEAGE by an earlier page:
        // a `NotarizationWitness` already carries a proof for this entry hash
        // naming this same predecessor DNA. Skip re-proving it — a second
        // proof would commit a second witness and a second `EntryToWitness`
        // link for content already witnessed.
        if held {
            if let Some(entry_hash) = action.entry_hash() {
                if entry_already_witnessed(entry_hash, &lineage_dna_hash)? {
                    already_carried = already_carried.saturating_add(1);
                    continue;
                }
            }
        }

        // Our OWN record: re-create it natively from the SAME bytes. Matching
        // the carried action's app entry-def index and round-tripping through
        // the concrete integrity struct reproduces the identical entry, so the
        // EntryHash is preserved across the DNA line.
        //
        // `!held` is load-bearing, not belt-and-braces. A held page names a
        // NEIGHBOUR's chain, so `action.author() == &me` should already be
        // false for every record on it — but a predecessor that returned the
        // wrong page would otherwise make this chain silently re-author another
        // peer's content as its own commit. A courier carries; it does not
        // author.
        let mut recreated = false;
        if !held && action.author() == &me {
            if let (Some(EntryType::App(def)), Some(entry)) =
                (action.entry_type(), carried_entry.as_ref())
            {
                let typed = EntryTypes::deserialize_from_type(
                    def.zome_index,
                    def.entry_index,
                    entry,
                )
                .map_err(|e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "carry_from: proof {i}: could not deserialize the carried entry into a \
                         known entry type: {e:?}"
                    )))
                })?;
                if let Some(typed) = typed {
                    // The whole promise of self-carry is CID continuity: the
                    // re-created entry must hash to exactly what the carried
                    // action commits to. If the two lineage ends disagree about
                    // the struct's shape, the round-trip silently produces
                    // DIFFERENT bytes — a new entry hash under an old action's
                    // proof. Refuse the page; never fall through to
                    // `entry: None`, which would drop the only copy of the
                    // bytes the witness needed.
                    let recreated_hash = hash_entry(&typed)?;
                    let committed = action.entry_hash().ok_or_else(|| {
                        wasm_error!(WasmErrorInner::Guest(format!(
                            "carry_from: proof {i}: an app entry was carried but the action \
                             references no entry hash"
                        )))
                    })?;
                    if &recreated_hash != committed {
                        return Err(wasm_error!(WasmErrorInner::Guest(format!(
                            "carry_from: proof {i}: re-created entry hash differs from the \
                             carried action's — schema drift between lineage ends (re-created \
                             {recreated_hash}, carried action commits to {committed})"
                        ))));
                    }
                    create_entry(typed)?;
                    recreated = true;
                    self_carried = self_carried.saturating_add(1);
                }
            }
        }

        proofs.push(CarriedProof {
            action,
            signature: signed.signature.clone(),
            // Self-carry omits the bytes (they are on this chain now);
            // held-carry ships them so the validator can check the entry hash.
            entry: if recreated { None } else { carried_entry },
        });
    }

    // (4) ONE witness per page — but only when the page actually has new
    // proofs. A page whose every record was already carried authors NO
    // witness at all (`witness_hash: ""`), which the landed storage decoder
    // already treats as "no witness this page" — see `witness_hash`'s doc.
    let carried = proofs.len() as u32 + already_carried;
    let witness_hash = if proofs.is_empty() {
        String::new()
    } else {
        commit_witness(NotarizationWitness {
            lineage_dna_hash,
            proofs,
        })?
        .to_string()
    };

    Ok(CarryReceipt {
        carried,
        next_cursor: page.next_cursor,
        v1_digest: page.digest,
        witness_hash,
        v1_total: page.total,
        self_carried,
        v1_observed_head: page.observed_head,
        already_carried,
    })
}
