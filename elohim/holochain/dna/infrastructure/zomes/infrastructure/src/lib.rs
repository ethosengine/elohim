//! Infrastructure Coordinator Zome
//!
//! Provides functions for doorway registration, heartbeat monitoring,
//! and trust tier computation.
//!
//! Key functions:
//! - register_doorway: Self-registration (operator = author)
//! - record_heartbeat: 60s status updates
//! - record_daily_summary: Midnight aggregation
//! - update_doorway_tier: Recompute trust tier from history

use hdk::prelude::*;
use infrastructure_integrity::*;

// =============================================================================
// Wire Types (re-exported from shared crate)
// =============================================================================

pub use infrastructure_types::{
    ContentServerOutput, DoorwayOutput, FindPublishersInput, FindPublishersOutput,
    HealthAttestationOutput, RecordHealthAttestationInput, RecordHeartbeatInput,
    RecordSummaryInput, RegisterContentServerInput, RegisterDoorwayInput, StorageEndpointInput,
};

// =============================================================================
// Integrity → Wire Type Conversions
// =============================================================================

/// Convert integrity DoorwayRegistration to wire type.
fn doorway_to_wire(
    entry: &infrastructure_integrity::DoorwayRegistration,
) -> infrastructure_types::DoorwayRegistration {
    infrastructure_types::DoorwayRegistration {
        id: entry.id.clone(),
        url: entry.url.clone(),
        operator_agent: entry.operator_agent.clone(),
        operator_human: entry.operator_human.clone(),
        capabilities_json: entry.capabilities_json.clone(),
        reach: entry.reach.clone(),
        region: entry.region.clone(),
        bandwidth_mbps: entry.bandwidth_mbps,
        version: entry.version.clone(),
        tier: entry.tier.clone(),
        registered_at: entry.registered_at.clone(),
        updated_at: entry.updated_at.clone(),
    }
}

/// Convert integrity HealthAttestation to wire type.
fn attestation_to_wire(
    entry: &infrastructure_integrity::HealthAttestation,
) -> infrastructure_types::HealthAttestation {
    infrastructure_types::HealthAttestation {
        attestor_doorway_id: entry.attestor_doorway_id.clone(),
        operator_agent: entry.operator_agent.clone(),
        subject_doorway_id: entry.subject_doorway_id.clone(),
        observed_status: entry.observed_status.clone(),
        response_time_ms: entry.response_time_ms,
        conductor_healthy: entry.conductor_healthy,
        timestamp: entry.timestamp,
    }
}

/// Convert integrity ContentServer to wire type.
fn content_server_to_wire(
    entry: &infrastructure_integrity::ContentServer,
) -> infrastructure_types::ContentServer {
    infrastructure_types::ContentServer {
        content_hash: entry.content_hash.clone(),
        capability: entry.capability.clone(),
        serve_url: entry.serve_url.clone(),
        endpoints: entry
            .endpoints
            .iter()
            .map(|e| infrastructure_types::StorageEndpoint {
                url: e.url.clone(),
                protocol: e.protocol.clone(),
                priority: e.priority,
            })
            .collect(),
        online: entry.online,
        priority: entry.priority,
        region: entry.region.clone(),
        bandwidth_mbps: entry.bandwidth_mbps,
        registered_at: entry.registered_at,
        last_heartbeat: entry.last_heartbeat,
    }
}

// =============================================================================
// Signals for Projection
// =============================================================================

/// Signal types emitted after commits for real-time projection.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum InfrastructureSignal {
    /// DoorwayRegistration was created or updated
    DoorwayCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        doorway: DoorwayRegistration,
        author: AgentPubKey,
    },
    /// DoorwayHeartbeat was recorded
    DoorwayHeartbeatCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        heartbeat: DoorwayHeartbeat,
        author: AgentPubKey,
    },
    /// DoorwayHeartbeatSummary was recorded (daily aggregate)
    DoorwaySummaryCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        summary: DoorwayHeartbeatSummary,
        author: AgentPubKey,
    },
    /// ContentServer was registered or updated
    ContentServerCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        server: ContentServer,
        author: AgentPubKey,
    },
    /// HealthAttestation was recorded (peer observed another doorway)
    HealthAttestationCommitted {
        action_hash: String,
        entry_hash: String,
        attestation: HealthAttestation,
        author: String,
    },
}

