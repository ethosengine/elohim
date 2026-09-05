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

/// What a multi-page walk learned on its FIRST page and need not learn again:
/// the head the walk is pinned to, the digest and total it established over the
/// whole chain, and the highest sequence it observed.
///
/// Why this exists (Holochain Evolution Epic Task 24, G8). Before it, every
/// page of `export_records` re-walked the WHOLE chain and re-hashed it to
/// report the same page-independent `digest`/`total`, so carrying N records
/// cost N/`EXPORT_CAP` whole-chain walks — quadratic in the corpus, on the one
/// path a migration has to run to completion. A caller that hands back the
/// `resume` its previous page returned pays that walk ONCE.
///
/// **It is a pin, not a cache.** Handing back a `resume` is a claim that the
/// walk is the same walk, and the export CHECKS it: a page whose chain head or
/// record count no longer matches is refused outright (`chain moved — restart
/// at 0`) rather than served against a stale digest. Refusing is what makes the
/// shortcut safe — a resumed page can never report a digest that disagrees with
/// the records it returns.
///
/// **Entirely optional.** A caller that never sends one (the landed storage
/// driver does not) gets exactly today's behaviour: every page recomputes.
/// `#[serde(default)]` on both the inputs and [`ExportPage::resume`] keeps that
/// byte-compatible in both directions.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct ExportResume {
    /// The chain head this walk is pinned to, rendered with `HoloHash`'s
    /// `Display` (base64) rather than sent as a native hash — the same
    /// string-not-byte-array discipline `CarryReceipt`'s hashes keep, so a
    /// consumer decoding into a `String` field cannot hit the msgpack
    /// byte-array class.
    ///
    /// On the OWN path (`export_records`) it is this agent's source-chain head
    /// from `agent_info()`, which is O(1) to read. On the HELD path
    /// (`export_held_records`) it is the neighbour's chain head as the
    /// agent-activity authority reports it (`ChainStatus`), which is the same
    /// fact from the courier's side.
    ///
    /// Note what this pins on the own path: the CHAIN head, not the last app
    /// entry. A write that commits only links still moves it, so a resumed page
    /// after such a write is refused even though the app-entry walk is
    /// unchanged. Conservative on purpose — a spurious restart costs one walk,
    /// a missed one serves a stale digest.
    pub head: String,
    /// The whole-walk digest established on the first page — the same value
    /// every page of the walk reports, per [`ExportPage::digest`].
    pub digest: String,
    /// The app-record count of the whole walk, per [`ExportPage::total`].
    /// Checked on every resumed page: a walk whose count changed is refused,
    /// which is what catches a courier's view growing underneath a held ordinal
    /// even when the neighbour's head has not moved.
    pub total: u32,
    /// The highest observed sequence, carried forward so a page answered by a
    /// momentarily blind authority does not erase what an earlier page
    /// established — the same last-non-`None` discipline
    /// [`CarryReceipt::v1_observed_head`] documents.
    ///
    /// On the OWN path this is the source-chain HEAD sequence, and it is the
    /// second half of the pin: a chain whose head hash is unchanged but whose
    /// head sequence moved is a contradiction, and a chain that grew moves both.
    #[serde(default)]
    pub observed_head: Option<u32>,
    /// **Additive (Task 24 fix round 1).** Where the page cursor SITS on the
    /// chain: `(app-entry ordinal, action_seq)`.
    ///
    /// This is the field that makes a resumed page cost its own window rather
    /// than the whole chain. The cursor is an ordinal into the app-entry
    /// SUBSEQUENCE, and nothing in an ordinal says which `action_seq` it lands
    /// on — the first implementation therefore rebuilt the whole ordinal index
    /// with a chain-wide header query on every page, so the pin skipped only
    /// the sha256 and the walk stayed linear-per-page (chain-length work, N/CAP
    /// times). Naming the sequence lets the next page start its scan where the
    /// last one stopped.
    ///
    /// Used ONLY when the caller's `cursor` equals the ordinal here. Any other
    /// cursor — a driver that jumped, restarted at a different offset, or is
    /// re-reading an earlier page — falls back to the full ordinal walk, which
    /// is always correct and never wrong, just slower.
    ///
    /// `None` from a page that could not name the next position (the scan
    /// reached the chain head without a further record) or from a coordinator
    /// that predates this field; either way the next page walks in full.
    #[serde(default)]
    pub cursor_seq: Option<(u32, u32)>,
}

/// How far past a page's start the bounded forward scan reads on its first
/// probe, as a multiple of the page limit — then doubling until it has the page
/// plus one, or reaches the chain head.
///
/// MEASURED on this DNA: `register_node` commits SIX actions per app entry (one
/// Create plus five CreateLinks), and the chain also carries three genesis
/// actions and an `InitZomesComplete`. A factor of 8 therefore clears a full
/// page in ONE probe here with slack for a chain whose link density is higher,
/// and the doubling covers anything denser without a second guess. It is a
/// probe size, never a correctness bound: a scan that comes up short simply
/// probes again.
const SCAN_SPAN_FACTOR: u32 = 8;

/// What one bounded forward scan found — named rather than returned as a
/// three-tuple so each number keeps its meaning at the call site.
struct ScannedWindow {
    /// The page's app-entry `(action_seq, ActionHash)` pairs, in chain order.
    window: Vec<(u32, ActionHash)>,
    /// The sequence of the app entry ONE PAST the page, when the scan reached
    /// it — this is what the next page's [`ExportResume::cursor_seq`] carries.
    /// `None` when the scan hit the chain head first, which costs the next page
    /// a full ordinal walk and is never wrong.
    next_start_seq: Option<u32>,
    /// Action rows read. Reported rather than swallowed because it is the
    /// number risk row R1 watches — see [`ExportPage::scanned`].
    scanned: u32,
}

/// Input to [`export_records`]: an opaque page cursor (`None` starts at the
/// beginning of the app-entry portion of the chain) and a page size, capped
/// at [`EXPORT_CAP`].
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExportInput {
    pub cursor: Option<u32>,
    pub limit: u32,
    /// **Additive.** The [`ExportResume`] the previous page returned, or `None`
    /// to start (or restart) a walk. `#[serde(default)]` keeps a caller that
    /// omits the key — every landed one does — byte-compatible.
    #[serde(default)]
    pub resume: Option<ExportResume>,
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
    /// **Additive (Task 26, G10).** The entry-def NAME the EXPORTING DNA
    /// publishes for each record, paired POSITIONALLY with `records` — `""` for
    /// a record that carries no app entry. Declared here rather than appended
    /// with the other additive fields because that pairing is the fact a reader
    /// needs: `records`, `entries` and `type_names` are the page's three
    /// positional vectors, and each is length-checked against the first.
    ///
    /// Why a NAME when the action already carries an entry-def index: the index
    /// is scoped to the DNA that authored it, and the two ends of a lineage are
    /// two DNAs whose entry-type order differs by construction (v2 appends
    /// `NotarizationWitness`). See the "ENTRY TYPES TRAVEL BY NAME" section.
    ///
    /// `#[serde(default)]`, and EMPTY from a coordinator that predates Task 26
    /// — which the receiving end falls back to the carried index for, with a
    /// logged warning rather than a refusal (§7 C10: old peers keep working).
    #[serde(default)]
    pub type_names: Vec<String>,
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
    /// **Additive.** Hand this back on the next [`ExportInput`]/
    /// [`ExportHeldInput`] and the walk is pinned rather than re-derived — see
    /// [`ExportResume`]. Every page returns one; ignoring it is always safe and
    /// costs exactly what this export cost before the field existed.
    #[serde(default)]
    pub resume: Option<ExportResume>,
    /// **Additive (Task 24 fix round 1).** How many action rows this page's
    /// POSITION scan read — the cost of finding where the page starts.
    ///
    /// **This is the metric risk row R1 reads.** Carry cost stays linear in
    /// chain length iff this stays bounded: assert `scanned` on resumed pages
    /// ≪ chain length, and — the property that actually matters — INDEPENDENT
    /// of how far into the chain the page sits. An unpinned page reports the
    /// whole chain's action count, because finding an arbitrary ordinal costs
    /// exactly that; a pinned page reports only its own probe span.
    ///
    /// Scope is deliberately the position scan ALONE. The windowed entry read
    /// that follows it is bounded by `limit` by construction and was never the
    /// quadratic term, so folding it in here would blunt the one number that
    /// tells you whether the walk regressed. On the held path this is the size
    /// of the agent-activity list the authority returned — the same "rows read
    /// to find the position" quantity, from the courier's side.
    ///
    /// `None` from a coordinator that predates the field — never a fabricated
    /// 0, which would read as a page that scanned nothing.
    #[serde(default)]
    pub scanned: Option<u32>,
    /// **Additive (Task 29).** WHICH read answered this page —
    /// [`HELD_VIEW_AUTHORITY`], [`HELD_VIEW_LOCAL_ONLY`] or
    /// [`HELD_VIEW_UNREACHABLE`] — or `None` on the own-chain path, where none
    /// of the three applies because `export_records` is a local `query()` of the
    /// caller's own chain and there is no other view to choose between.
    ///
    /// **This is the field Station 6 was missing.** MEASURED on the household
    /// mesh (2026-09-04): james's held view of jessica froze at 212 records
    /// against an `observed_head` of 732 while her chain had passed 322, and it
    /// read the SAME before and after a conductor respawn — because
    /// `GetOptions::local()` on a PARTIAL-ARC peer only ever reports the slice
    /// of her chain that hashes into james's arc, and no amount of waiting
    /// fills the rest. A page that reads short is indistinguishable from a
    /// chain that IS short unless the page says which store answered, so the
    /// courier reported a truncated view as a complete one.
    ///
    /// [`HELD_VIEW_AUTHORITY`] means the agent-activity AUTHORITIES answered
    /// over the network — the whole chain as the DHT holds it, independent of
    /// this peer's arc. [`HELD_VIEW_LOCAL_ONLY`] means the network handed over
    /// nothing and this conductor fell back to its own store, which did hold
    /// records; the page is then scoped to what THIS peer validated and
    /// integrated, and every "courier's view" caveat on `total`, `digest` and
    /// `next_cursor` applies at its strongest. [`HELD_VIEW_UNREACHABLE`] means
    /// nobody answered at all and this peer holds nothing either — an absence of
    /// evidence, never evidence of an empty chain.
    ///
    /// A driver reads it as a CONFIDENCE label, never as a completeness proof:
    /// `authority` does not promise the authorities were caught up either, it
    /// promises the read reached one and was not silently answered by this
    /// peer's arc — or by nothing at all.
    ///
    /// **Why a `String` and not a closed enum**, given three labels: a v2 cell
    /// DECODES this field off a predecessor's page in `carry_from`, so a closed
    /// enum would turn a fourth label from some future coordinator into a
    /// whole-page decode failure on an older peer. That is precisely the refusal
    /// epic §7 C10 forbids (old peers keep working, loudly). The three
    /// `HELD_VIEW_*` consts above are the labels' single home; the wire stays
    /// forward-tolerant.
    ///
    /// `Option<String>` + `#[serde(default)]`: `None` from a bundle packed
    /// before this field existed, which is honestly "not reported" rather than
    /// a fabricated label.
    #[serde(default)]
    pub view: Option<String>,
}

