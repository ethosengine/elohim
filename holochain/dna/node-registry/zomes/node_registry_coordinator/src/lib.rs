use hdk::prelude::*;
use holochain_serialized_bytes::{SerializedBytes, SerializedBytesError};
use node_registry_integrity::*;

// Re-export integrity types for convenience
pub use node_registry_integrity::{
    NodeRegistration, NodeHeartbeat, HealthAttestation, CustodianAssignment,
    EntryTypes, LinkTypes,
};

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
    let my_node = get_node_by_agent(my_agent_info.agent_latest_pubkey)?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(region_anchor_hash, LinkTypes::RegionToNode)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(tier_anchor_hash, LinkTypes::TierToNode)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(content_anchor_hash, LinkTypes::ContentToAssignment)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(node_anchor_hash, LinkTypes::NodeToAssignment)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?.build()
    )?;

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
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Deserialize a NodeRegistration from a Record
fn deserialize_node_registration(record: &Record) -> ExternResult<Option<NodeRegistration>> {
    match record.entry().as_option() {
        Some(entry) => {
            let sb: SerializedBytes = entry.clone().try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Serialize error: {:?}", e)))
            })?;
            let registration: NodeRegistration = sb.try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
            })?;
            Ok(Some(registration))
        }
        None => Ok(None),
    }
}

/// Deserialize a NodeHeartbeat from a Record
fn deserialize_node_heartbeat(record: &Record) -> ExternResult<Option<NodeHeartbeat>> {
    match record.entry().as_option() {
        Some(entry) => {
            let sb: SerializedBytes = entry.clone().try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Serialize error: {:?}", e)))
            })?;
            let heartbeat: NodeHeartbeat = sb.try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
            })?;
            Ok(Some(heartbeat))
        }
        None => Ok(None),
    }
}

/// Deserialize a HealthAttestation from a Record
fn deserialize_health_attestation(record: &Record) -> ExternResult<Option<HealthAttestation>> {
    match record.entry().as_option() {
        Some(entry) => {
            let sb: SerializedBytes = entry.clone().try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Serialize error: {:?}", e)))
            })?;
            let attestation: HealthAttestation = sb.try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
            })?;
            Ok(Some(attestation))
        }
        None => Ok(None),
    }
}

/// Deserialize a CustodianAssignment from a Record
fn deserialize_custodian_assignment(record: &Record) -> ExternResult<Option<CustodianAssignment>> {
    match record.entry().as_option() {
        Some(entry) => {
            let sb: SerializedBytes = entry.clone().try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Serialize error: {:?}", e)))
            })?;
            let assignment: CustodianAssignment = sb.try_into().map_err(|e: SerializedBytesError| {
                wasm_error!(WasmErrorInner::Guest(format!("Deserialize error: {:?}", e)))
            })?;
            Ok(Some(assignment))
        }
        None => Ok(None),
    }
}

fn get_node_registration_by_id(node_id: String) -> ExternResult<NodeRegistration> {
    let id_anchor = StringAnchor {
        anchor_type: "node_id".to_string(),
        anchor_value: node_id.clone(),
    };
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToNodeRegistration)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(id_anchor_hash, LinkTypes::IdToNodeRegistration)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(custodian_anchor_hash, LinkTypes::CustodianToNode)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(node_anchor_hash, LinkTypes::NodeToHeartbeat)?.build()
    )?;

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

    let links = get_links(
        GetLinksInputBuilder::try_new(subject_anchor_hash, LinkTypes::NodeToAttestations)?.build()
    )?;

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