// =============================================================================
// Post-Commit Callback
// =============================================================================

/// Post-commit callback - emits signals for projection.
#[hdk_extern]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) -> ExternResult<()> {
    for signed_action in committed_actions {
        let action = signed_action.hashed.content.clone();
        let action_hash = signed_action.hashed.hash.clone();

        let entry_hash = match &action {
            Action::Create(create) => create.entry_hash.clone(),
            Action::Update(update) => update.entry_hash.clone(),
            _ => continue,
        };

        let record = match get(action_hash.clone(), GetOptions::default())? {
            Some(r) => r,
            None => continue,
        };

        let author = action.author().clone();

        if let Some(doorway) = record
            .entry()
            .to_app_option::<DoorwayRegistration>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::DoorwayCommitted {
                action_hash,
                entry_hash,
                doorway,
                author,
            })?;
        } else if let Some(heartbeat) = record
            .entry()
            .to_app_option::<DoorwayHeartbeat>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::DoorwayHeartbeatCommitted {
                action_hash,
                entry_hash,
                heartbeat,
                author,
            })?;
        } else if let Some(summary) = record
            .entry()
            .to_app_option::<DoorwayHeartbeatSummary>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::DoorwaySummaryCommitted {
                action_hash,
                entry_hash,
                summary,
                author,
            })?;
        } else if let Some(server) = record
            .entry()
            .to_app_option::<ContentServer>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::ContentServerCommitted {
                action_hash,
                entry_hash,
                server,
                author,
            })?;
        } else if let Some(attestation) = record
            .entry()
            .to_app_option::<HealthAttestation>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::HealthAttestationCommitted {
                action_hash: action_hash.to_string(),
                entry_hash: entry_hash.to_string(),
                attestation,
                author: author.to_string(),
            })?;
        }
    }

    Ok(())
}

// =============================================================================
// Doorway Registration Functions
// =============================================================================

/// Register a new doorway (self-registration only).
///
/// The operator_agent is set to the calling agent - doorways can only
/// register themselves, not on behalf of others.
#[hdk_extern]
pub fn register_doorway(input: RegisterDoorwayInput) -> ExternResult<DoorwayOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    // Check if doorway already exists with this ID
    if get_doorway_by_id(input.id.clone())?.is_some() {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway with ID '{}' already exists",
            input.id
        ))));
    }

    let doorway = DoorwayRegistration {
        id: input.id.clone(),
        url: input.url,
        operator_agent: agent_info.agent_initial_pubkey.to_string(),
        operator_human: None,
        capabilities_json: input.capabilities_json,
        reach: input.reach,
        region: input.region.clone(),
        bandwidth_mbps: input.bandwidth_mbps,
        version: input.version,
        tier: "Emerging".to_string(),
        registered_at: timestamp.clone(),
        updated_at: timestamp,
    };

    let action_hash = create_entry(&EntryTypes::DoorwayRegistration(doorway.clone()))?;

    // Create ID lookup link
    let id_anchor = StringAnchor::new("doorway_id", &input.id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToDoorway,
        (),
    )?;

    // Create operator lookup link
    let operator_anchor = StringAnchor::new("doorway_operator", &doorway.operator_agent);
    let operator_anchor_hash = hash_entry(&EntryTypes::StringAnchor(operator_anchor))?;
    create_link(
        operator_anchor_hash,
        action_hash.clone(),
        LinkTypes::OperatorToDoorway,
        (),
    )?;

    // Create region link if specified
    if let Some(ref region) = input.region {
        let region_anchor = StringAnchor::new("doorway_region", region);
        let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;
        create_link(
            region_anchor_hash,
            action_hash.clone(),
            LinkTypes::RegionToDoorway,
            (),
        )?;
    }

    Ok(DoorwayOutput {
        action_hash,
        doorway: doorway_to_wire(&doorway),
    })
}

