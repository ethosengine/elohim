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

    // === CUSTODIAN ===
    pub custodian_node_id: String,    // Which node will custody
    pub strategy: String,             // See SHARD_STRATEGIES
    pub shard_index: Option<u32>,     // If using sharding, which shard

    // === DECISION METADATA ===
    pub decided_by: String,           // Regional coordinator or "quorum"
    pub decision_round: u32,          // For Byzantine consensus
    pub votes_json: String,           // JSON: [(node_id, vote)] if quorum

    // === LIFECYCLE ===
    pub created_at: String,           // When assignment was made
    pub expires_at: String,           // Assignments have TTL, must renew
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

    // Health tracking
    NodeToHeartbeat,           // NodeRegistration -> NodeHeartbeat (latest)
    NodeToAttestations,        // NodeRegistration -> HealthAttestation (all)

    // Custodian assignments
    ContentToAssignment,       // Anchor(content_id) -> CustodianAssignment
    NodeToAssignment,          // NodeRegistration -> CustodianAssignment (what node custodies)
}

// =============================================================================
// Validation Rules
// =============================================================================

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op {
        Op::StoreEntry(store_entry) => {
            match store_entry.entry {
                Entry::App(app_entry) => {
                    // Parse entry type
                    let entry_type = EntryTypes::deserialize_from_type(
                        store_entry.action.hashed.entry_type().clone(),
                        &app_entry,
                    )?;

                    match entry_type {
                        Some(EntryTypes::NodeRegistration(node)) => {
                            validate_node_registration(&node)
                        }
                        Some(EntryTypes::NodeHeartbeat(heartbeat)) => {
                            validate_node_heartbeat(&heartbeat)
                        }
                        Some(EntryTypes::HealthAttestation(attestation)) => {
                            validate_health_attestation(&attestation)
                        }
                        Some(EntryTypes::CustodianAssignment(assignment)) => {
                            validate_custodian_assignment(&assignment)
                        }
                        None => Ok(ValidateCallbackResult::Invalid(
                            "Unknown entry type".to_string()
                        )),
                    }
                }
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

fn validate_node_registration(node: &NodeRegistration) -> ExternResult<ValidateCallbackResult> {
    // TODO: Verify signature matches agent_pub_key
    // TODO: Ensure reasonable capacity values (no overflow attacks)
    // TODO: Validate region is known region

    // For now, accept all
    Ok(ValidateCallbackResult::Valid)
}

fn validate_node_heartbeat(heartbeat: &NodeHeartbeat) -> ExternResult<ValidateCallbackResult> {
    // TODO: Verify signature
    // TODO: Ensure timestamp is recent (< 60 seconds old)
    // TODO: Validate status is one of NODE_STATUS

    Ok(ValidateCallbackResult::Valid)
}

fn validate_health_attestation(attestation: &HealthAttestation) -> ExternResult<ValidateCallbackResult> {
    // TODO: Verify attester signature
    // TODO: Ensure timestamp is recent
    // TODO: Prevent self-attestation (attester != subject)

    Ok(ValidateCallbackResult::Valid)
}

fn validate_custodian_assignment(assignment: &CustodianAssignment) -> ExternResult<ValidateCallbackResult> {
    // TODO: Verify decided_by has authority to make assignments
    // TODO: If quorum, verify votes_json has valid signatures
    // TODO: Ensure strategy is one of SHARD_STRATEGIES

    Ok(ValidateCallbackResult::Valid)
}
