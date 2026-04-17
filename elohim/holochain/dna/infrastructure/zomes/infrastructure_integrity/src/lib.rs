//! Infrastructure Integrity Zome
//!
//! Defines entry and link types for network infrastructure:
//! - Doorway registration and federation
//! - Heartbeat monitoring
//! - Trust tier computation
//!
//! This DNA is foundational for the Elohim network's self-validating infrastructure.
//! Doorways are Web2 bridges to Holochain - they serve content but don't own it.

use hdi::prelude::*;

pub mod peer_status;
pub use peer_status::{PeerCapabilityFlags, PeerLifecycleState, PeerStatus};

// =============================================================================
// Doorway Status Constants
// =============================================================================

/// Doorway operational status
pub const DOORWAY_STATUSES: [&str; 4] = [
    "online",      // Fully operational
    "degraded",    // Reduced capacity
    "maintenance", // Planned downtime
    "offline",     // Not responding
];

// =============================================================================
// Doorway Types
// =============================================================================

/// Trust tier for doorways - computed from uptime history and attestations.
/// Displayed on login screens so users know who they're signing in through.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DoorwayTier {
    Emerging,    // New doorway, < 7 days uptime
    Established, // 7+ days, 95%+ uptime
    Trusted,     // 30+ days, 99%+ uptime, peer attestations
    Anchor,      // 90+ days, 99.9%+ uptime, significant content served
}

impl Default for DoorwayTier {
    fn default() -> Self {
        DoorwayTier::Emerging
    }
}

impl std::fmt::Display for DoorwayTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorwayTier::Emerging => write!(f, "Emerging"),
            DoorwayTier::Established => write!(f, "Established"),
            DoorwayTier::Trusted => write!(f, "Trusted"),
            DoorwayTier::Anchor => write!(f, "Anchor"),
        }
    }
}

/// DoorwayCapabilities - what services this doorway provides
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DoorwayCapabilities {
    pub bootstrap: bool,    // Can bootstrap new nodes
    pub signal: bool,       // Provides WebRTC signaling
    pub gateway: bool,      // HTTP/REST gateway
    pub projection: bool,   // Maintains MongoDB projections
    pub custodian: bool,    // Can store blobs
}

impl Default for DoorwayCapabilities {
    fn default() -> Self {
        Self {
            bootstrap: false,
            signal: false,
            gateway: true,  // Most doorways are at least gateways
            projection: true,
            custodian: false,
        }
    }
}

/// DoorwayRegistration - A doorway node registered in the DHT.
///
/// Doorways are the Web2 bridge to Holochain infrastructure. Unlike traditional
/// fediverse instances, doorways don't own user data - they project it from the DHT.
/// Users can switch doorways freely; their identity and data remain in the DHT.
///
/// Self-registration only: operator_agent must be the author (validation rule).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayRegistration {
    pub id: String,                       // "alpha-elohim-host"
    pub url: String,                      // "https://alpha.elohim.host"
    pub operator_agent: String,           // uhCAk... (who runs this doorway)
    pub operator_human: Option<String>,   // Reference to Human entry in imagodei DNA
    pub capabilities_json: String,        // DoorwayCapabilities as JSON
    pub reach: String,                    // What reach levels served
    pub region: Option<String>,           // Geographic locality for routing
    pub bandwidth_mbps: Option<u32>,      // Self-reported bandwidth capacity
    pub version: String,                  // Doorway software version
    pub tier: String,                     // DoorwayTier as string
    pub registered_at: String,
    pub updated_at: String,
}

/// DoorwayHeartbeat - Lightweight status update (60s interval).
///
/// Detailed heartbeats are kept for 24 hours only, then summarized.
/// This enables real-time health monitoring without unbounded storage.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayHeartbeat {
    pub doorway_id: String,               // References DoorwayRegistration.id
    pub status: String,                   // online, degraded, maintenance, offline
    pub uptime_ratio: f32,                // Rolling uptime since last summary
    pub active_connections: u32,          // Current active connections
    pub content_served: u64,              // Bytes served since last heartbeat
    pub timestamp: String,
}