/// [`ExportPage::view`] when the agent-activity AUTHORITIES answered over the
/// network — the read every held page wants, because it is independent of this
/// peer's arc.
pub const HELD_VIEW_AUTHORITY: &str = "authority";

/// [`ExportPage::view`] when the network handed over no valid activity and this
/// conductor fell back to its OWN store, which did hold records. The page is
/// then scoped to what this peer holds, which on a partial-arc peer is a slice
/// of the chain.
///
/// Deliberately NOT split by why the network came up short. Whether the
/// authorities answered empty or were never reached, a page served from the
/// local store is arc-scoped either way, and that is the whole content of the
/// label.
pub const HELD_VIEW_LOCAL_ONLY: &str = "local-only";

/// [`ExportPage::view`] when NOBODY ANSWERED: the network read came back in the
/// synthesised zero-response shape (no valid activity, no `highest_observed`,
/// `ChainStatus::Empty`) and the local store held nothing either, so the page
/// reports an empty walk that nothing in the network ever confirmed.
///
/// This is the label that keeps an unreachable authority from wearing
/// [`HELD_VIEW_AUTHORITY`]. `NoPeersForLocation` reaches a zome as a well-formed
/// empty response rather than an error (see `export_held_records`), so without
/// this third state an unanswered question is served as a settled fact — the
/// Station 6 failure shape, under the one label that tells a driver to trust it.
///
/// A driver reads it as "re-ask, do not record": an `unreachable` page's
/// `total: Some(0)` is an absence of evidence, never evidence of an empty chain.
/// MEASURED cause on the household mesh (2026-09-04): every peer in the
/// node_registry space advertises a null storage arc, so no peer is an
/// agent-activity authority for any location and every network read there
/// returns this shape.
pub const HELD_VIEW_UNREACHABLE: &str = "unreachable";

const EXPORT_CAP: u32 = 64;

/// The named refusal a resumed page gets when the walk it claims to continue is
/// no longer the walk it started. Named ON PURPOSE and identically on both
/// export paths: a driver matches on it to decide "restart at 0", and a message
/// that varied by path would make that decision path-dependent.
fn chain_moved(what: &str, was: &str, now: &str) -> WasmError {
    wasm_error!(WasmErrorInner::Guest(format!(
        "chain moved — restart at 0: the resume token pins {what} {was}, but this walk now sees \
         {now}. Re-issue this export with `resume: None` and `cursor: None`; a resumed page is \
         refused rather than served against a digest it no longer describes."
    )))
}

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

// ============================================================================
// ENTRY TYPES TRAVEL BY NAME (Holochain Evolution Epic Task 26, G10; §7 C10)
//
// An `AppEntryDef` on a carried action is a pair of INDEXES — a zome index and
// an entry-def index — and both are scoped to the DNA that authored it. Two
// lineage ends are two DNAs. The moment a successor adds, removes or reorders
// an entry type (v2 does exactly this: it appends `NotarizationWitness`), the
// same index pair names a DIFFERENT type on the two ends. A carry or a
// re-adoption that trusted the carried index would then re-create a record AS
// THE WRONG TYPE — silently, under a fresh entry hash wearing an old fact's
// name. That is the one failure the whole crossing exists to prevent, and an
// index is the one thing on the wire that cannot be checked across the boundary
// it crossed.
//
// So the type travels by NAME. Every export page carries `type_names`
// positionally beside `records`, resolved from the EXPORTING DNA's own
// `zome_types.entries`; every receiving end resolves that name through ITS OWN
// scope and never reads the carried index. A name this end does not host is
// `foreign` on the re-adoption path (v2's witness read by v1 — a count, never
// an error) and a NAMED refusal on the self-carry path (a fact this end cannot
// host must not be re-created as whatever the carried index happens to name).
//
// §7 C10 (contract evolution) requires old peers keep working: a page from a
// coordinator that predates this task carries NO names, and is walked by the
// carried index with a logged warning rather than refused. That fallback is the
// pre-Task-26 behaviour exactly, and it is the ONLY path on which an index is
// ever trusted across a DNA boundary.
//
// Unconditional and coordinator-only: the DNA hash covers integrity zomes and
// modifiers, so the default `node-registry.dna` hash is unmoved by every line
// of this section.
// ============================================================================

/// One app entry type as THIS DNA hosts it: the scoped `(zome_index,
/// entry_index)` an action on this chain carries for it, the NAME the DNA
/// publishes for it, and this zome's own `EntryTypes` unit — the three facts a
/// translation between lineage ends needs, in one row.
#[derive(Debug, Clone)]
struct LocalEntryType {
    scoped: ScopedEntryDefIndex,
    name: String,
    unit: UnitEntryTypes,
}

/// This DNA's app entry types, read from `zome_info()` ONCE per page.
///
/// The name is the entry-def id the integrity zome registers (`EntryDef::from`
/// a unit variant), which is the snake_case of the `EntryTypes` variant — the
/// same string on both lineage ends for any type both ends host, and so the
/// whole basis of the translation. Derived at runtime, so the `lineage-witness`
/// build's extra entry type is covered without this function naming it — the
/// same discipline [`app_entry_types_in_scope`] keeps.
///
/// Hoisted out of every caller's loop on purpose: `zome_info()` is a host call,
/// and resolving per record would pay one per record.
fn local_entry_types() -> ExternResult<Vec<LocalEntryType>> {
    let entries = zome_info()?.zome_types.entries;
    let mut table = Vec::new();
    for unit in UnitEntryTypes::iter() {
        if let Some(scoped) = entries.get(unit) {
            table.push(LocalEntryType {
                scoped,
                name: entry_def_name(unit),
                unit,
            });
        }
    }
    Ok(table)
}

/// The entry-def name a unit variant registers.
///
/// The two capability ids are rendered rather than unwrapped so this stays
/// total; neither is an app entry type, so neither can reach an export page.
fn entry_def_name(unit: UnitEntryTypes) -> String {
    match EntryDef::from(unit).id {
        EntryDefId::App(name) => name.0.into_owned(),
        EntryDefId::CapClaim => "cap_claim".to_string(),
        EntryDefId::CapGrant => "cap_grant".to_string(),
    }
}

/// The name THIS DNA publishes for the app entry type an action carries.
///
/// `""` for a record that carries no app entry — which is the ONE meaning of
/// the empty string on the wire, and what a receiving end reads it as. An
/// export walks only its own scope, so an app entry it cannot name is
/// unreachable; it renders as `""` too rather than panicking, and the receiving
/// end then treats it as a type it cannot host.
fn exported_type_name(table: &[LocalEntryType], action: &Action) -> String {
    match action.entry_type() {
        Some(EntryType::App(def)) => {
            let scoped = ScopedEntryDefIndex {
                zome_index: def.zome_index(),
                zome_type: def.entry_index(),
            };
            table
                .iter()
                .find(|t| t.scoped == scoped)
                .map(|t| t.name.clone())
                .unwrap_or_default()
        }
        _ => String::new(),
    }
}

/// How a page's carried entry types are being read on THIS end.
enum CarriedTypes {
    /// The page named its types. Positional with `records`, and the only path
    /// on which a type crossing a DNA boundary is checked.
    ByName(Vec<String>),
    /// The page named none — the far end predates Task 26. The carried
    /// entry-def INDEX is used, which is sound only while both lineage ends
    /// happen to agree on their entry-type order. §7 C10: old peers keep
    /// working, loudly.
    ByCarriedIndex,
}

/// What a carried record's app entry type resolves to on THIS DNA.
enum LocalTypeMatch {
    /// This DNA hosts the type. The unit is ITS OWN — resolved here, never
    /// taken from the carried index.
    Known(UnitEntryTypes),
    /// This DNA does not host it (v2's `NotarizationWitness` read by v1, above
    /// all). The `String` renders what the page claimed, for the message the
    /// caller writes: a count on the re-adoption path, a refusal on the carry.
    Foreign(String),
}