/// Update an existing doorway registration.
///
/// Only the original operator can update the doorway.
#[hdk_extern]
pub fn update_doorway(input: RegisterDoorwayInput) -> ExternResult<DoorwayOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let existing = get_doorway_by_id(input.id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            input.id
        )))
    })?;

    if existing.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can update this registration".to_string()
        )));
    }

    let doorway = DoorwayRegistration {
        id: input.id.clone(),
        url: input.url,
        operator_agent: existing.doorway.operator_agent,
        operator_human: existing.doorway.operator_human,
        capabilities_json: input.capabilities_json,
        reach: input.reach,
        region: input.region,
        bandwidth_mbps: input.bandwidth_mbps,
        version: input.version,
        tier: existing.doorway.tier,
        registered_at: existing.doorway.registered_at,
        updated_at: timestamp,
    };

    let action_hash = update_entry(
        existing.action_hash,
        &EntryTypes::DoorwayRegistration(doorway.clone()),
    )?;

    Ok(DoorwayOutput {
        action_hash,
        doorway: doorway_to_wire(&doorway),
    })
}

/// Get a doorway by its ID
#[hdk_extern]
pub fn get_doorway_by_id(id: String) -> ExternResult<Option<DoorwayOutput>> {
    let id_anchor = StringAnchor::new("doorway_id", &id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor))?;

    let query = LinkQuery::try_new(id_anchor_hash, LinkTypes::IdToDoorway)?;
    let links = get_links(query, GetStrategy::default())?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(doorway) = record
                    .entry()
                    .to_app_option::<DoorwayRegistration>()
                    .ok()
                    .flatten()
                {
                    return Ok(Some(DoorwayOutput {
                        action_hash,
                        doorway: doorway_to_wire(&doorway),
                    }));
                }
            }
        }
    }

    Ok(None)
}

/// Get all doorways registered by an operator
#[hdk_extern]
pub fn get_doorways_by_operator(operator_agent: String) -> ExternResult<Vec<DoorwayOutput>> {
    let operator_anchor = StringAnchor::new("doorway_operator", &operator_agent);
    let operator_anchor_hash = hash_entry(&EntryTypes::StringAnchor(operator_anchor))?;

    let query = LinkQuery::try_new(operator_anchor_hash, LinkTypes::OperatorToDoorway)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(doorway) = record
                    .entry()
                    .to_app_option::<DoorwayRegistration>()
                    .ok()
                    .flatten()
                {
                    results.push(DoorwayOutput {
                        action_hash,
                        doorway: doorway_to_wire(&doorway),
                    });
                }
            }
        }
    }

    Ok(results)
}

/// Get all doorways in a region
#[hdk_extern]
pub fn get_doorways_by_region(region: String) -> ExternResult<Vec<DoorwayOutput>> {
    let region_anchor = StringAnchor::new("doorway_region", &region);
    let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;

    let query = LinkQuery::try_new(region_anchor_hash, LinkTypes::RegionToDoorway)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut results = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(doorway) = record
                    .entry()
                    .to_app_option::<DoorwayRegistration>()
                    .ok()
                    .flatten()
                {
                    results.push(DoorwayOutput {
                        action_hash,
                        doorway: doorway_to_wire(&doorway),
                    });
                }
            }
        }
    }

    Ok(results)
}

// =============================================================================
// Heartbeat Functions
// =============================================================================

/// Record a doorway heartbeat.
///
/// Only the doorway's operator can record heartbeats for it.
#[hdk_extern]
pub fn record_heartbeat(input: RecordHeartbeatInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let doorway = get_doorway_by_id(input.doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            input.doorway_id
        )))
    })?;

    if doorway.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can record heartbeats".to_string()
        )));
    }

    let heartbeat = DoorwayHeartbeat {
        doorway_id: input.doorway_id,
        status: input.status,
        uptime_ratio: input.uptime_ratio,
        active_connections: input.active_connections,
        content_served: input.content_served,
        timestamp,
    };

    let action_hash = create_entry(&EntryTypes::DoorwayHeartbeat(heartbeat))?;

    // Link from doorway to heartbeat
    create_link(
        doorway.action_hash,
        action_hash.clone(),
        LinkTypes::DoorwayToHeartbeat,
        (),
    )?;

    Ok(action_hash)
}

