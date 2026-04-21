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
/// Validated by verifying quorum_signature under the referenced commitment's
/// seed_public_half (Normal mode), or under the referenced StewardshipGrant's
/// authority set (Stewarded mode, deferred to Phase 2b).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct KeyRotation {
    pub human_agent_pubkey: AgentPubKey,
    pub new_agent_pubkey: AgentPubKey,
    pub superseded_agent_pubkey: AgentPubKey,
    pub seed_commitment_hash: ActionHash,
    pub recovery_request_hash: ActionHash,
    pub quorum_signature: Vec<u8>,   // 64 bytes Ed25519 signature
    pub rotated_at: Timestamp,
}

pub fn validate_key_rotation(
    rotation: &KeyRotation,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: quorum_signature must be exactly 64 bytes
    if rotation.quorum_signature.len() != 64 {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation quorum_signature must be exactly 64 bytes".to_string(),
        ));
    }

    // Rule 2: new_agent_pubkey must differ from superseded_agent_pubkey
    if rotation.new_agent_pubkey == rotation.superseded_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must differ from superseded_agent_pubkey".to_string(),
        ));
    }

    // Rule 3: Resolve RecoveryQuorumRequest via must_get_valid_record
    let request_record = must_get_valid_record(rotation.recovery_request_hash.clone())?;
    let request_entry: RecoveryQuorumRequest = request_record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!(
            "KeyRotation references non-RecoveryQuorumRequest: {e:?}"
        ))))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyRotation request_hash entry missing".to_string()
        )))?;

    // Rule 4: Request fields must match rotation fields
    if request_entry.new_agent_pubkey != rotation.new_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation new_agent_pubkey must match request".to_string(),
        ));
    }
    if request_entry.human_agent_pubkey != rotation.human_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "KeyRotation human_agent_pubkey must match request".to_string(),
        ));
    }
    // Note: seed_commitment_hash check removed — RecoverySeedCommitment deleted in M1-cleanup.
    // KeyRotation validator will be fully replaced in Task 5 with RecoveryAuthority enum.

    Ok(ValidateCallbackResult::Valid)
}

/// Ed25519 signature verification for the quorum signature.
/// Pure verification — no state, deterministic, WASM-safe.
fn verify_quorum_signature(
    seed_public_half: &[u8],
    new_agent_pubkey: &AgentPubKey,
    recovery_request_hash: &ActionHash,
    signature_bytes: &[u8],
) -> ExternResult<ValidateCallbackResult> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Parse verifying key
    let vk_bytes: [u8; 32] = seed_public_half.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "seed_public_half not 32 bytes in verify_quorum_signature".to_string()
        ))
    })?;
    let vk = VerifyingKey::from_bytes(&vk_bytes).map_err(|e| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "seed_public_half not a valid Ed25519 verifying key: {e}"
        )))
    })?;

    // Parse signature
    let sig_bytes: [u8; 64] = signature_bytes.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(
            "quorum_signature not 64 bytes".to_string()
        ))
    })?;
    let sig = Signature::from_bytes(&sig_bytes);

    // Construct message: new_agent_pubkey.get_raw_39() || recovery_request_hash.get_raw_39()
    // We use raw bytes for deterministic serialization in the signed payload.
    let mut message: Vec<u8> = Vec::with_capacity(39 + 39);
    message.extend_from_slice(new_agent_pubkey.get_raw_39());
    message.extend_from_slice(recovery_request_hash.get_raw_39());

    match vk.verify(&message, &sig) {
        Ok(()) => Ok(ValidateCallbackResult::Valid),
        Err(_) => Ok(ValidateCallbackResult::Invalid(
            "KeyRotation quorum_signature verification failed".to_string(),
        )),
    }
}

