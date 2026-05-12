//! Infrastructure Coordinator Zome
//!
//! Provides functions for doorway registration, heartbeat monitoring,
//! and trust tier computation.
//!
//! Key functions:
//! - register_doorway: Self-registration (operator = author)
//! - record_daily_summary: Midnight aggregation (heartbeats moved to Track 2 substrate)
//! - update_doorway_tier: Recompute trust tier from history

use hdk::prelude::*;
use infrastructure_integrity::*;

pub mod peer_status;
pub use peer_status::*;

// =============================================================================
// Wire Types (re-exported from shared crate)
// =============================================================================

pub use infrastructure_types::{
    ContentServerOutput, DoorwayOutput, FindPublishersInput, FindPublishersOutput,
    HealthAttestationOutput, RecordHealthAttestationInput,
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
    // DoorwayHeartbeatCommitted removed (observation-event-layer spec §10 Stage 6) — DoorwayHeartbeat entry type removed
    /// DoorwayHeartbeatSummary was committed to the consolidated attestation store.
    /// action_hash and entry_hash are the elohim DNA's attestation record, not a local entry.
    DoorwaySummaryCommitted {
        doorway_id: String,
        date: String,
        uptime_ratio: f32,
        total_content_served: u64,
        peak_connections: u32,
        heartbeat_count: u32,
        /// CID of the consolidated attestation in elohim DNA (for provenance)
        consolidated_cid: String,
    },
    /// ContentServer was registered or updated
    ContentServerCommitted {
        action_hash: ActionHash,
        entry_hash: EntryHash,
        server: ContentServer,
        author: AgentPubKey,
    },
    /// HealthAttestation was committed to the consolidated attestation store.
    /// Fields are sourced from the bridge output, not a local DHT entry.
    HealthAttestationCommitted {
        attestor_doorway_id: String,
        subject_doorway_id: String,
        observed_status: String,
        response_time_ms: Option<u32>,
        conductor_healthy: Option<bool>,
        timestamp: i64,
        operator_agent: String,
        /// CID of the consolidated attestation in elohim DNA (for provenance)
        consolidated_cid: String,
    },
    /// PeerStatus was recorded (self-authored availability snapshot)
    ///
    /// Fields are flattened (rather than embedding the full `PeerStatus`
    /// struct) so the storage-side projection can deserialize without
    /// depending on the integrity crate. `peer_id` and `action_hash`
    /// serialize as base64 strings over the signal channel — the
    /// elohim-storage mirror variant should declare them as `String`.
    PeerStatusRecorded {
        peer_id: AgentPubKey,
        status: String,
        general_pool_member: bool,
        accepting_stewardship_reserves: bool,
        archetype_class: Option<String>,
        timestamp: i64,
        action_hash: ActionHash,
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
        // DoorwayHeartbeat signal branch removed (observation-event-layer spec §10 Stage 6)
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
        } else if let Some(ps) = record
            .entry()
            .to_app_option::<PeerStatus>()
            .ok()
            .flatten()
        {
            emit_signal(InfrastructureSignal::PeerStatusRecorded {
                peer_id: ps.peer_id.clone(),
                status: ps.status.to_string(),
                general_pool_member: ps.flags.general_pool_member,
                accepting_stewardship_reserves: ps.flags.accepting_stewardship_reserves,
                archetype_class: ps.archetype_class.clone(),
                timestamp: ps.timestamp.as_micros(),
                action_hash,
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
// Daily Summary Functions
// =============================================================================
// record_heartbeat removed (observation-event-layer spec §10 Stage 6):
// heartbeats now flow through infrastructure:doorway-heartbeat observations
// on Track 2 substrate (ObservationManagerBackend).

/// Record a daily heartbeat summary.
///
/// Called at midnight UTC to summarize the previous day's heartbeats.
/// Only the doorway's operator can record summaries.
///
/// Full-replacement bridge (Stage C): the local DoorwayHeartbeatSummary entry type
/// has been removed. Summaries are now consolidated into elohim DNA via
/// issue_attestation with attestation_kind "attestation:doorway-summary".
/// The signal is emitted directly from this function after the bridge call succeeds.
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

    // Full-replacement: bridge to elohim consolidated attestation store.
    let bridge_result = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:doorway-summary".to_string(),
        subject_cid: input.doorway_id.clone(),
        subject_kind: "doorway".to_string(),
        title: format!("Daily summary: {} for {}", input.doorway_id, input.date),
        description: Some(format!(
            "uptime={:.4} heartbeats={} peak_connections={}",
            input.uptime_ratio, input.heartbeat_count, input.peak_connections
        )),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "doorway_id": input.doorway_id,
            "date": input.date,
            "uptime_ratio": input.uptime_ratio,
            "total_content_served": input.total_content_served,
            "peak_connections": input.peak_connections,
            "heartbeat_count": input.heartbeat_count,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "operator-report".to_string(),
        proof_evidence: serde_json::json!({
            "class": "operator-report",
            "operator_agent": agent_info.agent_initial_pubkey.to_string(),
        }),
        expires_at: None,
    })?;

    // Emit signal directly (no local entry — signal fires from bridge result).
    emit_signal(InfrastructureSignal::DoorwaySummaryCommitted {
        doorway_id: input.doorway_id.clone(),
        date: input.date.clone(),
        uptime_ratio: input.uptime_ratio,
        total_content_served: input.total_content_served,
        peak_connections: input.peak_connections,
        heartbeat_count: input.heartbeat_count,
        consolidated_cid: bridge_result.cid.clone(),
    })?;

    // Return a stable handle: the bridge CID hashed as a pseudo-ActionHash substitute.
    // Callers receiving ActionHash use it as an opaque token; the real anchor is consolidated_cid.
    // We return the doorway's action_hash as a stable, verifiable handle for this doorway.
    Ok(doorway.action_hash)
}

// get_doorway_heartbeats removed (observation-event-layer spec §10 Stage 6):
// query heartbeat observations via ObservationManagerBackend instead.

/// Get daily summaries for a doorway.
///
/// Stage C bridge: DoorwayHeartbeatSummary local entry type has been removed.
/// Summaries are now consolidated into elohim DNA. This function returns an empty
/// vec — callers (including update_doorway_tier) degrade gracefully to "Emerging"
/// until Stage F wires a bridge query to elohim's get_attestations_for_subject.
#[hdk_extern]
pub fn get_doorway_summaries(_doorway_id: String) -> ExternResult<Vec<SummaryPlaceholder>> {
    // TODO(Stage-F): bridge to elohim::get_attestations_for_subject(doorway_id,
    // attestation_kind="attestation:doorway-summary") and translate results.
    Ok(Vec::new())
}

/// Placeholder shape for get_doorway_summaries pending Stage F bridge wiring.
/// Mirrors the fields callers need for tier computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryPlaceholder {
    pub doorway_id: String,
    pub date: String,
    pub uptime_ratio: f32,
    pub total_content_served: u64,
    pub peak_connections: u32,
    pub heartbeat_count: u32,
}

