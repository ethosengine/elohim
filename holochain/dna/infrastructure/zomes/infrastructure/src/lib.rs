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
// Input/Output Types
// =============================================================================

/// Input for registering a new doorway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterDoorwayInput {
    pub id: String,
    pub url: String,
    pub capabilities_json: String,
    pub reach: String,
    pub region: Option<String>,
    pub bandwidth_mbps: Option<u32>,
    pub version: String,
}

/// Output from doorway registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoorwayOutput {
    pub action_hash: ActionHash,
    pub doorway: DoorwayRegistration,
}

/// Input for recording a heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordHeartbeatInput {
    pub doorway_id: String,
    pub status: String,
    pub uptime_ratio: f32,
    pub active_connections: u32,
    pub content_served: u64,
}

/// Input for recording a daily summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordSummaryInput {
    pub doorway_id: String,
    pub date: String,
    pub uptime_ratio: f32,
    pub total_content_served: u64,
    pub peak_connections: u32,
    pub heartbeat_count: u32,
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

        if let Some(doorway) = record.entry().to_app_option::<DoorwayRegistration>().ok().flatten() {
            emit_signal(InfrastructureSignal::DoorwayCommitted {
                action_hash,
                entry_hash,
                doorway,
                author,
            })?;
        } else if let Some(heartbeat) = record.entry().to_app_option::<DoorwayHeartbeat>().ok().flatten() {
            emit_signal(InfrastructureSignal::DoorwayHeartbeatCommitted {
                action_hash,
                entry_hash,
                heartbeat,
                author,
            })?;
        } else if let Some(summary) = record.entry().to_app_option::<DoorwayHeartbeatSummary>().ok().flatten() {
            emit_signal(InfrastructureSignal::DoorwaySummaryCommitted {
                action_hash,
                entry_hash,
                summary,
                author,
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
        return Err(wasm_error!(WasmErrorInner::Guest(
            format!("Doorway with ID '{}' already exists", input.id)
        )));
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
    create_link(id_anchor_hash, action_hash.clone(), LinkTypes::IdToDoorway, ())?;

    // Create operator lookup link
    let operator_anchor = StringAnchor::new("doorway_operator", &doorway.operator_agent);
    let operator_anchor_hash = hash_entry(&EntryTypes::StringAnchor(operator_anchor))?;
    create_link(operator_anchor_hash, action_hash.clone(), LinkTypes::OperatorToDoorway, ())?;

    // Create region link if specified
    if let Some(ref region) = input.region {
        let region_anchor = StringAnchor::new("doorway_region", region);
        let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;
        create_link(region_anchor_hash, action_hash.clone(), LinkTypes::RegionToDoorway, ())?;
    }

    Ok(DoorwayOutput {
        action_hash,
        doorway,
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

    let existing = get_doorway_by_id(input.id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", input.id)
        )))?;

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

    let action_hash = update_entry(existing.action_hash, &EntryTypes::DoorwayRegistration(doorway.clone()))?;

    Ok(DoorwayOutput {
        action_hash,
        doorway,
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
                if let Some(doorway) = record.entry().to_app_option::<DoorwayRegistration>().ok().flatten() {
                    return Ok(Some(DoorwayOutput {
                        action_hash,
                        doorway,
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
                if let Some(doorway) = record.entry().to_app_option::<DoorwayRegistration>().ok().flatten() {
                    results.push(DoorwayOutput {
                        action_hash,
                        doorway,
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
                if let Some(doorway) = record.entry().to_app_option::<DoorwayRegistration>().ok().flatten() {
                    results.push(DoorwayOutput {
                        action_hash,
                        doorway,
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

    let doorway = get_doorway_by_id(input.doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", input.doorway_id)
        )))?;

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
    create_link(doorway.action_hash, action_hash.clone(), LinkTypes::DoorwayToHeartbeat, ())?;

    Ok(action_hash)
}

/// Record a daily heartbeat summary.
///
/// Called at midnight UTC to summarize the previous day's heartbeats.
/// Only the doorway's operator can record summaries.
#[hdk_extern]
pub fn record_daily_summary(input: RecordSummaryInput) -> ExternResult<ActionHash> {
    let agent_info = agent_info()?;

    let doorway = get_doorway_by_id(input.doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", input.doorway_id)
        )))?;

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
    create_link(doorway.action_hash, action_hash.clone(), LinkTypes::DoorwayToSummary, ())?;

    // Link by date for cross-doorway queries
    let date_anchor = StringAnchor::new("summary_date", &input.date);
    let date_anchor_hash = hash_entry(&EntryTypes::StringAnchor(date_anchor))?;
    create_link(date_anchor_hash, action_hash.clone(), LinkTypes::SummaryByDate, ())?;

    Ok(action_hash)
}

/// Get recent heartbeats for a doorway
#[hdk_extern]
pub fn get_doorway_heartbeats(doorway_id: String) -> ExternResult<Vec<DoorwayHeartbeat>> {
    let doorway = get_doorway_by_id(doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", doorway_id)
        )))?;

    let query = LinkQuery::try_new(doorway.action_hash, LinkTypes::DoorwayToHeartbeat)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut heartbeats = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(heartbeat) = record.entry().to_app_option::<DoorwayHeartbeat>().ok().flatten() {
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
    let doorway = get_doorway_by_id(doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", doorway_id)
        )))?;

    let query = LinkQuery::try_new(doorway.action_hash, LinkTypes::DoorwayToSummary)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut summaries = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                if let Some(summary) = record.entry().to_app_option::<DoorwayHeartbeatSummary>().ok().flatten() {
                    summaries.push(summary);
                }
            }
        }
    }

    Ok(summaries)
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

    let existing = get_doorway_by_id(doorway_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Doorway '{}' not found", doorway_id)
        )))?;

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
        tier: new_tier.to_string(),
        updated_at: timestamp,
        ..existing.doorway
    };

    let action_hash = update_entry(existing.action_hash, &EntryTypes::DoorwayRegistration(doorway.clone()))?;

    Ok(DoorwayOutput {
        action_hash,
        doorway,
    })
}

// =============================================================================
// Init
// =============================================================================

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    Ok(InitCallbackResult::Pass)
}