/// DoorwayHeartbeatSummary - Daily aggregate kept forever for reputation.
///
/// At midnight UTC, doorways summarize their previous day's heartbeats
/// and prune the detailed records. Summaries build the long-term trust history.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct DoorwayHeartbeatSummary {
    pub doorway_id: String,               // References DoorwayRegistration.id
    pub date: String,                     // "2024-01-15" (UTC date)
    pub uptime_ratio: f32,                // Average uptime for the day
    pub total_content_served: u64,        // Total bytes served
    pub peak_connections: u32,            // Maximum concurrent connections
    pub heartbeat_count: u32,             // How many heartbeats received (expect ~1440)
}

/// A peer doorway's observation of another doorway's health.
/// Published to DHT so all nodes see cross-validated health.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthAttestation {
    /// Doorway ID of the observer (must match author's DoorwayRegistration)
    pub attestor_doorway_id: String,
    /// Agent pubkey of the attestor (must match entry author)
    pub operator_agent: String,
    /// Doorway ID of the subject being observed
    pub subject_doorway_id: String,
    /// Observed status: "online", "degraded", "unreachable"
    pub observed_status: String,
    /// Response time in milliseconds (None if unreachable)
    pub response_time_ms: Option<u32>,
    /// Whether the subject's conductor pool was healthy (from /health response)
    pub conductor_healthy: Option<bool>,
    /// When the probe happened
    pub timestamp: i64,
}

const ATTESTATION_STATUSES: &[&str] = &["online", "degraded", "unreachable"];

// =============================================================================
// Content Server Types (P2P Content Publishing)
// =============================================================================

/// Content serving capabilities - what type of content an agent can serve.
/// Used by doorways to route requests to appropriate publishers.
pub const CONTENT_SERVER_CAPABILITIES: [&str; 5] = [
    "blob",             // Raw blob serving (GET /store/{hash})
    "html5_app",        // Zip extraction + file serving (GET /apps/{id}/{path})
    "media_stream",     // Range request support for video/audio
    "learning_package", // SCORM/xAPI packages
    "custom",           // Custom capability
];

/// Storage endpoint - a single reachable URL for content serving.
/// A ContentServer can have multiple endpoints (different protocols, redundancy).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageEndpoint {
    /// Base URL for fetching content. The content hash is appended to form the full URL.
    /// Examples:
    ///   - "http://192.168.1.100:8080/store" → GET /store/{hash}
    ///   - "https://my-node.example.com/api/v1/blob" → GET /api/v1/blob/{hash}
    pub url: String,
    /// Protocol type: "http", "https", "libp2p"
    pub protocol: String,
    /// Priority within this server (0-100, higher = preferred)
    pub priority: u8,
}

/// ContentServer - Registers an agent as content publisher in the DHT.
///
/// When an agent stores content (e.g., HTML5 app zip), they create a ContentServer
/// entry to announce their ability to serve it. Doorways discover publishers by
/// querying these entries and route requests to the nearest available publisher.
///
/// This enables true P2P content delivery: any agent can publish, any doorway
/// can serve, and users get content from the best available source.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ContentServer {
    /// Content hash this server can provide (e.g., "sha256-abc123")
    /// Use "*" for wildcard registration (can serve any content of this capability)
    pub content_hash: String,

    /// What type of content serving this server supports (from CONTENT_SERVER_CAPABILITIES)
    pub capability: String,

    /// URL where this server accepts content requests (DEPRECATED - use endpoints)
    /// Kept for backwards compatibility with existing entries
    pub serve_url: Option<String>,

    /// Multiple reachable endpoints for content fetching (NEW)
    /// Enables redundancy and protocol flexibility (HTTP, HTTPS, libp2p)
    pub endpoints: Vec<StorageEndpoint>,

    /// Whether this server is currently online and serving
    pub online: bool,

    /// Server priority (0-100, higher = preferred)
    pub priority: u8,

    /// Geographic region for latency-based routing
    pub region: Option<String>,

    /// Bandwidth capacity in Mbps (self-reported)
    pub bandwidth_mbps: Option<u32>,

    /// Unix timestamp when this registration was created
    pub registered_at: u64,

    /// Unix timestamp of last heartbeat (updated periodically)
    pub last_heartbeat: u64,
}