/// Record a daily heartbeat summary.
///
/// Called at midnight UTC to summarize the previous day's heartbeats.
/// Only the doorway's operator can record summaries.
#[hdk_extern]
pub fn record_daily_summary(input: RecordSummaryInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;

    let doorway = get_doorway_by_id(input.doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            input.doorway_id
        )))
    })?;

    if doorway.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can record summaries".to_string()
        )));
    }

    let summary = DoorwayHeartbeatSummary {
        doorway_id: input.doorway_id,
        date: input.date.clone(),
        uptime_ratio: input.uptime_ratio,
        total_content_served: input.total_content_served,
        peak_connections: input.peak_connections,
        heartbeat_count: input.heartbeat_count,
    };

    let action_hash = create_entry(&EntryTypes::DoorwayHeartbeatSummary(summary))?;

    // Link from doorway to summary
    create_link(
        doorway.action_hash,
        action_hash.clone(),
        LinkTypes::DoorwayToSummary,
        (),
    )?;

    // Link by date for cross-doorway queries
    let date_anchor = StringAnchor::new("summary_date", &input.date);
    let date_anchor_hash = hash_entry(&EntryTypes::StringAnchor(date_anchor))?;
    create_link(
        date_anchor_hash,
        action_hash.clone(),
        LinkTypes::SummaryByDate,
        (),
    )?;

    Ok(action_hash)
}

/// Get recent heartbeats for a doorway
#[hdk_extern]
pub fn get_doorway_heartbeats(doorway_id: String) -> ExternResult<Vec<DoorwayHeartbeat>> {
    let doorway = get_doorway_by_id(doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            doorway_id
        )))
    })?;

    let query = LinkQuery::try_new(doorway.action_hash, LinkTypes::DoorwayToHeartbeat)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut heartbeats = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(heartbeat) = record
                    .entry()
                    .to_app_option::<DoorwayHeartbeat>()
                    .ok()
                    .flatten()
                {
                    heartbeats.push(heartbeat);
                }
            }
        }
    }

    Ok(heartbeats)
}

/// Get daily summaries for a doorway
#[hdk_extern]
pub fn get_doorway_summaries(doorway_id: String) -> ExternResult<Vec<DoorwayHeartbeatSummary>> {
    let doorway = get_doorway_by_id(doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            doorway_id
        )))
    })?;

    let query = LinkQuery::try_new(doorway.action_hash, LinkTypes::DoorwayToSummary)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut summaries = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(summary) = record
                    .entry()
                    .to_app_option::<DoorwayHeartbeatSummary>()
                    .ok()
                    .flatten()
                {
                    summaries.push(summary);
                }
            }
        }
    }

    Ok(summaries)
}

// =============================================================================
// Health Attestation Functions (Peer Observation)
// =============================================================================

/// Record a health attestation (peer observation of another doorway).
///
/// Only a registered doorway operator can attest about another doorway.
/// The attestation is linked from the SUBJECT doorway for discovery.
#[hdk_extern]
pub fn record_health_attestation(input: RecordHealthAttestationInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = now.as_micros();

    // Verify attestor is a registered doorway operator
    let attestor_doorway =
        get_doorway_by_id(input.attestor_doorway_id.clone())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Attestor doorway '{}' not found",
                input.attestor_doorway_id
            )))
        })?;

    if attestor_doorway.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can record attestations".to_string()
        )));
    }

    // Verify subject doorway exists
    let subject_doorway =
        get_doorway_by_id(input.subject_doorway_id.clone())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Subject doorway '{}' not found",
                input.subject_doorway_id
            )))
        })?;

    let attestation = HealthAttestation {
        attestor_doorway_id: input.attestor_doorway_id,
        operator_agent: agent_info.agent_initial_pubkey.to_string(),
        subject_doorway_id: input.subject_doorway_id,
        observed_status: input.observed_status,
        response_time_ms: input.response_time_ms,
        conductor_healthy: input.conductor_healthy,
        timestamp,
    };

    let action_hash = create_entry(&EntryTypes::HealthAttestation(attestation))?;

    // Link from SUBJECT doorway to attestation (for discovery: "who observed this doorway?")
    create_link(
        subject_doorway.action_hash,
        action_hash.clone(),
        LinkTypes::DoorwayToAttestation,
        (),
    )?;

    Ok(action_hash)
}

