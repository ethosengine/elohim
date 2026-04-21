//! Recovery Protocol Phase 2 — Socially Derived Identity Recovery
//!
//! Entry types and validation for seed-quorum-based identity recovery.
//! See: genesis/docs/superpowers/specs/2026-04-21-recovery-protocol-phase-2-design.md
//!
//! New entry types (prefixed with the module to distinguish from legacy
//! attestation-vote-based recovery in lib.rs):
//! - RecoverySeedCommitment: on-DHT public half + thresholds, no holder list
//! - RecoveryQuorumRequest: claimant's request, authored by hosting doorway
//! - KeyRotation: authoritative claim "new agent X is Matthew now"
//! - HeldRecoveryShare: private source-chain entry on holder devices
//! - MyRecoveryAuthorization: optional private audit log on holder devices

use hdi::prelude::*;

// =============================================================================
// Public Enums
// =============================================================================

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum RecoveryMode {
    Normal,
    Stewarded { grant_hash: ActionHash },
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
// RecoverySeedCommitment
// =============================================================================

/// Public commitment to a recovery seed: seed's public half + threshold params.
/// Share-holder identities are NOT stored here (privacy invariant).
/// Author must be the human_agent_pubkey (only the human commits their own seed).
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct RecoverySeedCommitment {
    pub human_agent_pubkey: AgentPubKey,
    pub seed_public_half: Vec<u8>,   // 32 bytes Ed25519 public key; Vec<u8> for serialize compat
    pub threshold_n: u8,
    pub total_m: u8,
    pub commitment_nonce: Vec<u8>,   // 16 bytes random
    pub created_at: Timestamp,
}

pub fn validate_recovery_seed_commitment(
    commitment: &RecoverySeedCommitment,
    action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: threshold range
    if commitment.threshold_n < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment threshold_n must be >= 2".to_string(),
        ));
    }
    if commitment.threshold_n > commitment.total_m {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment threshold_n must be <= total_m".to_string(),
        ));
    }
    // Rule 2: total_m range
    if commitment.total_m < 2 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment total_m must be >= 2".to_string(),
        ));
    }
    if commitment.total_m > 16 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment total_m must be <= 16".to_string(),
        ));
    }
    // Rule 3: seed_public_half must be 32 bytes
    if commitment.seed_public_half.len() != 32 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment seed_public_half must be exactly 32 bytes".to_string(),
        ));
    }
    // Rule 4: commitment_nonce must be 16 bytes
    if commitment.commitment_nonce.len() != 16 {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment commitment_nonce must be exactly 16 bytes".to_string(),
        ));
    }
    // Rule 5: author must be the committing human
    // action.author is the AgentPubKey of the source-chain author (Create action)
    if action.author != commitment.human_agent_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "RecoverySeedCommitment: author must equal human_agent_pubkey".to_string(),
        ));
    }
    // Rule 6: seed_public_half must be valid Ed25519 public key bytes
    // Note: byte-length check (Rule 3) is the validation floor here;
    // further cryptographic verification happens in coordinator flows.
    Ok(ValidateCallbackResult::Valid)
}

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
    pub recovery_mode: RecoveryMode,
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