impl CarriedTypes {
    /// Read a page's `type_names`.
    ///
    /// A page whose names and records disagree in length is refused BY NAME:
    /// the two are paired POSITIONALLY, so a length mismatch means index `i`
    /// names a different record in each and nothing on the page can be paired
    /// at all — the same reason `records`/`entries` are length-checked.
    fn read(page: &ExportPage, whose: &str) -> ExternResult<Self> {
        if page.type_names.is_empty() {
            if !page.records.is_empty() {
                warn!(
                    "{whose}: this page carries no `type_names` — the far end predates the \
                     name-carrying export. Falling back to the carried entry-def INDEX for {} \
                     record(s); an index is only meaningful inside the DNA that authored it, so \
                     this is sound only while both lineage ends agree on their entry-type order.",
                    page.records.len()
                );
            }
            return Ok(Self::ByCarriedIndex);
        }
        if page.type_names.len() != page.records.len() {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "{whose}: the page carries {} type names for {} records — `type_names` is paired \
                 POSITIONALLY with `records`, so a length mismatch cannot be paired at all. \
                 Refusing rather than re-creating records under names that are not theirs.",
                page.type_names.len(),
                page.records.len()
            ))));
        }
        Ok(Self::ByName(page.type_names.clone()))
    }

    /// Resolve record `i`'s app entry type to the unit variant THIS DNA hosts
    /// for it.
    ///
    /// `def` is read ONLY on the legacy arm. On the named arm the carried index
    /// is ignored entirely — that is the whole point of G10.
    fn resolve(&self, table: &[LocalEntryType], i: usize, def: &AppEntryDef) -> LocalTypeMatch {
        match self {
            Self::ByName(names) => {
                // `read` established `names.len() == records.len()`, so this
                // cannot miss for an `i` that indexes `records`; the default
                // keeps it total, and `""` is a name no entry def registers.
                let name = names.get(i).map(String::as_str).unwrap_or_default();
                match table.iter().find(|t| t.name == name) {
                    Some(t) => LocalTypeMatch::Known(t.unit),
                    None => LocalTypeMatch::Foreign(format!("entry type `{name}`")),
                }
            }
            Self::ByCarriedIndex => {
                let scoped = ScopedEntryDefIndex {
                    zome_index: def.zome_index(),
                    zome_type: def.entry_index(),
                };
                match table.iter().find(|t| t.scoped == scoped) {
                    Some(t) => LocalTypeMatch::Known(t.unit),
                    None => LocalTypeMatch::Foreign(format!(
                        "the unnamed entry-def index {}/{} it carried",
                        def.zome_index().0,
                        def.entry_index().0
                    )),
                }
            }
        }
    }
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
///
/// # The walk is linear (Task 24, G8; corrected in fix round 1)
///
/// Three costs used to be one `query(include_entries(true))` over the whole
/// chain, run on every page: finding where the page starts, hashing the walk,
/// and loading the page's entries. Carrying N records therefore cost
/// `N * N/EXPORT_CAP` entry loads. They are now three separate reads, and the
/// two that scale with the chain are paid ONCE per walk rather than per page:
///
///   * **Entries are loaded for THIS WINDOW only** — a query bounded by
///     `ChainQueryFilterRange::ActionSeqRange` over the window's first and last
///     sequence. Unconditional: the landed storage driver gets it without
///     changing a byte it sends. Total entry loads across a walk: N, once.
///   * **The digest is computed once**, on the first (unpinned) page, and
///     reported verbatim by every page that hands the pin back.
///   * **The POSITION scan is bounded on a pinned page.** The page cursor is an
///     ordinal into the app-entry SUBSEQUENCE, so nothing in it says which
///     `action_seq` it lands on. An unpinned page has no choice but to rebuild
///     that index with one chain-wide HEADERS-ONLY query — which is what the
///     first cut of this task did on EVERY page, so the pin skipped only the
///     sha256 and this doc's claim to "pay that walk once" was false. A pinned
///     page whose cursor matches [`ExportResume::cursor_seq`] instead scans
///     FORWARD from the sequence the previous page named, in doubling probes
///     from `limit * SCAN_SPAN_FACTOR`, and stops as soon as it has the page
///     plus one. [`ExportPage::scanned`] reports what it read, and is the
///     number risk row R1 watches.
///
/// The pin itself costs NO queries at all: `agent_info()` reports the
/// source-chain head hash and sequence directly, and on an append-only chain an
/// unchanged head hash IS the proof that the chain below it is unchanged — the
/// head action commits to its predecessor, transitively to genesis. See
/// [`ExportPage::resume`].
#[hdk_extern]
pub fn export_records(input: ExportInput) -> ExternResult<ExportPage> {
    let limit = input.limit.clamp(1, EXPORT_CAP) as usize;
    let start = input.cursor.unwrap_or(0) as usize;

    // (1) The pin, at ZERO queries. `agent_info()` reports this agent's
    //     source-chain head — hash AND sequence — as of the start of the call,
    //     and this extern writes nothing, so it is stable for the page.
    let (head_hash, head_seq, _) = agent_info()?.chain_head;
    let head = head_hash.to_string();

    // (2) Verify the pin BEFORE reading anything. A source chain is append-only
    //     and every action commits to its predecessor, so an unchanged head
    //     hash is a complete proof that the chain below it is byte-identical to
    //     the one the first page walked — which is what licenses reusing that
    //     page's `digest` and `total` without re-deriving them. The sequence
    //     check beside it is free from the same call and catches the one shape
    //     the hash alone could not explain (a head that is somehow the same
    //     action at a different position).
    let pinned = match input.resume.as_ref() {
        None => None,
        Some(resume) => {
            if resume.head != head {
                return Err(chain_moved("chain head", &resume.head, &head));
            }
            if let Some(pinned_seq) = resume.observed_head {
                if pinned_seq != head_seq {
                    return Err(chain_moved(
                        "a chain-head sequence of",
                        &pinned_seq.to_string(),
                        &head_seq.to_string(),
                    ));
                }
            }
            Some(resume)
        }
    };
    let observed_head = Some(head_seq);

    // (3) Position. The FAST path applies only when the pin names where THIS
    //     cursor sits; anything else rebuilds the ordinal index in full, which
    //     is always correct and is what an unpinned first page does anyway.
    let fast_from = pinned.and_then(|resume| match resume.cursor_seq {
        Some((ordinal, seq)) if ordinal as usize == start => Some(seq),
        _ => None,
    });

    let (window, next_start_seq, total, digest, scanned) = match (fast_from, pinned) {
        (Some(from_seq), Some(resume)) => {
            // Bounded: probes forward from where the last page stopped. `total`
            // and `digest` come from the pin — verified above, so this page
            // never has to see the whole chain to report them.
            let scan = scan_forward(from_seq, head_seq, limit)?;
            (
                scan.window,
                scan.next_start_seq,
                resume.total,
                resume.digest.clone(),
                scan.scanned,
            )
        }
        _ => {
            // The ordinal index, HEADERS ONLY. `include_entries(false)` is the
            // whole point — the action rows carry the entry TYPE, the sequence
            // and the action hash, which is everything the cursor arithmetic
            // and the digest need. The entry blobs, which are the expensive
            // part, stay on disk until step (4) knows which handful this page
            // wants.
            let headers = query(ChainQueryFilter::new().include_entries(false))?;
            let scanned = headers.len() as u32;
            let mut app: Vec<(u32, ActionHash)> = headers
                .into_iter()
                .filter(|r| matches!(r.action().entry_type(), Some(EntryType::App(_))))
                .map(|r| (r.action().action_seq(), r.action_address().clone()))
                .collect();
            app.sort_by_key(|(seq, _)| *seq);
            let total = app.len() as u32;
            // Only assertable when we actually walked. It cannot fail after the
            // head check above — it is kept as the loud contradiction it would
            // be if it ever did.
            if let Some(resume) = pinned {
                if resume.total != total {
                    return Err(chain_moved(
                        "an app-record count of",
                        &resume.total.to_string(),
                        &total.to_string(),
                    ));
                }
            }
            let digest = match pinned {
                Some(resume) => resume.digest.clone(),
                None => chain_digest(app.iter().map(|(_, hash)| hash)),
            };
            let window: Vec<(u32, ActionHash)> =
                app.iter().skip(start).take(limit).cloned().collect();
            let next_start_seq = app.get(start + limit).map(|(seq, _)| *seq);
            (window, next_start_seq, total, digest, scanned)
        }
    };

    // A FULL page always offers a cursor, even when the chain happens to end
    // exactly on the page boundary — the landed `fold_carry` terminates on
    // `next_cursor: None` and expects the trailing empty page. Unchanged.
    let next_cursor = if window.len() == limit {
        Some((start + limit) as u32)
    } else {
        None
    };
    // The next page can skip its ordinal walk only if we can name where it
    // starts. `None` (the scan reached the head with nothing beyond the page)
    // costs that page one full walk and is never wrong.
    let cursor_seq = match (next_cursor, next_start_seq) {
        (Some(cursor), Some(seq)) => Some((cursor, seq)),
        _ => None,
    };

    // (4) Entries for the window, and only the window. The sequence range spans
    //     any non-app actions that happen to sit between the first and last
    //     record of the page — those carry no entry, so they cost nothing — and
    //     the records are re-ordered by the window's own order rather than the
    //     query's, so the page can never disagree with the ordinals it was
    //     asked for.
    let mut records = Vec::with_capacity(window.len());
    let mut entries = Vec::with_capacity(window.len());
    // The page names its own types (Task 26). One `zome_info()` for the page,
    // not one per record.
    let type_table = local_entry_types()?;
    let mut type_names = Vec::with_capacity(window.len());
    if let (Some((first_seq, _)), Some((last_seq, _))) = (window.first(), window.last()) {
        let loaded = query(
            ChainQueryFilter::new()
                .sequence_range(ChainQueryFilterRange::ActionSeqRange(*first_seq, *last_seq))
                .include_entries(true),
        )?;
        let mut by_hash: std::collections::HashMap<ActionHash, Record> = loaded
            .into_iter()
            .map(|r| (r.action_address().clone(), r))
            .collect();
        for (_, action_hash) in &window {
            // The header scan named this action a moment ago in the same call,
            // so a miss is not a stale-view problem — it is this export
            // disagreeing with itself. Refuse loudly rather than return a page
            // shorter than the cursor it advances.
            let record = by_hash.remove(action_hash).ok_or_else(|| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "export_records: action {action_hash} was named by the chain header scan but \
                     not returned by the windowed entry query — refusing to return a page that \
                     disagrees with its own cursor"
                )))
            })?;
            type_names.push(exported_type_name(&type_table, record.action()));
            entries.push(record.entry().as_option().cloned());
            records.push(record.signed_action);
        }
    }

    Ok(ExportPage {
        records,
        entries,
        type_names,
        next_cursor,
        digest: digest.clone(),
        total: Some(total),
        observed_head,
        resume: Some(ExportResume {
            head,
            digest,
            total,
            observed_head,
            cursor_seq,
        }),
        scanned: Some(scanned),
        // `None`, not a label. This is the OWN chain read with a local
        // `query()`; there is no authority-vs-local choice to report, and
        // stamping `local-only` here would read as the partial-arc shortfall
        // Task 29 named on the held path, which this path cannot have.
        view: None,
    })
}