/// Get all health attestations for a doorway (observations by peers).
#[hdk_extern]
pub fn get_doorway_attestations(doorway_id: String) -> ExternResult<Vec<HealthAttestationOutput>> {
    let doorway = get_doorway_by_id(doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            doorway_id
        )))
    })?;

    let query = LinkQuery::try_new(doorway.action_hash, LinkTypes::DoorwayToAttestation)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut attestations = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(attestation) = record
                    .entry()
                    .to_app_option::<HealthAttestation>()
                    .ok()
                    .flatten()
                {
                    let entry_hash = record
                        .action()
                        .entry_hash()
                        .map(|h| h.to_string())
                        .unwrap_or_default();
                    let author = record.action().author().to_string();

                    attestations.push(HealthAttestationOutput {
                        action_hash: action_hash.to_string(),
                        entry_hash,
                        attestation: attestation_to_wire(&attestation),
                        author,
                    });
                }
            }
        }
    }

    Ok(attestations)
}

// =============================================================================
// Tier Computation
// =============================================================================

/// Update a doorway's tier based on its history.
///
/// Tier computation:
/// - Emerging: < 7 days old
/// - Established: 7+ days, 95%+ uptime
/// - Trusted: 30+ days, 99%+ uptime
/// - Anchor: 90+ days, 99.9%+ uptime
#[hdk_extern]
pub fn update_doorway_tier(doorway_id: String) -> ExternResult<DoorwayOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let timestamp = format!("{:?}", now);

    let existing = get_doorway_by_id(doorway_id.clone())?.ok_or_else(|| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "Doorway '{}' not found",
            doorway_id
        )))
    })?;

    if existing.doorway.operator_agent != agent_info.agent_initial_pubkey.to_string() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "Only the doorway operator can update tier".to_string()
        )));
    }

    let summaries = get_doorway_summaries(doorway_id)?;
    let days_active = summaries.len();

    let avg_uptime = if summaries.is_empty() {
        0.0
    } else {
        summaries.iter().map(|s| s.uptime_ratio).sum::<f32>() / summaries.len() as f32
    };

    let new_tier = if days_active >= 90 && avg_uptime >= 0.999 {
        "Anchor"
    } else if days_active >= 30 && avg_uptime >= 0.99 {
        "Trusted"
    } else if days_active >= 7 && avg_uptime >= 0.95 {
        "Established"
    } else {
        "Emerging"
    };

    let doorway = DoorwayRegistration {
        id: existing.doorway.id,
        url: existing.doorway.url,
        operator_agent: existing.doorway.operator_agent,
        operator_human: existing.doorway.operator_human,
        capabilities_json: existing.doorway.capabilities_json,
        reach: existing.doorway.reach,
        region: existing.doorway.region,
        bandwidth_mbps: existing.doorway.bandwidth_mbps,
        version: existing.doorway.version,
        tier: new_tier.to_string(),
        registered_at: existing.doorway.registered_at,
        updated_at: timestamp,
    };

    let action_hash = update_entry(
        existing.action_hash,
        &EntryTypes::DoorwayRegistration(doorway.clone()),
    )?;

    Ok(DoorwayOutput {
        action_hash,
        doorway: doorway_to_wire(&doorway),
    })
}

// =============================================================================
// ContentServer Functions (P2P Content Publishing)
// =============================================================================

