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

#[cfg(not(feature = "lineage-witness"))] #[hdk_entry_types]
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

#[cfg(not(feature = "lineage-witness"))] #[hdk_link_types]
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

#[cfg(not(feature = "lineage-witness"))] #[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::CreateEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // TODO: Add link validation (e.g., verify link targets are valid entry types)
        FlatOp::Link(OpLink::CreateLink { .. }) => Ok(ValidateCallbackResult::Valid),
        FlatOp::Link(OpLink::DeleteLink { .. }) => Ok(ValidateCallbackResult::Valid),
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

// =============================================================================
// Lineage / Notarization carrying (Holochain Evolution Epic §2)
//
// Spec: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md
//
// A `NotarizationWitness` carries a proof that was witnessed in a PREDECESSOR
// DNA (v1) into this DNA (v2). The proof is the v1 `Action` plus the v1
// author's `Signature` over it. Because an action's signature preimage is the
// msgpack of the whole `Action` and contains NO DNA hash (except the genesis
// `Dna` action), v2's validators can re-verify a v1 notarization with no access
// to v1 at all.
//
// EVERYTHING in this section is gated behind the `lineage-witness` cargo
// feature (off by default): the default build must pack BYTE-IDENTICAL to
// pristine node-registry (no DNA-hash move) until the epic's own ceremony
// lands this on the fleet, not a CI roll. It lives at the END of the file,
// appended after the original content rather than interleaved with it, so
// the default build's compiled output — including any `file!()`/`line!()`
// location constants the toolchain bakes into panic/error paths for the
// SURVIVING code above — is not perturbed by this section's mere physical
// presence in the source text (cfg strips it from the AST, not from the
// file's line count). The `#[hdk_entry_types]` / `#[hdk_link_types]` macros
// also do not honor a `#[cfg]` on an individual variant (they consume the
// token stream before rustc's cfg-stripping reaches their generated code),
// so `EntryTypes` / `LinkTypes` / `validate` are each declared TWICE — once
// above (feature OFF, `#[cfg(not(feature = "lineage-witness"))]` combined
// onto their existing attribute line so it adds zero lines) and once here
// (feature ON) — rather than as one shared item with a cfg'd variant.
// =============================================================================

/// Maximum proofs carried by one witness (head-plane bundling budget, §3).
#[cfg(feature = "lineage-witness")]
pub const WITNESS_BATCH: usize = 16;

/// DNA properties this integrity zome reads via `dna_info()`.
///
/// `lineage` is the list of predecessor DNA hashes this DNA declares as its
/// parents. Because properties fold into the DNA hash, the DNA's own identity
/// commits to its lineage and every peer agrees on it without consulting
/// anything off-chain.
///
/// Unknown keys are ignored, so this coexists with the bootstrap-steward
/// `progenitor_pubkey` property in the same properties map.
#[cfg(feature = "lineage-witness")]
#[derive(Debug, Clone, Serialize, Deserialize, SerializedBytes)]
pub struct LineageProperties {
    #[serde(default)]
    pub lineage: Vec<DnaHash>,
    #[serde(default)]
    pub constitution_root: Option<String>,
}

/// One notarization carried forward from a predecessor DNA.
#[cfg(feature = "lineage-witness")]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CarriedProof {
    /// The predecessor action, embedded verbatim (0.7 `Action { header, data }`).
    pub action: Action,
    /// The predecessor author's signature over `action`.
    pub signature: Signature,
    /// The entry bytes — present ONLY for held-carry (§2.2), where the carrier
    /// is not the author and therefore cannot re-create the entry natively.
    pub entry: Option<Entry>,
}

/// A witnessed carriage: proofs from ONE predecessor DNA.
#[cfg(feature = "lineage-witness")]
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct NotarizationWitness {
    /// The predecessor DNA these proofs were witnessed in. MUST be declared in
    /// this DNA's `lineage` property.
    pub lineage_dna_hash: DnaHash,
    /// The carried proofs (<= `WITNESS_BATCH`).
    pub proofs: Vec<CarriedProof>,
}

// --- feature-ON EntryTypes: the pristine 6 variants PLUS the witness, ------
// appended LAST so existing entry-def indices are stable.
#[cfg(feature = "lineage-witness")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    NodeRegistration(NodeRegistration),
    NodeHeartbeat(NodeHeartbeat),
    HealthAttestation(HealthAttestation),
    CustodianAssignment(CustodianAssignment),
    StringAnchor(StringAnchor),
    ShardAssignment(ShardAssignment),
    NotarizationWitness(NotarizationWitness),
}

