//! Recovery Protocol Phase 2 — Graduated Authority Identity Recovery
//!
//! Entry types and validation for graduated-authority-based identity recovery.
//! See: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
//!
//! New entry types (prefixed with the module to distinguish from legacy
//! attestation-vote-based recovery in lib.rs):
//! - RecoveryQuorumRequest: claimant's request, authored by hosting doorway (M1-cleanup: merging into RecoveryRequest)
//! - KeyRotation: authoritative claim "new agent X is Matthew now"

use hdi::prelude::*;

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

// Entry type structs and validation functions will be added in subsequent tasks.

// =============================================================================
// RecoveryQuorumRequest
// =============================================================================

/// A request to rotate an agent key via seed-quorum recovery.
/// Authored by the hosting doorway (the recovering human has no working cell).
/// No authority is implied by authorship; authority comes from KeyRotation's
/// quorum_signature verifying under the commitment's seed_public_half.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoveryQuorumRequest {
    pub human_agent_pubkey: AgentPubKey,
    pub seed_commitment_hash: ActionHash,
    pub new_agent_pubkey: AgentPubKey,
    pub hosting_doorway_pubkey: AgentPubKey,
    // recovery_mode removed — RecoveryMode deleted in M1-cleanup (superseded by RecoveryAuthority)
    pub request_nonce: Vec<u8>,      // 16 bytes random
    pub created_at: Timestamp,
}

pub fn validate_recovery_quorum_request(
    request: &RecoveryQuorumRequest,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: request_nonce must be 16 bytes
    if request.request_nonce.len() != 16 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryQuorumRequest request_nonce must be exactly 16 bytes".to_string(),
        ));
    }
    // Rule 2: seed_commitment_hash must be non-empty
    // Further validation (existence on DHT, non-superseded) happens in
    // must_get_valid_record during KeyRotation validation — the request itself
    // is just a claim of intent, cheap to commit.
    // Rule 3: human_agent_pubkey must differ from new_agent_pubkey
    if request.human_agent_pubkey == request.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoveryQuorumRequest new_agent_pubkey must differ from human_agent_pubkey"
                .to_string(),
        ));
    }
    // Rule 4: Stewarded mode requires a grant_hash; validation of grant-ness
    // is deferred to Phase 2b (stewarded-specific validation branches in KeyRotation).
    // The enum variant carries the hash; no additional check here.
    Ok(ValidateCallbackResult::Valid)
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
    // NOTE: Task 6 replaces the legacy RecoveryRequest (string-ID struct) with the modernized
    // AgentPubKey-based struct. Until then, we resolve against RecoveryQuorumRequest which
    // has the same AgentPubKey fields. Task 6 will update this to RecoveryRequest.
    let request_record = must_get_valid_record(rotation.recovery_request_hash.clone())?;
    let request_entry: RecoveryQuorumRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "KeyRotation references non-RecoveryQuorumRequest entry: {e:?}"
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