/// Collect the app-entry `(action_seq, ActionHash)` pairs for one page WITHOUT
/// reading the whole chain — the bounded half of Task 24's fix round 1.
///
/// Starts at `from_seq` (the sequence the previous page named in
/// [`ExportResume::cursor_seq`]) and probes forward in header-only windows,
/// doubling the span until it holds `limit + 1` app entries or reaches
/// `head_seq`. The extra one is what lets the caller name the NEXT page's
/// starting sequence; without it every page would hand back a `cursor_seq` of
/// `None` and the walk would fall back to a full scan on each step.
///
/// Correctness does not depend on the probe size: coming up short simply probes
/// again with twice the span, and reaching the head ends the scan. The span
/// only decides how many round trips a page costs.
fn scan_forward(from_seq: u32, head_seq: u32, limit: usize) -> ExternResult<ScannedWindow> {
    let want = limit.saturating_add(1);
    let mut collected: Vec<(u32, ActionHash)> = Vec::with_capacity(want);
    let mut scanned: u32 = 0;
    let mut cursor = from_seq;
    let mut span = (limit as u32)
        .saturating_mul(SCAN_SPAN_FACTOR)
        .max(SCAN_SPAN_FACTOR);

    while cursor <= head_seq && collected.len() < want {
        let end = cursor.saturating_add(span.saturating_sub(1)).min(head_seq);
        let rows = query(
            ChainQueryFilter::new()
                .sequence_range(ChainQueryFilterRange::ActionSeqRange(cursor, end))
                .include_entries(false),
        )?;
        scanned = scanned.saturating_add(rows.len() as u32);
        let mut probed: Vec<(u32, ActionHash)> = rows
            .into_iter()
            .filter(|r| matches!(r.action().entry_type(), Some(EntryType::App(_))))
            .map(|r| (r.action().action_seq(), r.action_address().clone()))
            .collect();
        probed.sort_by_key(|(seq, _)| *seq);
        collected.extend(probed);
        if end >= head_seq {
            break;
        }
        cursor = end.saturating_add(1);
        span = span.saturating_mul(2);
    }

    collected.truncate(want);
    let next_start_seq = collected.get(limit).map(|(seq, _)| *seq);
    collected.truncate(limit);
    Ok(ScannedWindow {
        window: collected,
        next_start_seq,
        scanned,
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
    /// **Additive.** The [`ExportResume`] the previous page returned. On this
    /// path the pin is doubly load-bearing: a courier's view of a neighbour can
    /// GROW mid-walk as gossip arrives, which silently shifts the ordinals a
    /// cursor indexes into. An unpinned walk only notices via the digest, after
    /// the fact; a pinned one is refused at the door.
    #[serde(default)]
    pub resume: Option<ExportResume>,
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
/// **WHICH VIEW answered is reported, not assumed (Task 29).** The read asks the
/// agent-activity AUTHORITIES over the network first, and falls back to this
/// conductor's own store only when the network hands over no valid activity.
/// [`ExportPage::view`] names which of the three states served the page —
/// [`HELD_VIEW_AUTHORITY`], [`HELD_VIEW_LOCAL_ONLY`] or
/// [`HELD_VIEW_UNREACHABLE`]. That distinction is Station 6's root cause: a
/// local-only read on a PARTIAL-ARC peer returns the slice of the neighbour's
/// chain that hashes into this peer's arc and never fills, so a page taken from
/// it reports a permanent truncation in exactly the shape of a complete short
/// chain.
///
/// The third state exists because an unreachable network is not an error here:
/// `NoPeersForLocation` arrives as a well-formed EMPTY response, so `authority`
/// is gated on evidence that an authority actually spoke — see the comment on
/// the read itself.
///
/// **Even on the authority view, "whole" is a claim about the read, not a
/// proof.** `total`, `digest` and `next_cursor` describe the walk this page
/// drew from; on [`HELD_VIEW_LOCAL_ONLY`] that walk is a subset of `agent`'s
/// real chain, possibly gapped. A page from this extern is therefore never
/// self-evidencing: `next_cursor: None` means end-of-THIS-VIEW, and a receipt's
/// `carried == total` says the courier carried everything the view held.
/// [`ExportPage::observed_head`] is the one field that reaches past the walk and
/// lets a driver notice it is behind.
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

    // THE AUTHORITIES FIRST, this conductor's own store only as a fallback
    // (Task 29 — Station 6's root cause).
    //
    // This read used to be `GetOptions::local()` alone, on a
    // verify-locally-then-serve reading of the courier's role plus one measured
    // fact: a LONE conductor reading its OWN key with `network()` returns an
    // EMPTY activity list even 60 s after five `register_node` calls, while
    // `local()` returns all five. That measurement stands — and it is a
    // single-conductor artefact, which is why the fallback below keeps it
    // working.
    //
    // What it hid is the case that actually matters. MEASURED on the household
    // mesh (2026-09-04, probe r36): james's held view of jessica froze at 212
    // records against an `observed_head` of 732 while jessica's chain had
    // passed 322, and it read IDENTICALLY before and after a conductor
    // respawn. `local()` on a PARTIAL-ARC peer returns only the slice of a
    // neighbour's chain that hashes into that peer's arc — it is not a view
    // that is behind and catching up, it is a view that never fills. A courier
    // reading it reported a permanent truncation as a complete crossing, and
    // Station 6 could not reach the neighbour's real head at all.
    //
    // So: ask the agent-activity AUTHORITIES over the network, which answer for
    // the whole chain regardless of this peer's arc. Fall back to the local
    // store ONLY when the network answered with no valid activity at all AND
    // the local store does hold records — the lone-conductor shape above, and
    // the one case where `local()` is strictly more informative than the empty
    // answer it would otherwise be believed over.
    //
    // The page then SAYS which store answered (`ExportPage::view`), because a
    // short page from a partial-arc local read and a short page from a short
    // chain are otherwise indistinguishable — which is exactly how the frozen
    // 212 read as a total.
    let authority_activity = get_agent_activity(
        input.agent.clone(),
        filter.clone(),
        ActivityRequest::Full,
        GetOptions::network(),
    )?;

    // AN EMPTY NETWORK ANSWER IS NOT PROOF THAT AN AUTHORITY ANSWERED (fix
    // round 1). The absence of a reachable authority arrives here as a
    // well-formed EMPTY response, not as an error, so it is indistinguishable
    // from "the authorities answered and the chain holds nothing" unless this
    // read looks for evidence of an answer. Traced through the pinned conductor
    // fork (holochain 0.7.0):
    //
    //   * `HolochainP2pActor::get_agent_activity` raises `NoPeersForLocation`
    //     when no peer advertises a storage arc covering the agent's location;
    //   * `Cascade::fetch_agent_activity` converts that error into `vec![]`;
    //   * `agent_activity::merge_activities` over an EMPTY result set yields
    //     `ChainStatus::Empty`, empty `valid_activity`, `highest_observed: None`.
    //
    // Transport errors and timeouts still propagate and are refused by the `?`
    // above; the hole is `NoPeersForLocation` alone — and it is LIVE, not
    // hypothetical: MEASURED on the household mesh (2026-09-04), every peer in
    // the node_registry space advertises a null storage arc and no gossip round
    // has ever completed there, so there are no agent-activity authorities at
    // all and every network read returns exactly this shape.
    //
    // The discriminator is in the response and costs nothing to read: a real
    // authority speaking for a live agent reports `highest_observed: Some(_)`
    // and a non-`Empty` `ChainStatus`, because genesis actions exist even when
    // the FILTERED set is empty. The synthesised zero-response reports neither.
    let authority_answered = !authority_activity.valid_activity.is_empty()
        || authority_activity.highest_observed.is_some()
        || !matches!(authority_activity.status, ChainStatus::Empty);

    let (activity, view) = if !authority_activity.valid_activity.is_empty() {
        // Records came back over the network. A zero-response cannot produce
        // records, so this branch is self-evidencing.
        (authority_activity, HELD_VIEW_AUTHORITY)
    } else {
        let local_activity = get_agent_activity(
            input.agent.clone(),
            filter,
            ActivityRequest::Full,
            GetOptions::local(),
        )?;
        if !local_activity.valid_activity.is_empty() {
            // The local store holds records the network did not hand over. The
            // page is served from it and SAYS so — whether the network was
            // merely empty or was never reached, `local-only` is the honest
            // label, because either way this page is scoped to one peer's arc.
            (local_activity, HELD_VIEW_LOCAL_ONLY)
        } else if authority_answered {
            // Both empty, and the network answer carries evidence that an
            // authority produced it. THAT is an `authority` page reporting a
            // chain with no matching entries.
            (authority_activity, HELD_VIEW_AUTHORITY)
        } else {
            // Both empty and NOBODY ANSWERED. Reporting `authority` here would
            // be the Station 6 failure in a better costume: an unanswered
            // question served as a settled fact, under the one label that tells
            // a driver to trust it. The page says `unreachable` instead.
            (authority_activity, HELD_VIEW_UNREACHABLE)
        }
    };

    // MEASURED (holochain 0.7.0): `highest_observed` is computed by
    // `build_agent_activity_response` from the classified lists BEFORE
    // `filter.filter_actions` runs, so it spans the whole chain the authority
    // has seen — genesis, links and all — not just the app entries this walk
    // returns. That is what makes it a truncation check the page cannot fake:
    // it reaches past the filtered view the rest of this response describes.
    let observed_head = activity
        .highest_observed
        .map(|h| h.action_seq)
        .or_else(|| input.resume.as_ref().and_then(|r| r.observed_head));

    // The pin for the HELD path (Task 24, G8). `ChainStatus` is the authority's
    // statement about the neighbour's chain head IRRESPECTIVE of the filters —
    // the same fact `agent_info().chain_head` is on the own path, read from the
    // courier's side. The non-`Valid` states get stable renderings rather than
    // being collapsed into one string: a chain that goes Empty → Valid HAS
    // moved, and a resumed walk across that transition must be refused.
    let head = match &activity.status {
        ChainStatus::Empty => "empty".to_string(),
        ChainStatus::Valid(head) => head.hash.to_string(),
        // BOTH conflicting hashes, SORTED. `ChainFork` documents the ordering
        // of `first_action`/`second_action` as undefined and peer-dependent, and
        // the fork sequence alone cannot tell two different forks at the same
        // position apart — so rendering either of those alone would make the
        // pin disagree with itself across peers, or silently accept a walk that
        // resumed onto a different branch.
        ChainStatus::Forked(fork) => {
            let mut pair = [
                fork.first_action.to_string(),
                fork.second_action.to_string(),
            ];
            pair.sort();
            format!("forked@{}:{}:{}", fork.fork_seq, pair[0], pair[1])
        }
        ChainStatus::Invalid(head) => format!("invalid@{}", head.hash),
        // A SEALED predecessor (Station 8's `CloseChain`) is a distinct state
        // from a live one, and it is exactly the transition a crossing walks
        // through. Rendering it separately means a walk that began before the
        // seal and resumed after it is refused rather than continued against a
        // digest taken from the open chain.
        ChainStatus::Closed(head) => format!("closed@{}", head.hash),
    };

    // `valid_activity` arrives ascending by sequence (the authority sorts before
    // filtering), which is the order `export_records` establishes with its
    // explicit `sort_by_key(action_seq)`.
    let hashes: Vec<ActionHash> = activity
        .valid_activity
        .into_iter()
        .map(|(_seq, hash)| hash)
        .collect();

    let count = hashes.len() as u32;
    // The held path's POSITION scan IS the activity list: the authority hands
    // back the whole filtered set of `(seq, hash)` pairs in one round trip, and
    // there is no cheaper index into it. So `scanned` is that list's size, and
    // — unlike the own path — a pin cannot shrink it. What the pin removes here
    // is the per-page digest; see `ExportPage::scanned`.
    let scanned = count;
    // Digest once per walk, as on the own path. The COUNT check beside the head
    // check is what makes the pin sound here specifically: gossip adds records
    // to the courier's view, and a back-fill below the head would change the
    // digest without moving the head at all. A view that grew is a different
    // walk, and is refused rather than served against the old digest.
    let digest = match input.resume.as_ref() {
        Some(resume) => {
            if resume.head != head {
                return Err(chain_moved("chain head", &resume.head, &head));
            }
            if resume.total != count {
                return Err(chain_moved(
                    "an app-record count of",
                    &resume.total.to_string(),
                    &count.to_string(),
                ));
            }
            resume.digest.clone()
        }
        None => chain_digest(hashes.iter()),
    };
    let total = Some(count);

    let start = input.cursor.unwrap_or(0) as usize;
    let window: Vec<ActionHash> = hashes.into_iter().skip(start).take(limit).collect();
    let next_cursor = if window.len() == limit {
        Some((start + limit) as u32)
    } else {
        None
    };

    let mut records = Vec::with_capacity(window.len());
    let mut entries = Vec::with_capacity(window.len());
    // As on the own path (Task 26): the page names its own types, from THIS
    // DNA's scope. A courier and the neighbour it reads run the same DNA, so
    // the name it publishes for a held record is the neighbour's own name for
    // it — which is what makes a held page carryable by the same rule.
    let type_table = local_entry_types()?;
    let mut type_names = Vec::with_capacity(window.len());
    for action_hash in window {
        let record = get(action_hash.clone(), GetOptions::network())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "export_held_records: the agent-activity authority named action {action_hash} \
                 for {}, but it could not be fetched — refusing to return a page that \
                 disagrees with its own digest and total",
                input.agent
            )))
        })?;
        type_names.push(exported_type_name(&type_table, record.action()));
        entries.push(record.entry().as_option().cloned());
        records.push(record.signed_action);
    }

    Ok(ExportPage {
        records,
        entries,
        type_names,
        next_cursor,
        digest: digest.clone(),
        total,
        observed_head,
        resume: Some(ExportResume {
            head,
            digest,
            total: count,
            observed_head,
            // Deliberately NOT named on the held path. `cursor_seq` exists to
            // let the next page skip rebuilding an ordinal index; here the
            // index arrives whole with the activity response, so naming a
            // sequence would buy nothing and would imply a positional stability
            // a growing courier view does not have.
            cursor_seq: None,
        }),
        scanned: Some(scanned),
        view: Some(view.to_string()),
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
///
/// # Batched per page (Task 24, G8)
///
/// `seen` memoizes "which lineage does the witness at this action hash carry"
/// for the WHOLE page, and the caller owns it. One witness carries up to
/// [`WITNESS_BATCH`] proofs and therefore has an `EntryToWitness` link from
/// each of those entry hashes, so the un-memoized version re-fetched and
/// re-decoded the SAME witness record once per record of the page — a page of
/// 32 previously-carried records cost 32 `get`s of one entry. It now costs one.
///
/// The `get_links` per distinct entry hash stays: the link base IS the entry
/// hash, so there is no index that answers the whole page in one read. What is
/// removed is the redundant record fetch behind those links.
///
/// A witness that cannot be fetched or that decodes to no
/// [`NotarizationWitness`] memoizes as `None` — "known not to be a witness of
/// any lineage" — so a broken link is also read only once per page.
#[cfg(feature = "lineage-witness")]
fn entry_already_witnessed(
    entry_hash: &EntryHash,
    lineage_dna_hash: &DnaHash,
    seen: &mut std::collections::HashMap<ActionHash, Option<DnaHash>>,
) -> ExternResult<bool> {
    for link in witnesses_for(entry_hash.clone())? {
        let Some(action_hash) = link.target.into_action_hash() else {
            continue;
        };
        let lineage = match seen.get(&action_hash) {
            Some(cached) => cached.clone(),
            None => {
                let resolved = match get(action_hash.clone(), GetOptions::local())? {
                    None => None,
                    Some(record) => record
                        .entry()
                        .to_app_option::<NotarizationWitness>()
                        .map_err(|e| {
                            wasm_error!(WasmErrorInner::Guest(format!(
                                "entry_already_witnessed: could not decode NotarizationWitness \
                                 at {}: {e:?}",
                                record.action_address()
                            )))
                        })?
                        .map(|witness| witness.lineage_dna_hash),
                };
                seen.insert(action_hash, resolved.clone());
                resolved
            }
        };
        if lineage.as_ref() == Some(lineage_dna_hash) {
            return Ok(true);
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
// STATION 7 — RE-ADOPTION BEFORE SUNSET (Holochain Evolution Epic §4 step 4,
// §7 C14; Task 13b)
//
// The revert's other half. `carry_from` moves a v1 fact FORWARD into v2 under
// a witness; `readopt_from` moves a WINDOW-TIME v2 fact BACK onto v1 when the
// elohim revoke the migration commitment inside its horizon.
//
// It is deliberately NOT the mirror image of `carry_from`, and the asymmetry
// is the whole point: **the v1 line has no witness type**. There is nowhere on
// v1 to record "this action was authored on v2 and signed there", so a
// re-adoption cannot be a CARRY. It is a RE-AUTHORING — the agent writes its
// own fact again, natively, on the chain it is returning to. The v2 action and
// its signature stay where they were authored, in the disabled-but-intact v2
// cell, which is the evidence §7 C14 keeps.
//
// Three consequences follow, and each is enforced below:
//
//   * **Own records only.** A courier cannot re-adopt for an absent author,
//     because a courier would have to author another agent's fact as its own.
//     Records of other authors are ignored — the honest-absence report (§7 C4)
//     is the storage side's `pending` count, computed against `v2_total`, not
//     a number this zome can fabricate.
//   * **Entry-hash continuity or nothing.** The re-created entry must hash to
//     exactly what the v2 action committed to; schema drift between the
//     lineage ends is refused rather than silently minting a new entry hash
//     under an old fact's name.
//   * **A v2-only entry type is not an error.** v2's `NotarizationWitness` is
//     an entry-type NAME v1 does not host (Task 26 — the successor's entry-def
//     index is never read here). It is counted `foreign` and skipped:
//     witnesses are v2's own bookkeeping about the crossing, never a fact of
//     v1's, and refusing the page over one would make revert impossible for
//     exactly the chains that took the crossing.
//
// Coordinator-only, and UNCONDITIONAL: it must exist on the PRISTINE v1
// artifact, which is built without `lineage-witness`. That is safe because the
// DNA hash covers integrity zomes and modifiers only — the default
// `node-registry.dna` hash is unmoved by this section.
// ============================================================================

/// Input to [`readopt_from`]: which successor cell to read back from, where to
/// resume, and how many records to take.
///
/// `limit` is clamped to [`READOPT_CAP`]. Re-adoption commits one entry per
/// record on the page, so the page size IS the write batch — it is bounded for
/// the same reason `carry_from`'s is (§7 C3 liveness, C11 backpressure).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadoptInput {
    /// The successor cell to read this agent's window-time records from. Its
    /// DNA is NOT checked against a declared lineage: v1 predates the crossing
    /// and declares no successor, so there is nothing on this chain to check
    /// against. The caller (the storage vehicle, driving a revert it verified
    /// against the revoked commitment) names the cell.
    pub v2_cell: CellId,
    pub cursor: Option<u32>,
    pub limit: u32,
}

/// What one page of re-adoption produced.
///
/// Every count is scoped to the PAGE. The whole-sweep question — "did every
/// window-time fact come home?" — is the driver's, answered by summing
/// `readopted + already_present` across pages against [`Self::v2_total`], with
/// `foreign` and other authors' records subtracted. This receipt never claims
/// completeness on its own.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ReadoptReceipt {
    /// How many of this agent's own v2 records were re-created natively on
    /// this chain by THIS page — new actions over the SAME entry hashes.
    pub readopted: u32,
    /// How many were already on this chain before the page ran, found by one
    /// `query` over the page's entry hashes (§7 C6b, the same idempotency
    /// discipline Task 20 gave `carry_from`).
    ///
    /// On the FIRST sweep of a crossing this is normally non-zero and that is
    /// correct: v2's chain opens with the carried re-creations of v1's own
    /// records, whose entry hashes are on v1 already. `already_present` is
    /// therefore "v1 has this fact", never "a retry ran".
    pub already_present: u32,
    /// How many records on the page carried an app entry type THIS DNA does
    /// not know — v2's `NotarizationWitness` above all. Skipped, counted, and
    /// never an error: see the section comment.
    ///
    /// `#[serde(default)]` for the same additive reason every other receipt
    /// field on this zome carries it — a decoder built against a receipt
    /// shape without this field reads 0 rather than failing.
    #[serde(default)]
    pub foreign: u32,
    /// Resume token for the next page, or `None` when the successor's export
    /// is exhausted. `readopt_from` reads a successor cell's OWN chain
    /// (`export_records`), so exhaustion here is end-of-chain — never the
    /// held path's end-of-local-view.
    pub next_cursor: Option<u32>,
    /// The successor's whole-chain digest, reported verbatim from its export
    /// page so a driver can check that a multi-page sweep drew from ONE chain.
    pub v2_digest: String,
    /// The successor's app-record count, READ from [`ExportPage::total`] and
    /// never derived from `readopted` — so the driver's completeness check can
    /// actually fail. `None` when the successor bundle predates that field.
    pub v2_total: Option<u32>,
}

/// One page of re-adoption commits at most this many entries.
const READOPT_CAP: u32 = 16;

/// Re-author this agent's own window-time successor records back onto THIS
/// chain (Holochain Evolution Epic §4 step 4 — Station 7, revert before
/// sunset).
///
/// Runs ON THE PREDECESSOR (v1) cell and reaches ACROSS to `v2_cell` for one
/// bounded page of that cell's own export. Same agent on both sides, so no
/// capability secret is presented.
///
/// Idempotent by entry hash: a record whose entry is already on this chain is
/// counted `already_present` and skipped, so a retried page re-creates nothing
/// and the sweep can be driven defensively.
#[hdk_extern]
pub fn readopt_from(input: ReadoptInput) -> ExternResult<ReadoptReceipt> {
    let me = agent_info()?.agent_initial_pubkey;
    let limit = input.limit.clamp(1, READOPT_CAP);

    // (1) one bounded page of the successor's OWN chain, across the cell
    //     boundary. `export_records` is unconditional on both lineage ends.
    let response = call(
        CallTargetCell::OtherCell(input.v2_cell.clone()),
        "node_registry_coordinator",
        "export_records".into(),
        None,
        ExportInput {
            cursor: input.cursor,
            limit,
            // Unpinned: re-adoption pages are bounded by READOPT_CAP over a
            // short window-time tail, and `ReadoptInput`/`ReadoptReceipt` carry
            // no resume field to thread one through. The successor's export is
            // linear per page regardless (Task 24), so this pays only the
            // whole-chain digest per page.
            resume: None,
        },
    )?;
    let page: ExportPage = match response {
        ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: could not decode the successor's ExportPage: {e:?}"
            )))
        })?,
        other => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: the successor cell refused export_records: {other:?}"
            ))))
        }
    };

    // `records` and `entries` are paired POSITIONALLY — a page whose two
    // vectors disagree in length cannot be paired at all, and re-authoring the
    // wrong bytes under a fact's name is the one failure this whole station
    // exists to prevent. Refuse the page.
    if page.records.len() != page.entries.len() {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "readopt_from: the successor returned {} records but {} entries — the page cannot \
             be paired positionally",
            page.records.len(),
            page.entries.len()
        ))));
    }

    // (2) which app entry types THIS DNA hosts, and how the successor NAMES
    //     the ones on its page (Task 26, G10). A name this end does not host is
    //     `foreign` — asked here, BEFORE any deserialization, because §4 step 4
    //     requires a v2-only type to be a COUNT and never an error.
    //
    //     The successor's entry-def INDEX is never trusted: v2 appends
    //     `NotarizationWitness` to the same enum v1 defines, so the two ends
    //     agree on today's indexes only by accident of ordering, and a
    //     successor that reordered or removed a type would make an index-keyed
    //     re-adoption re-author records as the wrong type. The one exception is
    //     a successor that predates the named export, which `CarriedTypes` logs
    //     and falls back for (§7 C10).
    let table = local_entry_types()?;
    let carried_types = CarriedTypes::read(&page, "readopt_from")?;

    // (3) what this chain already holds, in ONE query over the page's
    //     candidate entry hashes rather than one query per record.
    let candidates: HashSet<EntryHash> = page
        .records
        .iter()
        .enumerate()
        .filter(|(_, signed)| signed.action().author() == &me)
        .filter_map(|(i, signed)| match signed.action().entry_type() {
            Some(EntryType::App(def))
                if matches!(
                    carried_types.resolve(&table, i, def),
                    LocalTypeMatch::Known(_)
                ) =>
            {
                signed.action().entry_hash().cloned()
            }
            _ => None,
        })
        .collect();
    let mut on_chain: HashSet<EntryHash> = if candidates.is_empty() {
        HashSet::new()
    } else {
        query(
            ChainQueryFilter::new()
                .entry_hashes(candidates)
                .include_entries(false),
        )?
        .iter()
        .filter_map(|r| r.action().entry_hash().cloned())
        .collect()
    };

    let mut readopted: u32 = 0;
    let mut already_present: u32 = 0;
    let mut foreign: u32 = 0;
    // What the successor CALLED the types this end could not host, deduplicated
    // and logged once for the whole page. `foreign` is the number the receipt
    // reports; this is the diagnosis a driver needs when that number surprises
    // it — "which type did v1 refuse to hold?" — without a host call per record.
    let mut foreign_named: Vec<String> = Vec::new();

    for (i, signed) in page.records.iter().enumerate() {
        let action = signed.action();

        // Another agent's fact. Re-adoption re-authors the agent's OWN facts
        // only — there is no witness type on this line to carry someone
        // else's, so a courier here would be forging authorship. Ignored, and
        // deliberately uncounted: the driver reports the residue as `pending`
        // against `v2_total`, which is the honest number.
        if action.author() != &me {
            continue;
        }

        let Some(EntryType::App(def)) = action.entry_type() else {
            // `export_records` filters to app entries, so this is unreachable
            // on a well-formed page; skipping is the conservative read.
            continue;
        };

        // A v2-only entry type (its `NotarizationWitness`), recognised BY THE
        // NAME the successor published for it. Skipped, counted, never an
        // error.
        let unit = match carried_types.resolve(&table, i, def) {
            LocalTypeMatch::Known(unit) => unit,
            LocalTypeMatch::Foreign(claimed) => {
                foreign = foreign.saturating_add(1);
                if !foreign_named.contains(&claimed) {
                    foreign_named.push(claimed);
                }
                continue;
            }
        };

        let entry_hash = action.entry_hash().ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: record {i}: an app entry was exported but its action references \
                 no entry hash"
            )))
        })?;

        // Already here — either carried onto this chain before the crossing,
        // or re-adopted by an earlier run of this same page.
        if on_chain.contains(entry_hash) {
            already_present = already_present.saturating_add(1);
            continue;
        }

        let Some(entry) = page.entries.get(i).cloned().flatten() else {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: record {i}: the successor exported an app action with no entry \
                 bytes — the fact cannot be re-authored from a page that does not carry it"
            ))));
        };

        // Deserialized as the type THIS DNA hosts under the name the successor
        // published — never as whatever the carried index names here. A failure
        // is real drift between the ends (a shared name over changed bytes) and
        // is refused, unlike an unhosted name, which was counted `foreign`
        // above.
        let typed = EntryTypes::try_from((unit, &entry)).map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: record {i}: could not deserialize the exported entry as `{}`, the \
                 entry type the successor named for it: {e:?}",
                entry_def_name(unit)
            )))
        })?;

        // CID continuity is the whole promise: the re-authored entry must hash
        // to exactly what the v2 action committed to. If the two lineage ends
        // disagree about the struct's shape the round-trip silently produces
        // DIFFERENT bytes — a new fact wearing an old fact's name. Refuse.
        let recreated_hash = hash_entry(&typed)?;
        if &recreated_hash != entry_hash {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "readopt_from: record {i}: re-authored entry hash differs from the successor \
                 action's — schema drift between lineage ends (re-authored {recreated_hash}, \
                 successor action commits to {entry_hash})"
            ))));
        }

        create_entry(typed)?;
        // Guard an intra-page duplicate too: `on_chain` was a pre-loop
        // snapshot, so without this a page carrying the same entry hash twice
        // would mint two actions over it.
        on_chain.insert(recreated_hash);
        readopted = readopted.saturating_add(1);
    }

    if !foreign_named.is_empty() {
        debug!(
            "readopt_from: skipped {foreign} record(s) this DNA hosts no entry type for: {}. \
             Expected for the successor's own crossing bookkeeping; anything else means the two \
             lineage ends disagree about which facts v1 can hold.",
            foreign_named.join(", ")
        );
    }

    Ok(ReadoptReceipt {
        readopted,
        already_present,
        foreign,
        next_cursor: page.next_cursor,
        v2_digest: page.digest,
        v2_total: page.total,
    })
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
    /// **Additive.** The [`ExportResume`] the previous page's
    /// [`CarryReceipt::resume`] carried, passed through verbatim to whichever
    /// export this page calls. `#[serde(default)]` keeps the landed storage
    /// driver — which sends no such key — byte-compatible, and a sweep that
    /// omits it simply pays the first-page cost on every page.
    ///
    /// Pass-through by design: this cell never mints or edits a resume, because
    /// the token describes the PREDECESSOR's chain and only the predecessor can
    /// speak to it. Carrying someone else's pin unchanged is the whole contract.
    #[serde(default)]
    pub resume: Option<ExportResume>,
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
    /// **Additive.** The predecessor's [`ExportResume`] for this walk, reported
    /// verbatim from [`ExportPage::resume`]. Hand it back on the next
    /// [`CarryInput`] and the predecessor stops re-walking its whole chain per
    /// page (Task 24, G8); ignore it and nothing changes.
    ///
    /// `#[serde(default)]` so a consumer built against the earlier receipt still
    /// decodes, reading `None` for a page from a zome that predates the field.
    #[serde(default)]
    pub resume: Option<ExportResume>,
    /// **Additive (Task 29).** WHICH read answered the export this page carried,
    /// reported verbatim from [`ExportPage::view`]: [`HELD_VIEW_AUTHORITY`],
    /// [`HELD_VIEW_LOCAL_ONLY`], [`HELD_VIEW_UNREACHABLE`], or `None` on a
    /// [`CarrySource::Own`] page, where the predecessor read its own chain
    /// locally and none of the three applies.
    ///
    /// **This is what makes a held receipt readable at all.** Station 6
    /// measured a courier whose held view of a neighbour froze at 212 records
    /// against an `observed_head` of 732 — a partial-arc local read that never
    /// fills — and nothing in the receipt said so, so the sweep recorded a
    /// permanent truncation as a crossing. A driver reads
    /// [`HELD_VIEW_LOCAL_ONLY`] on a held receipt as "this number is scoped to
    /// one peer's arc, do not record it as the neighbour's chain".
    ///
    /// It is a confidence label, never a completeness proof: `authority` says
    /// the read reached the agent-activity authorities, not that they were
    /// caught up. [`HELD_VIEW_UNREACHABLE`] is the sharpest of the three — a
    /// receipt carrying it recorded nothing anyone confirmed, and the sweep
    /// should re-ask rather than bank it. The completeness check stays the
    /// comparison of [`Self::v1_observed_head`] across views.
    ///
    /// `#[serde(default)]` so a consumer built against the earlier receipt still
    /// decodes, reading `None` — honestly "not reported" — for a page from a
    /// zome that predates the field.
    #[serde(default)]
    pub view: Option<String>,
    /// **Additive (Task 29).** How many action rows the predecessor's POSITION
    /// scan read to find where this page starts, reported verbatim from
    /// [`ExportPage::scanned`] — **the metric risk row R1 reads**.
    ///
    /// The producer half of a mirror the storage side landed ahead of it at
    /// Task 28 (`release_adoption::carry::CarryReceipt::scanned`, which decodes
    /// `None` as "not reported" rather than a fabricated 0). Carry cost stays
    /// linear in chain length iff this stays bounded on RESUMED pages and,
    /// the property that actually matters, independent of how far into the
    /// chain the page sits.
    ///
    /// On the held path it is the size of the agent-activity list the answering
    /// view returned — so it moves with [`Self::view`], and a `local-only` page
    /// reporting a small `scanned` is reporting a small ARC, not a cheap walk.
    ///
    /// `#[serde(default)]`; `None` from a page that predates the field, never a
    /// fabricated 0.
    #[serde(default)]
    pub scanned: Option<u32>,
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
    let lineage_dna_hash = declared_predecessor()?;
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
                resume: input.resume.clone(),
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
                    resume: input.resume.clone(),
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

    carry_page(page, held, lineage_dna_hash, me)
}