// =============================================================================
// Health Attestation Functions (Peer Observation)
// =============================================================================

/// Record a health attestation (peer observation of another doorway).
///
/// Only a registered doorway operator can attest about another doorway.
///
/// Full-replacement bridge (Stage C): the local HealthAttestation entry type has been
/// removed. Attestations are consolidated into elohim DNA via issue_attestation with
/// attestation_kind "attestation:device-health". Signal is emitted directly after the
/// bridge call succeeds.
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

    // Verify subject doorway exists (no local link needed — bridge handles DHT anchor)
    let _subject_doorway =
        get_doorway_by_id(input.subject_doorway_id.clone())?.ok_or_else(|| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "Subject doorway '{}' not found",
                input.subject_doorway_id
            )))
        })?;

    let operator_agent = agent_info.agent_initial_pubkey.to_string();

    // Full-replacement (Stage C): no local create_entry. Bridge to elohim consolidated store.
    let bridge_result = call_elohim_issue_attestation(ConsolidatedIssueAttestationInput {
        attestation_kind: "attestation:device-health".to_string(),
        subject_cid: input.subject_doorway_id.clone(),
        subject_kind: "device".to_string(),
        title: format!("Health attestation: {}", input.subject_doorway_id),
        description: Some(format!(
            "Observed status: {} - attestor: {}",
            input.observed_status, input.attestor_doorway_id
        )),
        reach: "community".to_string(),
        metadata: serde_json::json!({
            "attestor_doorway_id": input.attestor_doorway_id,
            "subject_doorway_id": input.subject_doorway_id,
            "observed_status": input.observed_status,
            "response_time_ms": input.response_time_ms,
            "conductor_healthy": input.conductor_healthy,
            "timestamp": timestamp,
        }),
        parent_governance_action_cid: None,
        vote_value: None,
        proof_class: "witness".to_string(),
        proof_evidence: serde_json::json!({
            "class": "witness",
            "operator_agent": operator_agent,
        }),
        expires_at: None,
    })?;

    // Emit signal directly (no local entry - signal fires from bridge result).
    emit_signal(InfrastructureSignal::HealthAttestationCommitted {
        attestor_doorway_id: input.attestor_doorway_id.clone(),
        subject_doorway_id: input.subject_doorway_id.clone(),
        observed_status: input.observed_status.clone(),
        response_time_ms: input.response_time_ms,
        conductor_healthy: input.conductor_healthy,
        timestamp,
        operator_agent,
        consolidated_cid: bridge_result.cid.clone(),
    })?;

    // Return attestor doorway's action_hash as stable opaque handle.
    // The real DHT anchor is bridge_result.cid in elohim DNA.
    Ok(attestor_doorway.action_hash)
}

