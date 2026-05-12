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

// DoorwayHeartbeat removed (observation-event-layer spec §10 Stage 6):
// functionality moved to infrastructure:doorway-heartbeat observations on
// Track 2 substrate (ObservationManagerBackend). Health attestations graduate
// from accumulated observations via GraduationEvaluator (Stages 5.3 + 5.5).

// DoorwayHeartbeatSummary removed (Stage C.3.a): consolidated into elohim DNA under
// attestation_kind "attestation:doorway-summary" via content_store::issue_attestation.

// HealthAttestation removed (Stage C.3.a): consolidated into elohim DNA under
// attestation_kind "attestation:device-health" via content_store::issue_attestation.

// =============================================================================
// Content Server Types (P2P Content Publishing)
// =============================================================================

/// Content serving capabilities - what type of content an agent can serve.
/// Used by doorways to route requests to appropriate publishers.
pub const CONTENT_SERVER_CAPABILITIES: [&str; 5] = [
    "blob",             // Raw blob serving (GET /blob/{hash})
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
    ///   - "http://192.168.1.100:8080" → GET /blob/{hash}
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
    // DoorwayHeartbeat removed (observation-event-layer spec §10 Stage 6) — moved to Track 2 substrate
    // DoorwayHeartbeatSummary removed (Stage C.3.a) — see comment above
    // HealthAttestation removed (Stage C.3.a) — see comment above
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

    // DoorwayToHeartbeat removed (observation-event-layer spec §10 Stage 6) — DoorwayHeartbeat entry type removed

    // DoorwayToAttestation removed (Stage C.3.a) — HealthAttestation entry type removed
    // DoorwayToSummary removed (Stage C.3.a) — DoorwayHeartbeatSummary entry type removed
    // SummaryByDate removed (Stage C.3.a) — DoorwayHeartbeatSummary entry type removed

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
                    // DoorwayHeartbeat removed (observation-event-layer spec §10 Stage 6)
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

// validate_doorway_heartbeat removed (observation-event-layer spec §10 Stage 6) — DoorwayHeartbeat entry type removed

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
