//! Node Registry Integrity Zome
//!
//! Defines entry types for distributed node orchestration:
//! - NodeRegistration: Nodes publish capacity, location, capabilities
//! - NodeHeartbeat: Lightweight health updates every 30 seconds
//! - HealthAttestation: Peer-to-peer health verification
//! - CustodianAssignment: Orchestration decisions about who hosts what
//!
//! This enables:
//! - Plug-and-play node discovery
//! - Automatic disaster recovery
//! - Byzantine-fault-tolerant consensus
//! - Opt-in-by-default participation ("organ donation" model)

use hdi::prelude::*;
use serde::{Deserialize, Serialize};

// =============================================================================
// Constants
// =============================================================================

/// Node status values
pub const NODE_STATUS: [&str; 4] = [
    "online",      // Actively serving
    "maintenance", // Temporarily unavailable (planned)
    "degraded",    // Running but with reduced capacity
    "offline",     // Not responding to heartbeats
];

/// Shard strategies for content replication
pub const SHARD_STRATEGIES: [&str; 3] = [
    "full_replica",    // Complete copy on each custodian
    "threshold_split", // M-of-N Shamir's Secret Sharing
    "erasure_coded",   // Reed-Solomon erasure coding
];

/// Steward tier levels (from Shefa economic model)
pub const STEWARD_TIERS: [&str; 4] = [
    "caretaker", // Tier 1: Basic participation
    "guardian",  // Tier 2: Consistent contribution
    "steward",   // Tier 3: Significant commitment
    "pioneer",   // Tier 4: Network backbone
];

/// Maximum shard index for 4+3 Reed-Solomon (7 shards, indices 0-6)
pub const MAX_SHARD_INDEX: u32 = 6;

// =============================================================================
// Node Registration
// =============================================================================

/// Every node publishes this when it boots
/// Opt-in by default ("organ donation" model)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NodeRegistration {
    // === IDENTITY ===
    pub node_id: String,              // Unique hardware identifier (MAC, serial, etc.)
    pub agent_pub_key: String,        // Holochain agent public key
    pub display_name: String,         // Human-readable name (e.g., "Alice's Family Rack")

    // === CAPACITY ===
    pub cpu_cores: u32,               // Total CPU cores available
    pub memory_gb: u32,               // Total RAM in GB
    pub storage_tb: f64,              // Total storage in TB
    pub bandwidth_mbps: u32,          // Network bandwidth in Mbps

    // === LOCATION ===
    pub region: String,               // Geographic region (e.g., "us-west", "eu-central")
    pub latitude: Option<f64>,        // Optional precise location
    pub longitude: Option<f64>,       // Optional precise location

    // === CAPABILITIES ===
    pub zomes_hosted: Vec<String>,    // Which DNAs/zomes this node can run
    pub steward_tier: String,         // See STEWARD_TIERS

    // === PARTICIPATION (KEY FEATURE: OPT-IN BY DEFAULT) ===
    pub custodian_opt_in: bool,       // DEFAULT: true ("organ donation" model)
    pub max_custody_gb: Option<f64>,  // How much storage willing to contribute
    pub max_bandwidth_mbps: Option<u32>, // How much bandwidth willing to contribute
    pub max_cpu_percent: Option<f64>, // Max CPU utilization for custodianship

    // === HEALTH ===
    pub uptime_percent: f64,          // Rolling 30-day uptime (0.0-1.0)
    pub last_heartbeat: String,       // ISO 8601 timestamp

    // === METADATA ===
    pub registered_at: String,        // When node first registered
    pub updated_at: String,           // Last update to registration

    // === STEWARDSHIP CLAIM ===
    pub claim_status: String,         // "unclaimed", "claimed", "released"
    pub context_epr_id: Option<String>, // EPR reference to natural language context

    // === PROOF (PREVENTS SPOOFING) ===
    pub signature: String,            // Self-signed with agent key (hex-encoded)
}

// =============================================================================
// Node Heartbeat
// =============================================================================

/// Lightweight health update (every 30 seconds)
/// Minimal data to reduce DHT traffic
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NodeHeartbeat {
    pub node_id: String,              // Which node is reporting
    pub timestamp: String,            // ISO 8601 timestamp
    pub status: String,               // See NODE_STATUS
    pub current_load: f64,            // CPU load (0.0-1.0)
    pub active_connections: u32,      // Current WebSocket connections
    pub signature: String,            // Prevents spoofing
}

// =============================================================================
// Health Attestation
// =============================================================================

/// Peer-to-peer health verification
/// Nodes attest to each other's health (Byzantine fault tolerance)
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HealthAttestation {
    pub attester_node_id: String,     // Who is attesting
    pub subject_node_id: String,      // Who they're attesting about
    pub response_time_ms: u32,        // Measured latency
    pub success: bool,                // Did the health check succeed?
    pub timestamp: String,            // When attestation was made
    pub signature: String,            // Attester's signature
}

// =============================================================================
// Custodian Assignment
// =============================================================================