/// Get all health attestations for a doorway (observations by peers).
///
/// Stage C bridge: HealthAttestation local entry type has been removed.
/// Returns empty vec — callers preserve their signature pending Stage F wiring to
/// elohim's get_attestations_for_subject(doorway_id, "attestation:device-health").
#[hdk_extern]
pub fn get_doorway_attestations(_doorway_id: String) -> ExternResult<Vec<HealthAttestationOutput>> {
    // TODO(Stage-F): bridge to elohim::get_attestations_for_subject(doorway_id,
    // attestation_kind="attestation:device-health") and translate to HealthAttestationOutput.
    Ok(Vec::new())
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

// =============================================================================
// Cross-DNA Bridges (Stage C — full-replacement)
//
// C.3.a: HealthAttestation and DoorwayHeartbeatSummary local entry types removed.
// Both record_health_attestation and record_daily_summary now bridge exclusively to
// elohim DNA's content_store::issue_attestation under:
//   - "attestation:device-health"  (health attestations)
//   - "attestation:doorway-summary" (daily summaries)
// Signals are emitted directly from coordinator functions after bridge success.
// =============================================================================

/// Input wire type — wire-compatible with elohim DNA's content_store::issue_attestation.
/// Defined locally because cross-DNA calls serialise through msgpack;
/// infrastructure cannot depend on elohim crates directly.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedIssueAttestationInput {
    pub attestation_kind: String,
    pub subject_cid: String,
    pub subject_kind: String,
    pub title: String,
    pub description: Option<String>,
    pub reach: String,
    pub metadata: serde_json::Value,
    pub parent_governance_action_cid: Option<String>,
    pub vote_value: Option<String>,
    pub proof_class: String,
    pub proof_evidence: serde_json::Value,
    pub expires_at: Option<String>,
}

/// Output wire type — wire-compatible with elohim DNA's content_store::issue_attestation.
#[derive(Debug, Serialize, Deserialize)]
struct ConsolidatedAttestationOutput {
    pub cid: String,
    pub attestation_kind: String,
    pub subject_cid: String,
    pub issuer_cid: String,
}

/// Bridge helper: call elohim content_store::issue_attestation and handle all
/// ZomeCallResponse arms uniformly. Mirrors the pattern from imagodei B.9 bridge.
fn call_elohim_issue_attestation(
    input: ConsolidatedIssueAttestationInput,
) -> ExternResult<ConsolidatedAttestationOutput> {
    let response = call(
        CallTargetCell::OtherRole("elohim".into()),
        ZomeName::from("content_store"),
        FunctionName::from("issue_attestation"),
        None,
        input,
    )?;
    match response {
        ZomeCallResponse::Ok(result) => result.decode().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "bridge decode (elohim::issue_attestation): {e}"
            )))
        }),
        ZomeCallResponse::Unauthorized(_, _, _, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Unauthorized bridge call to elohim::issue_attestation".to_string()
        ))),
        ZomeCallResponse::NetworkError(err) => Err(wasm_error!(WasmErrorInner::Guest(format!(
            "Network error bridging to elohim::issue_attestation: {err}"
        )))),
        ZomeCallResponse::CountersigningSession(err) => Err(wasm_error!(WasmErrorInner::Guest(
            format!("Countersigning error bridging to elohim: {err}")
        ))),
        ZomeCallResponse::AuthenticationFailed(_, _) => Err(wasm_error!(WasmErrorInner::Guest(
            "Authentication failed bridging to elohim::issue_attestation".to_string()
        ))),
    }
}