// --- feature-ON LinkTypes: the pristine 12 variants PLUS EntryToWitness. --
#[cfg(feature = "lineage-witness")]
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

    // Lineage / notarization carrying
    EntryToWitness,             // EntryHash(carried entry) -> NotarizationWitness
    AuthorToClose,              // Anchor(lineage:author) -> the SEAL witness (Station 8)
}

// --- feature-ON validate(): pristine rules PLUS the witness rules. --------
#[cfg(feature = "lineage-witness")]
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        // --- NotarizationWitness (Holochain Evolution Epic §2) --------------
        // Both the StoreEntry and StoreRecord ops are matched, as upstream's
        // MigrationRecord does, so the rule holds for every authority.
        //
        // `action` is BOUND (not `..`) because Station 8's after-close rule
        // needs the CARRIER's own identity and chain position: the close a
        // carrier has already witnessed lives on the carrier's own v2 chain,
        // and `header.prev_action` is the deterministic chain top the walk
        // starts from. Both arms carry a `TypedAction<CreateData>`, so one
        // binding serves both.
        FlatOp::CreateEntry(OpEntry::CreateEntry {
            app_entry: EntryTypes::NotarizationWitness(witness),
            action,
        })
        | FlatOp::CreateRecord(OpRecord::CreateEntry {
            app_entry: EntryTypes::NotarizationWitness(witness),
            action,
        }) => validate_notarization_witness(&witness, &action.header),

        // (4a) a witness can never be updated.
        FlatOp::CreateEntry(OpEntry::UpdateEntry {
            app_entry: EntryTypes::NotarizationWitness(_),
            ..
        })
        | FlatOp::CreateRecord(OpRecord::UpdateEntry {
            app_entry: EntryTypes::NotarizationWitness(_),
            ..
        })
        | FlatOp::Update(OpUpdate::Entry {
            app_entry: EntryTypes::NotarizationWitness(_),
            ..
        }) => Ok(ValidateCallbackResult::Invalid(
            "NotarizationWitness entries cannot be updated".to_string(),
        )),

        // (4b) a witness can never be deleted. The flattened delete ops carry
        // only the Delete action, so the deleted action is resolved with
        // `must_get_action` (HDI-legal) to learn its entry type.
        FlatOp::Delete(OpDelete { ref action }) => {
            refuse_witness_delete(&action.data.deletes_address)
        }
        FlatOp::CreateRecord(OpRecord::DeleteEntry { ref action }) => {
            refuse_witness_delete(&action.data.deletes_address)
        }

        // --- pre-existing rules --------------------------------------------
        FlatOp::CreateEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            OpEntry::UpdateEntry { app_entry, .. } => validate_create_entry(&app_entry),
            _ => Ok(ValidateCallbackResult::Valid),
        },
        // TODO: Add link validation (e.g., verify link targets are valid entry types)
        FlatOp::Link(OpLink::CreateLink { .. }) => Ok(ValidateCallbackResult::Valid),
        FlatOp::Link(OpLink::DeleteLink { .. }) => Ok(ValidateCallbackResult::Valid),
        _ => Ok(ValidateCallbackResult::Valid),
    }
}

/// Refuse a delete whose target is a `NotarizationWitness`.
#[cfg(feature = "lineage-witness")]
fn refuse_witness_delete(deletes_address: &ActionHash) -> ExternResult<ValidateCallbackResult> {
    let deleted = must_get_action(deletes_address.clone())?;
    if let Some(EntryType::App(app_entry_def)) = deleted.action().entry_type() {
        let witness: ScopedEntryDefIndex = (&UnitEntryTypes::NotarizationWitness).try_into()?;
        if app_entry_def.zome_index == witness.zome_index
            && app_entry_def.entry_index == witness.zome_type
        {
            return Ok(ValidateCallbackResult::Invalid(
                "NotarizationWitness entries cannot be deleted".to_string(),
            ));
        }
    }
    Ok(ValidateCallbackResult::Valid)
}