/// This DNA's declared predecessor, read from its identity-bearing properties.
///
/// Factored out of [`carry_from`] so `carry_page_for_test` reaches the same
/// lineage through the same read — a test entry point that trusted a different
/// predecessor would not be exercising the crossing.
#[cfg(feature = "lineage-witness")]
fn declared_predecessor() -> ExternResult<DnaHash> {
    let properties: LineageProperties =
        dna_info()?.modifiers.properties.try_into().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "carry_from: could not deserialize DNA properties: {e:?}"
            )))
        })?;
    properties.lineage.first().cloned().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "carry_from: this DNA declares no lineage — there is no predecessor to carry from"
                .to_string(),
        ))
    })
}

/// Carry ONE already-fetched [`ExportPage`] onto this chain.
///
/// Split out of [`carry_from`] at Task 26 and otherwise unchanged: `carry_from`
/// is now (verify the lineage · fetch the page · carry it), and this is the
/// third step. The split is what lets `carry_page_for_test` drive a page the
/// zome did not fetch through exactly this code, rather than a paraphrase of it
/// — a refusal proven by a second implementation proves nothing about the
/// first.
///
/// `held` is the caller's statement about WHOSE chain the page came from, and
/// it gates native re-creation: a courier carries, it does not author.
#[cfg(feature = "lineage-witness")]
fn carry_page(
    page: ExportPage,
    held: bool,
    lineage_dna_hash: DnaHash,
    me: AgentPubKey,
) -> ExternResult<CarryReceipt> {
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
    //
    // The types a self-carry re-creates are resolved by NAME (Task 26, G10):
    // the predecessor's entry-def index means nothing on this DNA, which
    // appends `NotarizationWitness` to the same enum. Read once per page, and
    // read even on a held page so the length check on `type_names` fires there
    // too — a malformed page is malformed whoever authored its records.
    let type_table = local_entry_types()?;
    let carried_types = CarriedTypes::read(&page, "carry_from")?;
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
    // Page-scoped memo for the held-carry idempotency check (Task 24, G8) — the
    // records of one page were, if already carried at all, almost always
    // carried by ONE prior witness, so without this the same witness record is
    // fetched and decoded once per record. Scoped to the page and dropped with
    // it: nothing here outlives the call, so a witness committed by an earlier
    // page of the same sweep is still read fresh.
    let mut witness_lineage: std::collections::HashMap<ActionHash, Option<DnaHash>> =
        std::collections::HashMap::new();

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
                if entry_already_witnessed(entry_hash, &lineage_dna_hash, &mut witness_lineage)? {
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
                // The type comes from the NAME the predecessor published, never
                // from the index it carried (Task 26, G10). A name this DNA
                // does not host is a REFUSAL on this arm, not a `foreign`
                // count: the re-adoption path may skip a fact it cannot hold,
                // but a successor that cannot host its own predecessor's type
                // has no honest way to re-create the record, and falling
                // through would carry it as bytes while reporting a self-carry.
                let unit = match carried_types.resolve(&type_table, i, def) {
                    LocalTypeMatch::Known(unit) => unit,
                    LocalTypeMatch::Foreign(claimed) => {
                        return Err(wasm_error!(WasmErrorInner::Guest(format!(
                            "carry_from: proof {i}: the predecessor named {claimed} for a record \
                             THIS AGENT authored, and this DNA hosts no such entry type. Entry \
                             types travel by NAME across lineage ends — refusing rather than \
                             re-creating the record as whatever type the carried entry-def index \
                             happens to name here."
                        ))));
                    }
                };
                let typed = EntryTypes::try_from((unit, entry)).map_err(|e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "carry_from: proof {i}: could not deserialize the carried entry as `{}`, \
                         the entry type the predecessor named for it: {e:?}",
                        entry_def_name(unit)
                    )))
                })?;
                // The whole promise of self-carry is CID continuity: the
                // re-created entry must hash to exactly what the carried
                // action commits to. If the two lineage ends disagree about
                // the struct's shape — or the page named a type that is not
                // this record's — the round-trip silently produces DIFFERENT
                // bytes, a new entry hash under an old action's proof. Refuse
                // the page; never fall through to `entry: None`, which would
                // drop the only copy of the bytes the witness needed.
                let recreated_hash = hash_entry(&typed)?;
                let committed = action.entry_hash().ok_or_else(|| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "carry_from: proof {i}: an app entry was carried but the action \
                         references no entry hash"
                    )))
                })?;
                if &recreated_hash != committed {
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "carry_from: proof {i}: re-created entry hash differs from the carried \
                         action's — schema drift between lineage ends, or a page that named the \
                         wrong type for this record (re-created as `{}` to {recreated_hash}, \
                         carried action commits to {committed})",
                        entry_def_name(unit)
                    ))));
                }
                create_entry(typed)?;
                recreated = true;
                self_carried = self_carried.saturating_add(1);
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
        resume: page.resume,
        // Both reported VERBATIM from the page. This cell never mints or
        // re-derives either: `view` is the predecessor's statement about which
        // store answered its read, and `scanned` is what that read cost — two
        // facts only the exporting cell can speak to.
        view: page.view,
        scanned: page.scanned,
    })
}