/// Register as a content server for a specific content hash.
///
/// Creates a ContentServer entry and links for discovery by doorways.
/// Any agent can register to serve content they have stored.
#[hdk_extern]
pub fn register_content_server(
    input: RegisterContentServerInput,
) -> ExternResult<ContentServerOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let now_secs = now.as_seconds_and_nanos().0 as u64;

    // Build endpoints list
    let endpoints: Vec<StorageEndpoint> = match input.endpoints {
        Some(eps) if !eps.is_empty() => eps
            .into_iter()
            .map(|e| StorageEndpoint {
                url: e.url,
                protocol: e.protocol,
                priority: e.priority.unwrap_or(50),
            })
            .collect(),
        _ => {
            // Backwards compatibility: create endpoint from serve_url if provided
            if let Some(ref url) = input.serve_url {
                let protocol = if url.starts_with("https://") {
                    "https".to_string()
                } else {
                    "http".to_string()
                };
                vec![StorageEndpoint {
                    url: url.clone(),
                    protocol,
                    priority: 50,
                }]
            } else {
                Vec::new()
            }
        }
    };

    let server = ContentServer {
        content_hash: input.content_hash.clone(),
        capability: input.capability.clone(),
        serve_url: input.serve_url,
        endpoints,
        online: true,
        priority: input.priority.unwrap_or(50),
        region: input.region.clone(),
        bandwidth_mbps: input.bandwidth_mbps,
        registered_at: now_secs,
        last_heartbeat: now_secs,
    };

    let action_hash = create_entry(&EntryTypes::ContentServer(server.clone()))?;

    // Create content hash lookup link (primary discovery path)
    let hash_anchor = StringAnchor::new("content_hash", &input.content_hash);
    let hash_anchor_hash = hash_entry(&EntryTypes::StringAnchor(hash_anchor))?;
    create_link(
        hash_anchor_hash,
        action_hash.clone(),
        LinkTypes::HashToContentServer,
        (),
    )?;

    // Create agent lookup link (for finding all servers an agent operates)
    let agent_anchor = StringAnchor::new(
        "content_server_agent",
        &agent_info.agent_initial_pubkey.to_string(),
    );
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(
        agent_anchor_hash,
        action_hash.clone(),
        LinkTypes::AgentToContentServer,
        (),
    )?;

    // Create capability lookup link
    let cap_anchor = StringAnchor::new("content_server_capability", &input.capability);
    let cap_anchor_hash = hash_entry(&EntryTypes::StringAnchor(cap_anchor))?;
    create_link(
        cap_anchor_hash,
        action_hash.clone(),
        LinkTypes::CapabilityToContentServer,
        (),
    )?;

    // Create region lookup link if specified
    if let Some(ref region) = input.region {
        let region_anchor = StringAnchor::new("content_server_region", region);
        let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;
        create_link(
            region_anchor_hash,
            action_hash.clone(),
            LinkTypes::RegionToContentServer,
            (),
        )?;
    }

    Ok(ContentServerOutput {
        action_hash,
        server: content_server_to_wire(&server),
    })
}

/// Update content server heartbeat (marks as online and updates timestamp).
///
/// Call periodically to indicate this server is still alive and serving.
#[hdk_extern]
pub fn update_content_server_heartbeat(
    action_hash: ActionHash,
) -> ExternResult<ContentServerOutput> {
    let now = sys_time()?;
    let now_secs = now.as_seconds_and_nanos().0 as u64;

    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("ContentServer not found".to_string())))?;

    let mut server = record
        .entry()
        .to_app_option::<ContentServer>()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Deserialization error: {:?}",
                e
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid ContentServer entry".to_string()
            ))
        })?;

    server.last_heartbeat = now_secs;
    server.online = true;

    let new_action_hash = update_entry(action_hash, &EntryTypes::ContentServer(server.clone()))?;

    Ok(ContentServerOutput {
        action_hash: new_action_hash,
        server: content_server_to_wire(&server),
    })
}

