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
// ContentServer Input/Output Types
// =============================================================================

/// Input for registering a content server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterContentServerInput {
    /// Content hash this server can provide (e.g., "sha256-abc123")
    pub content_hash: String,
    /// Capability: blob, html5_app, media_stream, learning_package, custom
    pub capability: String,
    /// URL where this server accepts content requests
    pub serve_url: Option<String>,
    /// Server priority (0-100, higher = preferred)
    pub priority: Option<u8>,
    /// Geographic region for latency-based routing
    pub region: Option<String>,
    /// Bandwidth capacity in Mbps
    pub bandwidth_mbps: Option<u32>,
}

/// Output from content server operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentServerOutput {
    pub action_hash: ActionHash,
    pub server: ContentServer,
}

/// Input for finding content publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersInput {
    /// Content hash to find publishers for
    pub content_hash: String,
    /// Optional: filter by capability
    pub capability: Option<String>,
    /// Optional: prefer publishers in this region
    pub prefer_region: Option<String>,
    /// Maximum number of publishers to return (default: 10)
    pub limit: Option<usize>,
    /// Only return online publishers (default: true)
    pub online_only: Option<bool>,
}

/// Output from finding publishers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindPublishersOutput {
    /// Content hash queried
    pub content_hash: String,
    /// Found publishers, sorted by priority
    pub publishers: Vec<ContentServerOutput>,
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
        } else if let Some(server) = record.entry().to_app_option::<ContentServer>().ok().flatten() {
            emit_signal(InfrastructureSignal::ContentServerCommitted {
                action_hash,
                entry_hash,
                server,
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
// ContentServer Functions (P2P Content Publishing)
// =============================================================================

/// Register as a content server for a specific content hash.
///
/// Creates a ContentServer entry and links for discovery by doorways.
/// Any agent can register to serve content they have stored.
#[hdk_extern]
pub fn register_content_server(input: RegisterContentServerInput) -> ExternResult<ContentServerOutput> {
    let agent_info = agent_info()?;
    let now = sys_time()?;
    let now_secs = now.as_seconds_and_nanos().0 as u64;

    let server = ContentServer {
        content_hash: input.content_hash.clone(),
        capability: input.capability.clone(),
        serve_url: input.serve_url,
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
    create_link(hash_anchor_hash, action_hash.clone(), LinkTypes::HashToContentServer, ())?;

    // Create agent lookup link (for finding all servers an agent operates)
    let agent_anchor = StringAnchor::new("content_server_agent", &agent_info.agent_initial_pubkey.to_string());
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;
    create_link(agent_anchor_hash, action_hash.clone(), LinkTypes::AgentToContentServer, ())?;

    // Create capability lookup link
    let cap_anchor = StringAnchor::new("content_server_capability", &input.capability);
    let cap_anchor_hash = hash_entry(&EntryTypes::StringAnchor(cap_anchor))?;
    create_link(cap_anchor_hash, action_hash.clone(), LinkTypes::CapabilityToContentServer, ())?;

    // Create region lookup link if specified
    if let Some(ref region) = input.region {
        let region_anchor = StringAnchor::new("content_server_region", region);
        let region_anchor_hash = hash_entry(&EntryTypes::StringAnchor(region_anchor))?;
        create_link(region_anchor_hash, action_hash.clone(), LinkTypes::RegionToContentServer, ())?;
    }

    Ok(ContentServerOutput {
        action_hash,
        server,
    })
}

/// Update content server heartbeat (marks as online and updates timestamp).
///
/// Call periodically to indicate this server is still alive and serving.
#[hdk_extern]
pub fn update_content_server_heartbeat(action_hash: ActionHash) -> ExternResult<ContentServerOutput> {
    let now = sys_time()?;
    let now_secs = now.as_seconds_and_nanos().0 as u64;

    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "ContentServer not found".to_string()
        )))?;

    let mut server = record.entry().to_app_option::<ContentServer>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialization error: {:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid ContentServer entry".to_string())))?;

    server.last_heartbeat = now_secs;
    server.online = true;

    let new_action_hash = update_entry(action_hash, &EntryTypes::ContentServer(server.clone()))?;

    Ok(ContentServerOutput {
        action_hash: new_action_hash,
        server,
    })
}

/// Mark content server as offline.
///
/// Call when stopping content serving for this hash.
#[hdk_extern]
pub fn mark_content_server_offline(action_hash: ActionHash) -> ExternResult<ContentServerOutput> {
    let record = get(action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            "ContentServer not found".to_string()
        )))?;

    let mut server = record.entry().to_app_option::<ContentServer>()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Deserialization error: {:?}", e))))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid ContentServer entry".to_string())))?;

    server.online = false;

    let new_action_hash = update_entry(action_hash, &EntryTypes::ContentServer(server.clone()))?;

    Ok(ContentServerOutput {
        action_hash: new_action_hash,
        server,
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
                if let Some(server) = record.entry().to_app_option::<ContentServer>().ok().flatten() {
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
                        server,
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
                return if a_matches { std::cmp::Ordering::Less } else { std::cmp::Ordering::Greater };
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
pub fn get_content_servers_by_agent(agent_pubkey: String) -> ExternResult<Vec<ContentServerOutput>> {
    let agent_anchor = StringAnchor::new("content_server_agent", &agent_pubkey);
    let agent_anchor_hash = hash_entry(&EntryTypes::StringAnchor(agent_anchor))?;

    let query = LinkQuery::try_new(agent_anchor_hash, LinkTypes::AgentToContentServer)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut servers = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(server) = record.entry().to_app_option::<ContentServer>().ok().flatten() {
                    servers.push(ContentServerOutput {
                        action_hash,
                        server,
                    });
                }
            }
        }
    }

    Ok(servers)
}

/// Get all content servers with a specific capability.
#[hdk_extern]
pub fn get_content_servers_by_capability(capability: String) -> ExternResult<Vec<ContentServerOutput>> {
    let cap_anchor = StringAnchor::new("content_server_capability", &capability);
    let cap_anchor_hash = hash_entry(&EntryTypes::StringAnchor(cap_anchor))?;

    let query = LinkQuery::try_new(cap_anchor_hash, LinkTypes::CapabilityToContentServer)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut servers = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(server) = record.entry().to_app_option::<ContentServer>().ok().flatten() {
                    servers.push(ContentServerOutput {
                        action_hash,
                        server,
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