/// Carry ONE hand-built [`ExportPage`] through exactly the carry [`carry_from`]
/// runs on a page it fetched (Holochain Evolution Epic Task 26, G10).
///
/// **This exists because the refusal it proves cannot be reached any other
/// way.** `carry_from` fetches its page from a predecessor CELL, so a test can
/// only ever hand it an HONEST page — one whose `type_names` a real export
/// produced. The whole point of G10 is what happens when those names are wrong:
/// a predecessor whose entry-type order differs from this DNA's, or one that
/// simply lies. There is no injection point on the honest path, and a refusal
/// nothing can exercise is a claim rather than a check.
///
/// It authors nothing a `carry_from` page would not: the same declared
/// predecessor, the same [`carry_page`], the same witness. `held` is `false`
/// because the refusal being exercised is the SELF-CARRY one — a held page
/// re-creates nothing and so resolves no types at all.
///
/// # It does not ship (Task 26 review, addressed in Task 24 fix round 1)
///
/// `lineage-witness` alone is the DEPLOYABLE successor, and this extern must not
/// be in it: it is an unauthenticated injection point for hand-built pages, and
/// a zome call is a zome call — "for_test" in the name is a comment, not a
/// fence. It is therefore gated on `lineage-witness` AND a second feature,
/// `lineage-test`, which only the sweettest-facing build passes
/// (`just build-witness-test`). `just build-witness` — the bundle a conductor
/// installs — omits it.
///
/// The two bundles are DNA-hash-IDENTICAL, because the difference is
/// coordinator-only. That is not a loophole, it is the point: the sweettest
/// exercises the same DNA the fleet runs, and only the test bundle can be
/// talked to through this door.
#[cfg(all(feature = "lineage-witness", feature = "lineage-test"))]
#[hdk_extern]
pub fn carry_page_for_test(page: ExportPage) -> ExternResult<CarryReceipt> {
    let lineage_dna_hash = declared_predecessor()?;
    let me = agent_info()?.agent_initial_pubkey;
    carry_page(page, false, lineage_dna_hash, me)
}