/// Mark content server as offline.
///
/// Call when stopping content serving for this hash.
#[hdk_extern]
pub fn mark_content_server_offline(action_hash: ActionHash) -> ExternResult<ContentServerOutput> {
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("ContentServer not found".to_string())))?;

    let mut server = record
        .entry()
        .to_app_option::<ContentServer>()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Deserialization error: {:?}",
                e
            )))
        })?
        .ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(
                "Invalid ContentServer entry".to_string()
            ))
        })?;

    server.online = false;

    let new_action_hash = update_entry(action_hash, &EntryTypes::ContentServer(server.clone()))?;

    Ok(ContentServerOutput {
        action_hash: new_action_hash,
        server: content_server_to_wire(&server),
    })
}

/// Find publishers for a content hash.
///
/// This is the primary discovery function used by doorways to find
/// which agents can serve a particular piece of content.
#[hdk_extern]
pub fn find_publishers(input: FindPublishersInput) -> ExternResult<FindPublishersOutput> {
    let limit = input.limit.unwrap_or(10);
    let online_only = input.online_only.unwrap_or(true);

    let hash_anchor = StringAnchor::new("content_hash", &input.content_hash);
    let hash_anchor_hash = hash_entry(&EntryTypes::StringAnchor(hash_anchor))?;

    let query = LinkQuery::try_new(hash_anchor_hash, LinkTypes::HashToContentServer)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut publishers = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(server) = record
                    .entry()
                    .to_app_option::<ContentServer>()
                    .ok()
                    .flatten()
                {
                    // Apply filters
                    if online_only && !server.online {
                        continue;
                    }

                    if let Some(ref cap) = input.capability {
                        if &server.capability != cap {
                            continue;
                        }
                    }

                    publishers.push(ContentServerOutput {
                        action_hash,
                        server: content_server_to_wire(&server),
                    });
                }
            }
        }
    }

    // Sort by priority (higher first), then by region preference
    publishers.sort_by(|a, b| {
        // Prefer region match
        if let Some(ref preferred) = input.prefer_region {
            let a_matches = a.server.region.as_ref() == Some(preferred);
            let b_matches = b.server.region.as_ref() == Some(preferred);
            if a_matches != b_matches {
                return if a_matches {
                    std::cmp::Ordering::Less
                } else {
                    std::cmp::Ordering::Greater
                };
            }
        }
        // Then by priority (higher is better)
        b.server.priority.cmp(&a.server.priority)
    });

    // Apply limit
    publishers.truncate(limit);

    Ok(FindPublishersOutput {
        content_hash: input.content_hash,
        publishers,
    })
}

/// Get all content servers operated by an agent.
#[hdk_extern]
pub fn get_content_servers_by_agent(
    agent_pubkey: String,
) -> ExternResult<Vec<ContentServerOutput>> {
    let agent_anchor = StringAnchor::new("content_server_agent", &agent_pubkey);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

    let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToContentServer)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut servers = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(server) = record
                    .entry()
                    .to_app_option::<ContentServer>()
                    .ok()
                    .flatten()
                {
                    servers.push(ContentServerOutput {
                        action_hash,
                        server: content_server_to_wire(&server),
                    });
                }
            }
        }
    }

    Ok(servers)
}

/// Get all content servers with a specific capability.
#[hdk_extern]
pub fn get_content_servers_by_capability(
    capability: String,
) -> ExternResult<Vec<ContentServerOutput>> {
    let cap_anchor = StringAnchor::new("content_server_capability", &capability);
    let cap_anchor_hash = hash_entry(&EntryTypes::StringAnchor(cap_anchor))?;

    let query = LinkQuery::try_new(cap_anchor_hash, LinkTypes::CapabilityToContentServer)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut servers = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(server) = record
                    .entry()
                    .to_app_option::<ContentServer>()
                    .ok()
                    .flatten()
                {
                    servers.push(ContentServerOutput {
                        action_hash,
                        server: content_server_to_wire(&server),
                    });
                }
            }
        }
    }

    Ok(servers)
}

// =============================================================================
// Init
// =============================================================================

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
