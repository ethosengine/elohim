use hdk::prelude::*;
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
    let registration = get_node_registration_by_id(input.node_id.clone())?;
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

    let query = LinkQuery::try_new(region_anchor_hash, LinkTypes::RegionToNode)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut nodes = Vec::new();
    for link in links {
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::NodeRegistration(registration)) =
                record.entry().to_app_option().ok().flatten() {
                nodes.push(registration);
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::NodeRegistration(registration)) =
                record.entry().to_app_option().ok().flatten() {

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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::NodeRegistration(registration)) =
                record.entry().to_app_option().ok().flatten() {
                nodes.push(registration);
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::CustodianAssignment(assignment)) =
                record.entry().to_app_option().ok().flatten() {
                assignments.push(assignment);
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::CustodianAssignment(assignment)) =
                record.entry().to_app_option().ok().flatten() {
                assignments.push(assignment);
            }
        }
    }

    Ok(assignments)
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::NodeRegistration(registration)) =
                record.entry().to_app_option().ok().flatten() {

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
            from_custodians: find_other_custodians(&assignment.content_id, &failed_node_id)?,
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

    if let Some(record) = get(links[0].target.clone(), GetOptions::default())? {
        if let Some(EntryTypes::NodeRegistration(registration)) =
            record.entry().to_app_option().ok().flatten() {
            return Ok(registration);
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

    Ok(links[0].target.clone().into_action_hash().ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest("Invalid action hash".to_string()))
    })?)
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::NodeRegistration(registration)) =
                record.entry().to_app_option().ok().flatten() {
                if registration.agent_pub_key == agent_key.to_string() {
                    return Ok(registration);
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

    if let Some(record) = get(latest_link.target.clone(), GetOptions::default())? {
        if let Some(EntryTypes::NodeHeartbeat(heartbeat)) =
            record.entry().to_app_option().ok().flatten() {
            return Ok(heartbeat);
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(
        format!("Failed to retrieve heartbeat for node: {}", node_id)
    )))
}

fn get_recent_attestations(node_id: &str, max_age_seconds: u64) -> ExternResult<Vec<HealthAttestation>> {
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
        if let Some(record) = get(link.target, GetOptions::default())? {
            if let Some(EntryTypes::HealthAttestation(attestation)) =
                record.entry().to_app_option().ok().flatten() {

                let attestation_time = parse_timestamp(&attestation.timestamp)?;
                let elapsed = now.as_seconds_and_nanos().0 - attestation_time.as_seconds_and_nanos().0;

                if elapsed <= max_age_seconds {
                    recent_attestations.push(attestation);
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