// ============================================================================
// STATION 8 — THE SEAL (Holochain Evolution Epic §4 step 5, §8)
//
// Gated behind `lineage-witness` and APPENDED at the end of the file, for the
// same reason the sections above are: the default build's compiled output must
// not be perturbed by this section's mere physical presence in the source text.
//
// The sunset is a separate notarized act, taken only after fleet convergence
// AND a `sunsets-lineage` commitment. `seal_close` is its DNA half, in the one
// order that makes it irreversible:
//
//   1. v1 `close_chain(Dna(v2))`  — the predecessor declares its close;
//   2. v2 `open_chain(Dna(v1), close_hash)` — the successor names it, so the
//      crossing is on BOTH chains and the far end is discoverable from either;
//   3. v2 `commit_witness([the CloseChain proof])` — the close becomes a FACT
//      in v2's DHT, which is what makes the integrity zome's after-close rule
//      (`refuse_carried_after_close`) able to fire on every later witness;
//   4. an `AuthorToClose` link from an anchor over (lineage, author) to that
//      seal witness — the coordinator-side read index. Validation does NOT
//      traverse it (HDI has no `get_links`; see the integrity zome's recorded
//      deviation); the vehicle and the passport do.
//
// The v1 chain stays READABLE forever: nothing here deletes, disables or
// rewrites it. Disabling the v1 CELL is the storage controller's half of the
// fence (Task 14b) — this extern never touches installation state.
// ============================================================================

/// What one [`seal_close`] produced.
///
/// Every hash is rendered as canonical base64, NOT a native `HoloHash`, for
/// exactly the reason [`CarryReceipt::witness_hash`] documents: a `HoloHash`
/// serialises to a msgpack BYTE ARRAY and the storage-side decoder reads a
/// `String`. The zome renders; storage never re-derives a hash.
#[cfg(feature = "lineage-witness")]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SealReceipt {
    /// The predecessor's `CloseChain` action, on the v1 chain.
    pub close_hash: String,
    /// This chain's `OpenChain` action, which names `close_hash`.
    pub open_hash: String,
    /// The witness that carried the close into v2, or `""` when a prior seal
    /// was found without one (see [`SealReceipt::already_sealed`]).
    pub witness_hash: String,
    /// `true` when this call found an existing seal for the same lineage on
    /// this chain and authored NOTHING.
    ///
    /// A sunset is irreversible, so a retried vehicle step must never author a
    /// SECOND `CloseChain` on the predecessor — there is no un-closing it. The
    /// guard is one local `query`, and the retry reads as the same receipt.
    pub already_sealed: bool,
    /// **Additive.** `true` when this call found a HALF-SEAL — v1 already
    /// closed toward this DNA, but this chain had no matching `OpenChain` — and
    /// RESUMED at the open step rather than closing v1 again.
    ///
    /// The window is real: [`seal_close`] closes v1 through a cross-cell call
    /// and then opens here, and a failure between those two steps leaves v1
    /// closed with nothing on this side to key idempotency on. Closing again
    /// would author a second `CloseChain`, which Probe B2 measured the remote
    /// authority refusing with a WARRANT against the author — a sunset that
    /// warrants the peer performing it. So the retry probes v1 first.
    ///
    /// `#[serde(default)]` so a consumer built against the pre-resume receipt
    /// still decodes, reading `false`.
    #[serde(default)]
    pub resumed: bool,
}

/// How far back from v1's chain head [`seal_close`] looks for an existing
/// `CloseChain` toward this DNA before authoring one.
///
/// A bound, not a guess: the half-seal window is the gap between two adjacent
/// steps of ONE seal, so an existing close sits at or within a handful of
/// actions of v1's head. Post-close writes are exactly what the after-close
/// rule forbids, so a chain with 32 actions above its close is already outside
/// the sunset's honest shape. Bounded because each step back is a cross-cell
/// `get`, and an unbounded scan would make every seal pay for v1's whole chain.
#[cfg(feature = "lineage-witness")]
const HALF_SEAL_SCAN: u32 = 32;