/// Validate a carried notarization, per spec §2.
///
/// 1. `lineage_dna_hash` is declared in this DNA's `lineage` property.
/// 2. every proof's signature verifies against `Action::signer()` — NOT the
///    author: a `CloseChain` toward `MigrationTarget::Agent` is signed by the
///    NEW key, which is exactly the semantics `holochain_keystore`'s
///    `action_signer()` uses when the conductor signs and verifies actions.
/// 3. when `entry` is carried (held-carry), it must hash to the entry hash the
///    carried action commits to.
/// 4. **after close** (Station 8, epic §3 (ii)): no proof may sit after the
///    predecessor chain's close — see [`refuse_carried_after_close`].
#[cfg(feature = "lineage-witness")]
fn validate_notarization_witness(
    witness: &NotarizationWitness,
    header: &ActionHeader,
) -> ExternResult<ValidateCallbackResult> {
    // (1) lineage — read from the DNA's own identity-bearing properties.
    let properties: LineageProperties =
        dna_info()?.modifiers.properties.try_into().map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "NotarizationWitness: could not deserialize DNA properties: {e:?}"
            )))
        })?;

    if !properties.lineage.contains(&witness.lineage_dna_hash) {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "NotarizationWitness lineage_dna_hash {} is not declared in this DNA's lineage property",
            witness.lineage_dna_hash
        )));
    }

    if witness.proofs.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "NotarizationWitness must carry at least one proof".to_string(),
        ));
    }

    if witness.proofs.len() > WITNESS_BATCH {
        return Ok(ValidateCallbackResult::Invalid(format!(
            "NotarizationWitness carries {} proofs, exceeding WITNESS_BATCH {}",
            witness.proofs.len(),
            WITNESS_BATCH
        )));
    }

    for (i, proof) in witness.proofs.iter().enumerate() {
        // (2) the predecessor notarization itself.
        if !verify_signature(
            proof.action.signer().clone(),
            proof.signature.clone(),
            &proof.action,
        )? {
            return Ok(ValidateCallbackResult::Invalid(format!(
                "NotarizationWitness proof {i}: carried signature does not verify against the \
                 action's signer {}",
                proof.action.signer()
            )));
        }

        // (3) held-carry: the carried entry must be the one the action commits to.
        if let Some(entry) = &proof.entry {
            let Some(expected) = proof.action.entry_hash() else {
                return Ok(ValidateCallbackResult::Invalid(format!(
                    "NotarizationWitness proof {i}: an entry was carried but the action \
                     references no entry hash"
                )));
            };
            let actual = hash_entry(entry.clone())?;
            if &actual != expected {
                return Ok(ValidateCallbackResult::Invalid(format!(
                    "NotarizationWitness proof {i}: carried entry hashes to {actual}, but the \
                     action commits to {expected}"
                )));
            }
        }
    }

    // (4) the sunset fence.
    refuse_carried_after_close(witness, header)
}

// =============================================================================
// STATION 8 — the after-close rule (the sunset's fence is OURS)
//
// Spec §3 "Design Constraints Discovered" (ii) and §4 step 5. MEASURED on 0.7
// (probes B and B2): `close_chain` is NOT a source-chain guard — the author's
// own conductor accepts a post-close create, and the REMOTE agent-activity
// authority refuses only the CloseChain's immediate successor (one rejection
// plus a warrant; the tail validates again and the bytes stay fetchable). So
// Holochain's contribution to the sunset is EVIDENCE, not a fence. The fence
// is (i) the storage controller disabling the v1 cell (Task 14b) and (ii) this
// rule: v2 refuses to carry a predecessor fact that sits AFTER that chain's
// close.
//
// -----------------------------------------------------------------------------
// IMPLEMENTATION DEVIATION, recorded (the same shape as the spec's note that
// rule (4)'s `entry: None` branch "is not expressible as written").
//
// The task brief specifies looking the close up through an `AuthorToClose`
// LINK. HDI 0.8 has no `get_links` — a validator's only deterministic
// dependencies are `must_get_entry` / `must_get_action` / `must_get_valid_record`
// (by exact hash) and `must_get_agent_activity` (an agent's chain in THIS DNA,
// by chain filter). So the link cannot be traversed here. `AuthorToClose` is
// still authored by `seal_close` — as the COORDINATOR-side read index the
// passport and the vehicle query — and the VALIDATOR reaches the same fact the
// HDI-legal way: it walks the carrier's OWN v2 chain, which is exactly "an
// earlier witness for that lineage" from the brief.
//
// What this buys and what it does not, stated plainly:
//   * a carrier that has sealed CANNOT then carry post-close facts — the close
//     is on its own chain and every authority re-derives the same verdict. That
//     is the sunset case (the peer seals, disables v1, and must not re-carry),
//     and it is non-evadable.
//   * a carrier that carries the close and the post-close facts in ONE witness
//     is refused by the intra-witness half of the same rule.
//   * a COURIER that never sealed and never carried the close cannot be
//     refused by any deterministic HDI rule, because "does a close exist for
//     author A?" is an unbounded lookup and HDI has no unbounded lookup. That
//     hole is named, not hidden: it is why the fence has leg (i) at all, and
//     why Probe B2's warrant is read by the storage plane as evidence of a
//     violated sunset.
//
// COST, declared: the walk makes the carrier's own chain a validation
// dependency for every witness it authors. For the node_registry rehearsal
// (tens of witnesses) that is small; a DNA carrying thousands of witnesses
// should bound the filter (`ChainFilter::until_hash` at its own OpenChain)
// before adopting this rule at that scale.
// =============================================================================