/// Orchestration decision: which node should custody which content
/// Can be decided by regional coordinator OR by quorum consensus
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct CustodianAssignment {
    pub assignment_id: String,        // Unique ID for this assignment

    // === CONTENT ===
    pub content_id: String,           // What content
    pub content_hash: String,         // SHA256 for integrity
    pub content_size_gb: Option<f64>, // Size of content for capacity planning

    // === CUSTODIAN ===
    pub custodian_node_id: String,    // Which node will custody
    pub strategy: String,             // See SHARD_STRATEGIES
    pub shard_index: Option<u32>,     // If using sharding, which shard
    pub preferred_region: Option<String>, // Preferred geographic region
    pub required_tier: Option<String>,    // Minimum steward tier required

    // === DECISION METADATA ===
    pub decided_by: String,           // Regional coordinator or "quorum"
    pub decision_round: Option<u32>,  // For Byzantine consensus
    pub votes_json: String,           // JSON: [(node_id, vote)] if quorum

    // === LIFECYCLE ===
    pub created_at: String,           // When assignment was made
    pub expires_at: String,           // Assignments have TTL, must renew
}

// =============================================================================
// String Anchor (for creating anchor points in DHT)
// =============================================================================

/// Generic string anchor for creating stable entry points in the DHT
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct StringAnchor {
    pub anchor_type: String,
    pub anchor_value: String,
}

// =============================================================================
// Recovery Protocol: Shard Assignment
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShardStatus {
    Active,
    Stale,
    Failed,
    Migrating,
    Reconstructing,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ShardingStrategy {
    Geographic,
    TrustTier,
    FamilyCluster,
    Manual,
}

/// Tracks which custodian holds which shard of a Reed-Solomon encoded blob
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct ShardAssignment {
    pub assignment_hash: Option<String>, // Self-reference for updates
    pub content_hash: String,
    pub custodian_did: String,
    pub shard_index: u32, // 0-6 for a 4+3 RS encoding
    pub strategy: ShardingStrategy,
    pub status: ShardStatus,
    pub verified_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// =============================================================================
// Entry Types Enum
// =============================================================================

#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    NodeRegistration(NodeRegistration),
    NodeHeartbeat(NodeHeartbeat),
    HealthAttestation(HealthAttestation),
    CustodianAssignment(CustodianAssignment),
    StringAnchor(StringAnchor),
    ShardAssignment(ShardAssignment),
}

// =============================================================================
// Link Types
// =============================================================================

#[hdk_link_types]
pub enum LinkTypes {
    // Node discovery
    RegionToNode,              // Anchor(region) -> NodeRegistration
    StatusToNode,              // Anchor(status) -> NodeRegistration
    TierToNode,                // Anchor(steward_tier) -> NodeRegistration
    IdToNodeRegistration,      // Anchor(node_id) -> NodeRegistration (for lookups by ID)
    CustodianToNode,           // Anchor(custodian="available") -> NodeRegistration (nodes opted-in)

    // Health tracking
    NodeToHeartbeat,           // NodeRegistration -> NodeHeartbeat (latest)
    NodeToAttestations,        // NodeRegistration -> HealthAttestation (all)

    // Custodian assignments
    ContentToAssignment,       // Anchor(content_id) -> CustodianAssignment
    NodeToAssignment,          // NodeRegistration -> CustodianAssignment (what node custodies)

    // Shard assignments
    ContentToShardAssignment,   // Anchor(content_hash) -> ShardAssignment
    CustodianToShardAssignment, // Anchor(custodian_did) -> ShardAssignment
    ShardIndexToAssignment,     // Anchor(content_hash:shard_index) -> ShardAssignment
}

// =============================================================================
// Validation Rules
// =============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // TODO: Add link validation (e.g., verify link targets are valid entry types)
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_create_entry(app_entry: &EntryTypes) -> ExternResult<ValidateCallbackResult> {
    match app_entry {
        EntryTypes::ShardAssignment(assignment) => validate_shard_assignment(assignment),
        // TODO: Add validation for NodeRegistration, NodeHeartbeat,
        // HealthAttestation, CustodianAssignment
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_shard_assignment(
    assignment: &ShardAssignment,
) -> ExternResult<ValidateCallbackResult> {
    if assignment.content_hash.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShardAssignment content_hash cannot be empty".to_string(),
        ));
    }

    if assignment.custodian_did.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShardAssignment custodian_did cannot be empty".to_string(),
        ));
    }

    if assignment.shard_index > MAX_SHARD_INDEX {
        return Ok(ValidateCallbackResult::Invalid(
            format!(
                "ShardAssignment shard_index {} exceeds maximum {} (4+3 RS encoding)",
                assignment.shard_index, MAX_SHARD_INDEX
            ),
        ));
    }

    if assignment.created_at.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShardAssignment created_at cannot be empty".to_string(),
        ));
    }

    if assignment.updated_at.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "ShardAssignment updated_at cannot be empty".to_string(),
        ));
    }

    if let Some(ref verified) = assignment.verified_at {
        if verified.is_empty() {
            return Ok(ValidateCallbackResult::Invalid(
                "ShardAssignment verified_at must not be empty when present".to_string(),
            ));
        }
    }

    if let Some(ref hash) = assignment.assignment_hash {
        if hash.is_empty() {
            return Ok(ValidateCallbackResult::Invalid(
                "ShardAssignment assignment_hash must not be empty when present".to_string(),
            ));
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