/// The seal this chain already holds for `lineage_dna_hash`, if any.
///
/// Keyed on the `OpenChain` action, because that is the one action a sealed
/// chain must carry and it names the predecessor's close hash directly.
///
/// The seal WITNESS is then keyed on THAT close hash, not merely on "a witness
/// from this lineage carrying some close": a courier could have carried another
/// agent's close from the same lineage, and reporting it as this chain's seal
/// witness would hand the vehicle a receipt pointing at somebody else's act.
/// The carried action is re-hashed and compared.
#[cfg(feature = "lineage-witness")]
fn existing_seal(lineage_dna_hash: &DnaHash) -> ExternResult<Option<SealReceipt>> {
    let target = MigrationTarget::Dna(lineage_dna_hash.clone());
    let records = query(ChainQueryFilter::new().include_entries(true))?;

    // (1) the OpenChain — the seal's own record on this chain.
    let mut open: Option<(ActionHash, ActionHash)> = None;
    for record in &records {
        if let ActionData::OpenChain(data) = &record.action().data {
            if data.prev_target == target {
                open = Some((record.action_address().clone(), data.close_hash.clone()));
            }
        }
    }
    let Some((open_hash, close_hash)) = open else {
        return Ok(None);
    };

    // (2) the witness carrying THAT close.
    let mut witness_hash: Option<ActionHash> = None;
    for record in &records {
        if witness_hash.is_some() {
            break;
        }
        if !matches!(record.action().data, ActionData::Create(_)) {
            continue;
        }
        // A record that is not a witness simply fails to decode as one; that is
        // a skip, never an error, because this walk crosses every app entry on
        // the chain.
        let Ok(Some(w)) = record.entry().to_app_option::<NotarizationWitness>() else {
            continue;
        };
        if &w.lineage_dna_hash != lineage_dna_hash {
            continue;
        }
        for proof in &w.proofs {
            if !matches!(proof.action.data, ActionData::CloseChain(_)) {
                continue;
            }
            if hash_action(proof.action.clone())? == close_hash {
                witness_hash = Some(record.action_address().clone());
                break;
            }
        }
    }

    Ok(Some(SealReceipt {
        close_hash: close_hash.to_string(),
        open_hash: open_hash.to_string(),
        witness_hash: witness_hash.map(|h| h.to_string()).unwrap_or_default(),
        already_sealed: true,
        resumed: false,
    }))
}

/// v1's existing `CloseChain` toward `my_dna_hash`, if it already has one.
///
/// The half-seal probe. Reads v1 through externs the PRISTINE v1 coordinator
/// already exports — `my_chain_activity` for the (seq, hash) pairs and
/// `get_record_at` for each candidate's action data — so it needs no v1
/// redeploy. Walks newest-first and stops at [`HALF_SEAL_SCAN`] actions below
/// v1's head.
#[cfg(feature = "lineage-witness")]
fn v1_close_toward(v1_cell: &CellId, my_dna_hash: &DnaHash) -> ExternResult<Option<ActionHash>> {
    let activity: AgentActivityStatus = match call(
        CallTargetCell::OtherCell(v1_cell.clone()),
        "node_registry_coordinator",
        "my_chain_activity".into(),
        None,
        (),
    )? {
        ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "seal_close: could not decode the predecessor's chain activity: {e:?}"
            )))
        })?,
        other => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "seal_close: the predecessor cell refused my_chain_activity: {other:?}"
            ))))
        }
    };

    let mut seen = activity.valid_activity;
    seen.sort_by_key(|a| std::cmp::Reverse(a.0));
    let Some((head_seq, _)) = seen.first().cloned() else {
        return Ok(None);
    };

    let wanted = MigrationTarget::Dna(my_dna_hash.clone());
    for (seq, action_hash) in seen {
        if head_seq.saturating_sub(seq) > HALF_SEAL_SCAN {
            break;
        }
        let record: Option<Record> = match call(
            CallTargetCell::OtherCell(v1_cell.clone()),
            "node_registry_coordinator",
            "get_record_at".into(),
            None,
            (action_hash.clone(), true),
        )? {
            ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "seal_close: could not decode the predecessor's record at {action_hash}: {e:?}"
                )))
            })?,
            other => {
                return Err(wasm_error!(WasmErrorInner::Guest(format!(
                    "seal_close: the predecessor cell refused get_record_at: {other:?}"
                ))))
            }
        };
        let Some(record) = record else {
            continue;
        };
        if let ActionData::CloseChain(data) = &record.action().data {
            if data.new_target.as_ref() == Some(&wanted) {
                return Ok(Some(action_hash));
            }
        }
    }

    Ok(None)
}

/// Seal the crossing: close the predecessor chain toward THIS DNA, open this
/// chain from that close, and witness the close here.
///
/// `v1_cell` must name the DNA this DNA declares as its predecessor — read from
/// this DNA's own `lineage` property, which folds into the DNA hash, so the
/// check is one every peer agrees on. Same agent on both cells, so the
/// cross-cell call presents no capability secret (measured on 0.7.0).
///
/// Idempotent at BOTH points a retry can arrive at. A completed seal is keyed
/// on the `OpenChain` already on this chain (`already_sealed: true`, nothing
/// authored). A HALF-seal — v1 closed, this side empty — is keyed on v1's own
/// chain, and resumes at the open step (`resumed: true`) rather than closing v1
/// a second time.
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn seal_close(v1_cell: CellId) -> ExternResult<SealReceipt> {
    // (1) the declared predecessor, from this DNA's identity-bearing properties.
    let properties: LineageProperties =
        dna_info()?.modifiers.properties.try_into().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "seal_close: could not deserialize DNA properties: {e:?}"
            )))
        })?;
    let lineage_dna_hash = properties.lineage.first().cloned().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(
            "seal_close: this DNA declares no lineage — there is no predecessor to seal"
                .to_string(),
        ))
    })?;
    if v1_cell.dna_hash() != &lineage_dna_hash {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "seal_close: v1_cell names DNA {}, but this DNA declares {} as its predecessor",
            v1_cell.dna_hash(),
            lineage_dna_hash
        ))));
    }

    // (2) never seal twice — a CloseChain cannot be taken back.
    if let Some(sealed) = existing_seal(&lineage_dna_hash)? {
        return Ok(sealed);
    }

    // (3) close v1 toward this DNA — unless it is ALREADY closed toward us.
    //
    // The half-seal window. Step (2) keys idempotency on this chain's own
    // OpenChain, which does not exist yet when a call dies between the close
    // below and the open at (5): v1 is closed, this side holds nothing, and a
    // naive retry would author a SECOND CloseChain. Probe B2 measured what that
    // costs — the remote authority rejects the action after a close and issues
    // a WARRANT against its author, so a re-close warrants the very peer
    // performing the sunset. Probe v1 first and resume at the open step.
    let my_dna_hash = dna_info()?.hash;
    let (close_hash, resumed): (ActionHash, bool) = match v1_close_toward(&v1_cell, &my_dna_hash)? {
        Some(existing) => (existing, true),
        None => {
            let fresh: ActionHash = match call(
                CallTargetCell::OtherCell(v1_cell.clone()),
                "node_registry_coordinator",
                "close_chain_for".into(),
                None,
                my_dna_hash,
            )? {
                ZomeCallResponse::Ok(io) => io.decode().map_err(|e| {
                    wasm_error!(WasmErrorInner::Guest(format!(
                        "seal_close: could not decode the predecessor's CloseChain hash: {e:?}"
                    )))
                })?,
                other => {
                    return Err(wasm_error!(WasmErrorInner::Guest(format!(
                        "seal_close: the predecessor cell refused close_chain_for: {other:?}"
                    ))))
                }
            };
            (fresh, false)
        }
    };

    // (4) read the close back, SIGNED — that signed action is the proof the
    //     witness carries, and it is what lets v2's validators know the close's
    //     `action_seq` without any access to v1.
    let signed_close: SignedActionHashed = match call(
        CallTargetCell::OtherCell(v1_cell.clone()),
        "node_registry_coordinator",
        "get_signed_action".into(),
        None,
        close_hash.clone(),
    )? {
        ZomeCallResponse::Ok(io) => io
            .decode::<Option<SignedActionHashed>>()
            .map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "seal_close: could not decode the predecessor's signed CloseChain: {e:?}"
                )))
            })?
            .ok_or_else(|| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "seal_close: the predecessor cell has no record of the CloseChain it just \
                     authored at {close_hash}"
                )))
            })?,
        other => {
            return Err(wasm_error!(WasmErrorInner::Guest(format!(
                "seal_close: the predecessor cell refused get_signed_action: {other:?}"
            ))))
        }
    };

    // (5) open THIS chain from that close — after the close, never before.
    let open_hash = open_chain(MigrationTarget::Dna(lineage_dna_hash.clone()), close_hash.clone())?;

    // (6) carry the close itself into v2 as a proof. A CloseChain references no
    //     entry, so the proof carries no entry bytes.
    let close_author = signed_close.action().header.author.clone();
    let witness_hash = commit_witness(NotarizationWitness {
        lineage_dna_hash: lineage_dna_hash.clone(),
        proofs: vec![CarriedProof {
            action: signed_close.action().clone(),
            signature: signed_close.signature.clone(),
            entry: None,
        }],
    })?;

    // (7) the coordinator-side read index: (lineage, author) -> the seal witness.
    let anchor = StringAnchor {
        anchor_type: "lineage_close".to_string(),
        anchor_value: format!("{lineage_dna_hash}:{close_author}"),
    };
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    create_link(
        anchor_hash,
        witness_hash.clone(),
        LinkTypes::AuthorToClose,
        (),
    )?;

    Ok(SealReceipt {
        close_hash: close_hash.to_string(),
        open_hash: open_hash.to_string(),
        witness_hash: witness_hash.to_string(),
        already_sealed: false,
        resumed,
    })
}

/// Every seal witness this peer holds for `(lineage_dna_hash, author)` — the
/// `AuthorToClose` read index [`seal_close`] authors.
///
/// Validation cannot traverse links (HDI has no `get_links`), so this is a
/// COORDINATOR-side query only: the vehicle and the passport ask it "has this
/// author's predecessor chain been sealed, as far as I can see?". It is
/// evidence, never the fence.
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn get_closes_for(input: (DnaHash, AgentPubKey)) -> ExternResult<Vec<Link>> {
    let (lineage_dna_hash, author) = input;
    let anchor = StringAnchor {
        anchor_type: "lineage_close".to_string(),
        anchor_value: format!("{lineage_dna_hash}:{author}"),
    };
    let anchor_hash = hash_entry(&EntryTypes::StringAnchor(anchor))?;
    let query = LinkQuery::try_new(anchor_hash, LinkTypes::AuthorToClose)?;
    get_links(query, GetStrategy::default())
}