/// Fold the `CloseChain` proofs a witness carries into `closes` as
/// `(author, lowest close action_seq)`.
///
/// The LOWEST seq wins: two closes on one chain would mean the earlier one
/// already fenced everything after it, and taking the later would let facts
/// between them through.
#[cfg(feature = "lineage-witness")]
fn collect_closes(witness: &NotarizationWitness, closes: &mut Vec<(AgentPubKey, u32)>) {
    for proof in &witness.proofs {
        if !matches!(proof.action.data, ActionData::CloseChain(_)) {
            continue;
        }
        let author = proof.action.header.author.clone();
        let seq = proof.action.header.action_seq;
        match closes.iter_mut().find(|(a, _)| a == &author) {
            Some((_, known)) => {
                if seq < *known {
                    *known = seq;
                }
            }
            None => closes.push((author, seq)),
        }
    }
}

/// Refuse any carried proof that sits after its author's predecessor-chain
/// close, per the section header above.
///
/// A close is known from two places, both deterministic:
///   (a) THIS witness — a batch carrying the close cannot also carry facts
///       authored after it;
///   (b) an EARLIER witness on the carrier's own v2 chain that names the same
///       `lineage_dna_hash` — reached with `must_get_agent_activity` from the
///       carrier's `prev_action`, which is the chain top every authority
///       resolves identically.
///
/// **Absence of a close is not a rule.** A witness carried before any sunset
/// finds no close and is untouched — pre-sunset carriage (Stations 4, 5, 6) is
/// unaffected by construction.
#[cfg(feature = "lineage-witness")]
fn refuse_carried_after_close(
    witness: &NotarizationWitness,
    header: &ActionHeader,
) -> ExternResult<ValidateCallbackResult> {
    let mut closes: Vec<(AgentPubKey, u32)> = Vec::new();

    // (a) this witness's own batch.
    collect_closes(witness, &mut closes);

    // (b) the carrier's earlier witnesses for the SAME lineage.
    if let Some(prev_action) = header.prev_action.as_ref() {
        let witness_def: ScopedEntryDefIndex = (&UnitEntryTypes::NotarizationWitness).try_into()?;
        let activity = must_get_agent_activity(
            header.author.clone(),
            ChainFilter::new(prev_action.clone()),
        )?;
        for entry in activity {
            let action = entry.action.action().clone();
            let ActionData::Create(create) = &action.data else {
                continue;
            };
            let EntryType::App(app_entry_def) = &create.entry_type else {
                continue;
            };
            if app_entry_def.zome_index != witness_def.zome_index
                || app_entry_def.entry_index != witness_def.zome_type
            {
                continue;
            }
            let earlier =
                NotarizationWitness::try_from(must_get_entry(create.entry_hash.clone())?)?;
            if earlier.lineage_dna_hash != witness.lineage_dna_hash {
                continue;
            }
            collect_closes(&earlier, &mut closes);
        }
    }

    if closes.is_empty() {
        return Ok(ValidateCallbackResult::Valid);
    }

    for (i, proof) in witness.proofs.iter().enumerate() {
        // The close itself is the fence post, never fenced by itself.
        if matches!(proof.action.data, ActionData::CloseChain(_)) {
            continue;
        }
        let author = &proof.action.header.author;
        let seq = proof.action.header.action_seq;
        if let Some((_, close_seq)) = closes.iter().find(|(a, _)| a == author) {
            if seq > *close_seq {
                return Ok(ValidateCallbackResult::Invalid(format!(
                    "NotarizationWitness proof {i}: after close — {author}'s chain in lineage \
                     {} was closed at action_seq {close_seq}, but this proof sits at \
                     action_seq {seq}",
                    witness.lineage_dna_hash
                )));
            }
        }
    }

    Ok(ValidateCallbackResult::Valid)
}