impl ContentServer {
    /// Check if this server is stale (no heartbeat for given seconds)
    pub fn is_stale(&self, max_age_secs: u64, now: u64) -> bool {
        now.saturating_sub(self.last_heartbeat) > max_age_secs
    }
}

// =============================================================================
// Anchor Entry (for link indexing)
// =============================================================================

/// Generic string anchor for creating deterministic link bases
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    pub anchor_type: String,
    pub anchor_value: String,
}

impl StringAnchor {
    pub fn new(anchor_type: &str, anchor_value: &str) -> Self {
        Self {
            anchor_type: anchor_type.to_string(),
            anchor_value: anchor_value.to_string(),
        }
    }
}

// =============================================================================
// Entry Types Enum
// =============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    DoorwayRegistration(DoorwayRegistration),
    DoorwayHeartbeat(DoorwayHeartbeat),
    DoorwayHeartbeatSummary(DoorwayHeartbeatSummary),
    HealthAttestation(HealthAttestation),
    ContentServer(ContentServer),
    StringAnchor(StringAnchor),
    PeerStatus(PeerStatus),
}

// =============================================================================
// Link Types
// =============================================================================

#[hdk_link_types]
pub enum LinkTypes {
    // DoorwayRegistration links
    IdToDoorway,                // Anchor(doorway_id) -> DoorwayRegistration
    OperatorToDoorway,          // Anchor(operator_agent) -> DoorwayRegistration
    RegionToDoorway,            // Anchor(region) -> DoorwayRegistration
    ReachToDoorway,             // Anchor(reach) -> DoorwayRegistration
    TierToDoorway,              // Anchor(tier) -> DoorwayRegistration

    // DoorwayHeartbeat links (recent, pruned daily)
    DoorwayToHeartbeat,         // DoorwayRegistration -> DoorwayHeartbeat

    // HealthAttestation links
    DoorwayToAttestation,       // DoorwayRegistration (subject) -> HealthAttestation (peer observations about this doorway)

    // DoorwayHeartbeatSummary links (kept forever)
    DoorwayToSummary,           // DoorwayRegistration -> DoorwayHeartbeatSummary
    SummaryByDate,              // Anchor(date) -> DoorwayHeartbeatSummary

    // ContentServer links (P2P content publishing)
    HashToContentServer,        // Anchor(content_hash) -> ContentServer
    AgentToContentServer,       // Anchor(agent_pubkey) -> ContentServer
    CapabilityToContentServer,  // Anchor(capability) -> ContentServer
    RegionToContentServer,      // Anchor(region) -> ContentServer (for geo-routing)

    // PeerStatus links
    AgentToPeerStatus,          // base: AgentPubKey (as EntryHash), target: PeerStatus action hash
}

// =============================================================================
// Validation
// =============================================================================

