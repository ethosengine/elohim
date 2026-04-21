//! Recovery Protocol Phase 2 — Graduated Authority Identity Recovery
//!
//! Entry types and validation for graduated-authority-based identity recovery.
//! See: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
//!
//! New entry types (prefixed with the module to distinguish from legacy
//! attestation-vote-based recovery in lib.rs):
//! - KeyRotation: authoritative claim "new agent X is Matthew now"

use hdi::prelude::*;

// =============================================================================
// Layer Constants & Helpers (Graduated Authority Ordering)
// =============================================================================

/// Layer name constants for RecoveryAuthority variants.
pub const LAYER_INTIMATE: &str = "intimate";
pub const LAYER_COMMUNITY: &str = "community";
pub const LAYER_GOVERNANCE: &str = "governance";
pub const LAYER_NETWORK: &str = "network";
pub const LAYER_CRYPTOGRAPHIC: &str = "cryptographic";

/// All valid layer names, ordered by ascending authority for the ordered layers.
/// `cryptographic` is orthogonal — it bypasses the freeze-floor ordering.
pub const RECOVERY_AUTHORITY_LAYERS: &[&str] = &[
    LAYER_INTIMATE,
    LAYER_COMMUNITY,
    LAYER_GOVERNANCE,
    LAYER_NETWORK,
    LAYER_CRYPTOGRAPHIC,
];

/// Map a RecoveryAuthority variant to its layer name.
pub fn authority_layer_name(authority: &RecoveryAuthority) -> &'static str {
    match authority {
        RecoveryAuthority::IntimateQuorum { .. } => LAYER_INTIMATE,
        RecoveryAuthority::CommunityConsensus { .. } => LAYER_COMMUNITY,
        RecoveryAuthority::GovernanceAct { .. } => LAYER_GOVERNANCE,
        RecoveryAuthority::NetworkWitness { .. } => LAYER_NETWORK,
        RecoveryAuthority::CryptographicQuorum { .. } => LAYER_CRYPTOGRAPHIC,
    }
}

/// Ordered layer rank for comparison. Returns None for `cryptographic` (orthogonal).
pub fn authority_layer_rank(layer: &str) -> Option<u8> {
    match layer {
        LAYER_INTIMATE => Some(1),
        LAYER_COMMUNITY => Some(2),
        LAYER_GOVERNANCE => Some(3),
        LAYER_NETWORK => Some(4),
        _ => None,
    }
}

// =============================================================================
// Public Enums
// =============================================================================

/// Purpose of a NetworkWitness authority — either restore access or retire the account.
/// Dissolution variant is reserved for cradle-to-grave care (deferred to constitutional-governance design).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum NetworkWitnessPurpose {
    /// Rescue: restore access to the human's active identity.
    Rescue,
    /// Dissolution: retire the account (deceased, irrecoverable).
    /// new_agent_pubkey is a memorial-marker null agent.
    /// Phase 2: stub-rejected in validator; shape reserved for constitutional-governance design.
    Dissolution,
}

/// Evidence supporting a KeyRotation. Five variants; any one sufficient for authorization.
/// Phase 2 implements IntimateQuorum + CryptographicQuorum; other variants stub-reject.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthority {
    /// Layer 1: Intimate-circle quorum via HumanityWitness entries from emergency contacts.
    /// Phase 2: IMPLEMENTED (structural shape; variant-specific validation lands in M2).
    IntimateQuorum {
        witness_hashes: Vec<ActionHash>,
    },
    /// Layer 2: Extended community via IdentityChallenge resolution.
    /// Phase 2: STUB-REJECTED (Phase 2b).
    CommunityConsensus {
        challenge_hash: ActionHash,
    },
    /// Layer 3: Governance act via qahal/stewardship resolution.
    /// Phase 2: STUB-REJECTED (cross-DNA qahal/mishpat work pending).
    GovernanceAct {
        grant_hash: ActionHash,
        resolution_hash: ActionHash,
    },
    /// Layer 4: Global elohim witness — prevents absolute lockout.
    /// Phase 2: STUB-REJECTED (pending elohim constitutional-governance design).
    NetworkWitness {
        witness_entries: Vec<ActionHash>,
        consensus_threshold_met_at: Timestamp,
        purpose: NetworkWitnessPurpose,
    },
    /// Layer 5 (orthogonal): Cryptographic M-of-N threshold via KeyStewardship.
    /// Provisioned only when elohim judges the human vulnerable enough.
    /// Phase 2: IMPLEMENTED (structural shape; variant-specific validation lands in M2).
    CryptographicQuorum {
        stewardship_hash: ActionHash,
        quorum_signature: Vec<u8>,
    },
}

/// Claimant's declared intent for which authority path a RecoveryRequest will pursue.
/// The actual KeyRotation authority can differ (escalation is allowed).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryAuthorityKind {
    IntimateQuorum,
    CommunityConsensus,
    GovernanceAct { grant_hash: ActionHash },
    NetworkWitness { purpose: NetworkWitnessPurpose },
    CryptographicQuorum { stewardship_hash: ActionHash },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum ConfidenceTier {
    None,
    Light,
    Deep,
    Constitutional,
}

// =============================================================================
// KeyRotation
// =============================================================================

/// The authoritative claim that a human's agent key has rotated.
/// Evidence is carried in the `authority` field as one of five graduated variants.
/// Phase 2 validator accepts the structural shape; variant-specific validation
/// (Ed25519 sig verification, HumanityWitness quorum counting) lands in M2.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyRotation {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub superseded_agent_pubkey: AgentPubKey,
    pub recovery_request_hash: ActionHash,
    pub authority: RecoveryAuthority,
    pub rotated_at: Timestamp,
}

pub fn validate_key_rotation(
    rotation: &KeyRotation,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: new_agent_pubkey must differ from superseded_agent_pubkey
    if rotation.new_agent_pubkey == rotation.superseded_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must differ from superseded_agent_pubkey".to_string(),
        ));
    }

    // Rule 2: Resolve the referenced RecoveryRequest and verify matching fields.
    let request_record = must_get_valid_record(rotation.recovery_request_hash.clone())?;
    let request_entry: super::RecoveryRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "KeyRotation references non-RecoveryRequest entry: {e:?}"
        ))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyRotation recovery_request_hash entry missing".to_string()
        )))?;

    if request_entry.human_agent_pubkey != rotation.human_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation human_agent_pubkey must match RecoveryRequest".to_string(),
        ));
    }
    if request_entry.new_agent_pubkey != rotation.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must match RecoveryRequest".to_string(),
        ));
    }

    // Rule 3: Phase 2 stub-rejects all variant-specific validation.
    // M2 milestone implements IntimateQuorum + CryptographicQuorum happy paths
    // and wires the floor-check against active IdentityFreeze entries.
    match &rotation.authority {
        RecoveryAuthority::IntimateQuorum { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::IntimateQuorum: variant validation pending in M2".to_string(),
        )),
        RecoveryAuthority::CommunityConsensus { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::CommunityConsensus: Phase 2b — IdentityChallenge resolution flow not yet implemented".to_string(),
        )),
        RecoveryAuthority::GovernanceAct { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::GovernanceAct: Phase 2b — cross-DNA qahal/mishpat resolution not yet implemented".to_string(),
        )),
        RecoveryAuthority::NetworkWitness { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::NetworkWitness: reserved for elohim constitutional-governance design".to_string(),
        )),
        RecoveryAuthority::CryptographicQuorum { .. } => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation::CryptographicQuorum: variant validation pending in M2".to_string(),
        )),
    }
}

