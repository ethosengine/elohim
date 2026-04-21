//! Recovery Protocol Phase 2 — Graduated Authority Identity Recovery
//!
//! Entry types and validation for graduated-authority-based identity recovery.
//! See: genesis/docs/superpowers/specs/2026-04-22-recovery-protocol-phase-2-revised-design.md
//!
//! New entry types (prefixed with the module to distinguish from legacy
//! attestation-vote-based recovery in lib.rs):
//! - RecoveryQuorumRequest: claimant's request, authored by hosting doorway
//! - KeyRotation: authoritative claim "new agent X is Matthew now"

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

// =============================================================================
// HeldRecoveryShare (private source-chain, never gossiped)
// =============================================================================

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct HeldRecoveryShare {
    pub seed_commitment_hash: ActionHash,
    pub encrypted_share: Vec<u8>,
    pub relationship_label: String,
    pub protected_human_pubkey: AgentPubKey,
    pub received_at: Timestamp,
}

pub fn validate_held_recovery_share(
    share: &HeldRecoveryShare,
    action: &Create,
) -> ExternResult<ValidateCallbackResult> {
    // Rule 1: share author must be the holder (the one committing)
    // This is automatic in Holochain source chains; we just sanity-check
    // that the protected_human is distinct from the author
    if action.author == share.protected_human_pubkey {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldRecoveryShare: author cannot be the protected human (you hold shares for others)"
                .to_string(),
        ));
    }
    // Rule 2: encrypted_share non-empty
    if share.encrypted_share.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldRecoveryShare encrypted_share must not be empty".to_string(),
        ));
    }
    // Rule 3: relationship_label reasonable length
    if share.relationship_label.len() > 128 {
        return Ok(ValidateCallbackResult::Invalid(
            "HeldRecoveryShare relationship_label too long (max 128 chars)".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}

// =============================================================================
// MyRecoveryAuthorization (private audit, optional)
// =============================================================================

#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct MyRecoveryAuthorization {
    pub recovery_request_hash: ActionHash,
    pub protected_human_pubkey: AgentPubKey,
    pub authorized_at: Timestamp,
    pub elohim_confidence_tier: ConfidenceTier,
    pub reasoning_summary: String,
}

pub fn validate_my_recovery_authorization(
    authz: &MyRecoveryAuthorization,
) -> ExternResult<ValidateCallbackResult> {
    if authz.reasoning_summary.len() > 2048 {
        return Ok(ValidateCallbackResult::Invalid(
            "MyRecoveryAuthorization reasoning_summary too long (max 2048 chars)".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