#[hdk_extern]
pub fn genesis_self_check(_data: GenesisSelfCheckData) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                match app_entry {
                    EntryTypes::DoorwayRegistration(doorway) => {
                        validate_doorway_registration(&doorway, &action)
                    }
                    EntryTypes::DoorwayHeartbeat(heartbeat) => {
                        validate_doorway_heartbeat(&heartbeat)
                    }
                    EntryTypes::DoorwayHeartbeatSummary(summary) => {
                        validate_doorway_summary(&summary)
                    }
                    EntryTypes::HealthAttestation(attestation) => {
                        validate_health_attestation(&attestation, &action)
                    }
                    EntryTypes::ContentServer(server) => {
                        validate_content_server(&server)
                    }
                    EntryTypes::StringAnchor(_) => Ok(ValidateCallbackResult::Valid),
                    EntryTypes::PeerStatus(ps) => validate_peer_status(&ps, &action),
                }
            }
            OpEntry::UpdateEntry { app_entry, action, .. } => {
                match app_entry {
                    EntryTypes::DoorwayRegistration(doorway) => {
                        validate_doorway_update(&doorway, &action)
                    }
                    _ => Ok(ValidateCallbackResult::Valid),
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Validate DoorwayRegistration
fn validate_doorway_registration(
    doorway: &DoorwayRegistration,
    action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    // Self-registration only: operator_agent must be the author
    let author_str = action.author.to_string();
    if doorway.operator_agent != author_str {
        return Ok(ValidateCallbackResult::Invalid(
            "Doorway operator_agent must match the author (self-registration only)".to_string(),
        ));
    }

    // Validate URL format (basic check)
    if !doorway.url.starts_with("http://") && !doorway.url.starts_with("https://") {
        return Ok(ValidateCallbackResult::Invalid(
            "Doorway URL must start with http:// or https://".to_string(),
        ));
    }

    // Validate ID is not empty
    if doorway.id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "Doorway ID cannot be empty".to_string(),
        ));
    }

    // Validate tier is valid
    let valid_tiers = ["Emerging", "Established", "Trusted", "Anchor"];
    if !valid_tiers.contains(&doorway.tier.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid tier '{}'. Must be one of: {:?}", doorway.tier, valid_tiers),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate DoorwayRegistration update
fn validate_doorway_update(
    doorway: &DoorwayRegistration,
    action: &Update,
) -> ExternResult<ValidateCallbackResult> {
    // Self-registration only: operator_agent must be the author
    let author_str = action.author.to_string();
    if doorway.operator_agent != author_str {
        return Ok(ValidateCallbackResult::Invalid(
            "Doorway operator_agent must match the author (self-registration only)".to_string(),
        ));
    }

    // Validate URL format (basic check)
    if !doorway.url.starts_with("http://") && !doorway.url.starts_with("https://") {
        return Ok(ValidateCallbackResult::Invalid(
            "Doorway URL must start with http:// or https://".to_string(),
        ));
    }

    // Validate tier is valid
    let valid_tiers = ["Emerging", "Established", "Trusted", "Anchor"];
    if !valid_tiers.contains(&doorway.tier.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid tier '{}'. Must be one of: {:?}", doorway.tier, valid_tiers),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate DoorwayHeartbeat
fn validate_doorway_heartbeat(heartbeat: &DoorwayHeartbeat) -> ExternResult<ValidateCallbackResult> {
    // Validate status is valid
    if !DOORWAY_STATUSES.contains(&heartbeat.status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!("Invalid status '{}'. Must be one of: {:?}", heartbeat.status, DOORWAY_STATUSES),
        ));
    }

    // Validate uptime_ratio is in valid range
    if heartbeat.uptime_ratio < 0.0 || heartbeat.uptime_ratio > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "uptime_ratio must be between 0.0 and 1.0".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate DoorwayHeartbeatSummary
fn validate_doorway_summary(summary: &DoorwayHeartbeatSummary) -> ExternResult<ValidateCallbackResult> {
    // Validate uptime_ratio is in valid range
    if summary.uptime_ratio < 0.0 || summary.uptime_ratio > 1.0 {
        return Ok(ValidateCallbackResult::Invalid(
            "uptime_ratio must be between 0.0 and 1.0".to_string(),
        ));
    }

    // Validate date format (basic check for YYYY-MM-DD)
    if summary.date.len() != 10 || summary.date.chars().nth(4) != Some('-') {
        return Ok(ValidateCallbackResult::Invalid(
            "date must be in YYYY-MM-DD format".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate HealthAttestation
fn validate_health_attestation(
    attestation: &HealthAttestation,
    action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    if attestation.attestor_doorway_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "attestor_doorway_id cannot be empty".to_string(),
        ));
    }

    if attestation.subject_doorway_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "subject_doorway_id cannot be empty".to_string(),
        ));
    }

    if attestation.attestor_doorway_id == attestation.subject_doorway_id {
        return Ok(ValidateCallbackResult::Invalid(
            "Cannot attest about yourself".to_string(),
        ));
    }

    if attestation.operator_agent != action.author.to_string() {
        return Ok(ValidateCallbackResult::Invalid(
            "operator_agent must match the entry author".to_string(),
        ));
    }

    if !ATTESTATION_STATUSES.contains(&attestation.observed_status.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Invalid observed_status '{}'. Must be one of: {:?}",
                attestation.observed_status, ATTESTATION_STATUSES
            ),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate PeerStatus
///
/// Rules:
/// - Author must equal `peer_id` (peers cannot author PeerStatus for others).
/// - Timestamp must be within ±5 minutes of DHT validation time.
fn validate_peer_status(
    ps: &PeerStatus,
    action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: author must equal peer_id (self-authored only)
    if ps.peer_id != action.author {
        return Ok(ValidateCallbackResult::Invalid(
            "PeerStatus.peer_id must match entry author".to_string(),
        ));
    }

    // Rule 2: timestamp within ±5 minutes of the action timestamp.
    // We compare against action.timestamp (signed by author, non-repudiable)
    // rather than sys_time(): integrity validation must be deterministic, so
    // every validator arrives at the same verdict. See spec §PeerStatus.
    let now_us = action.timestamp.as_micros();
    let ts_us = ps.timestamp.as_micros();
    let delta = (now_us - ts_us).abs();
    if delta > 5 * 60 * 1_000_000 {
        return Ok(ValidateCallbackResult::Invalid(
            "PeerStatus.timestamp outside ±5m window".to_string(),
        ));
    }

    Ok(ValidateCallbackResult::Valid)
}

/// Validate ContentServer registration
fn validate_content_server(server: &ContentServer) -> ExternResult<ValidateCallbackResult> {
    // Validate content_hash is not empty
    if server.content_hash.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "content_hash cannot be empty".to_string(),
        ));
    }

    // Validate capability is valid
    if !CONTENT_SERVER_CAPABILITIES.contains(&server.capability.as_str()) {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "Invalid capability '{}'. Must be one of: {:?}",
                server.capability, CONTENT_SERVER_CAPABILITIES
            ),
        ));
    }

    // Validate priority is in valid range (0-100)
    if server.priority > 100 {
        return Ok(ValidateCallbackResult::Invalid(
            "priority must be between 0 and 100".to_string(),
        ));
    }

    // Validate serve_url format if provided (deprecated but still accepted)
    if let Some(url) = &server.serve_url {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ValidateCallbackResult::Invalid(
                "serve_url must start with http:// or https://".to_string(),
            ));
        }
    }

    // Validate endpoints
    for endpoint in &server.endpoints {
        // Validate URL format
        let valid_protocols = ["http", "https", "libp2p"];
        if !valid_protocols.contains(&endpoint.protocol.as_str()) {
            return Ok(ValidateCallbackResult::Invalid(
                format!(
                    "Invalid endpoint protocol '{}'. Must be one of: {:?}",
                    endpoint.protocol, valid_protocols
                ),
            ));
        }

        // Validate URL matches protocol
        match endpoint.protocol.as_str() {
            "http" => {
                if !endpoint.url.starts_with("http://") {
                    return Ok(ValidateCallbackResult::Invalid(
                        format!("Endpoint URL '{}' must start with http:// for http protocol", endpoint.url),
                    ));
                }
            }
            "https" => {
                if !endpoint.url.starts_with("https://") {
                    return Ok(ValidateCallbackResult::Invalid(
                        format!("Endpoint URL '{}' must start with https:// for https protocol", endpoint.url),
                    ));
                }
            }
            "libp2p" => {
                // libp2p multiaddrs typically start with /ip4/, /ip6/, or /dns4/
                // We'll be lenient here since libp2p addresses have many formats
            }
            _ => {}
        }

        // Validate priority
        if endpoint.priority > 100 {
            return Ok(ValidateCallbackResult::Invalid(
                format!("Endpoint priority must be between 0 and 100, got {}", endpoint.priority),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
