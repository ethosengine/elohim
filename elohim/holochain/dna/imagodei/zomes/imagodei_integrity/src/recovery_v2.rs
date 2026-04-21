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

// =============================================================================
// Pure-Logic Helpers (unit-testable without HDI runtime)
// =============================================================================

/// Decode a base64 string to exactly 32 bytes. Pure; unit-testable.
pub fn base64_decode_32(s: &str) -> Result<[u8; 32], String> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let bytes = STANDARD
        .decode(s)
        .map_err(|e| format!("base64 decode failed: {e}"))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| format!("expected 32 bytes, got {}", v.len()))
}

/// Pure-logic rules for IntimateQuorum validation. Takes pre-resolved witnesses
/// (and their authors) so unit tests don't need must_get_valid_record.
///
/// Returns Valid on pass; Invalid(reason) on any rule violation.
pub fn check_intimate_quorum_rules(
    request: &super::RecoveryRequest,
    resolved_witnesses: &[(super::HumanityWitness, AgentPubKey)],
) -> ValidateCallbackResult {
    // Absolute floor: no fewer than 2 witnesses, ever.
    if resolved_witnesses.len() < 2 {
        return ValidateCallbackResult::Invalid(
            "IntimateQuorum requires at least 2 witnesses".to_string(),
        );
    }

    // Threshold floor from coordinator-computed request.required_witness_count.
    if (resolved_witnesses.len() as u32) < request.required_witness_count {
        return ValidateCallbackResult::Invalid(format!(
            "IntimateQuorum requires {} witnesses; got {}",
            request.required_witness_count,
            resolved_witnesses.len(),
        ));
    }

    // Rule: all witnesses target the same human_id.
    let first_human_id = &resolved_witnesses[0].0.human_id;
    if resolved_witnesses.iter().any(|(w, _)| &w.human_id != first_human_id) {
        return ValidateCallbackResult::Invalid(
            "IntimateQuorum witnesses disagree on human_id target".to_string(),
        );
    }

    // Rule: target human_id matches request.human_id (coordinator must populate).
    let req_human_id = match &request.human_id {
        Some(id) => id,
        None => return ValidateCallbackResult::Invalid(
            "IntimateQuorum requires RecoveryRequest.human_id to be populated by coordinator".to_string(),
        ),
    };
    if first_human_id != req_human_id {
        return ValidateCallbackResult::Invalid(
            "IntimateQuorum witness human_id does not match RecoveryRequest.human_id".to_string(),
        );
    }

    // Rule: no witness is explicitly revoked.
    if resolved_witnesses.iter().any(|(w, _)| w.revoked_at.is_some()) {
        return ValidateCallbackResult::Invalid(
            "IntimateQuorum includes a revoked HumanityWitness".to_string(),
        );
    }

    // Rule: distinct authors (no double-voting).
    let mut seen: Vec<&AgentPubKey> = Vec::with_capacity(resolved_witnesses.len());
    for (_, author) in resolved_witnesses {
        if seen.contains(&author) {
            return ValidateCallbackResult::Invalid(
                "IntimateQuorum witnesses must have distinct authors".to_string(),
            );
        }
        seen.push(author);
    }

    // Defense-in-depth: distinct count >= required (redundant with distinct-check above).
    if (seen.len() as u32) < request.required_witness_count {
        return ValidateCallbackResult::Invalid(format!(
            "IntimateQuorum distinct authors {} below required {}",
            seen.len(),
            request.required_witness_count,
        ));
    }

    ValidateCallbackResult::Valid
}

/// Pure-logic rules for CryptographicQuorum validation. Takes pre-resolved
/// stewardship + pre-extracted raw bytes so unit tests don't need must_get_valid_record.
pub fn check_cryptographic_quorum_rules(
    stewardship: &super::KeyStewardship,
    new_agent_pubkey_raw: &[u8],
    recovery_request_hash_raw: &[u8],
    quorum_signature: &[u8],
) -> ValidateCallbackResult {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    // Non-superseded check.
    if stewardship.rotated_at.is_some() {
        return ValidateCallbackResult::Invalid(
            "CryptographicQuorum references a superseded KeyStewardship".to_string(),
        );
    }

    // Signature length check.
    let sig_bytes: [u8; 64] = match quorum_signature.try_into() {
        Ok(b) => b,
        Err(_) => {
            return ValidateCallbackResult::Invalid(format!(
                "CryptographicQuorum quorum_signature must be 64 bytes, got {}",
                quorum_signature.len(),
            ))
        }
    };

    // Decode verifying key from shard_commitment_hash (per spec §3.1).
    let vk_bytes = match base64_decode_32(&stewardship.shard_commitment_hash) {
        Ok(b) => b,
        Err(e) => {
            return ValidateCallbackResult::Invalid(format!(
                "CryptographicQuorum KeyStewardship.shard_commitment_hash not a base64 32-byte Ed25519 key: {e}",
            ))
        }
    };
    let vk = match VerifyingKey::from_bytes(&vk_bytes) {
        Ok(k) => k,
        Err(e) => {
            return ValidateCallbackResult::Invalid(format!(
                "CryptographicQuorum shard_commitment_hash is not a valid Ed25519 verifying key: {e}",
            ))
        }
    };

    // Message: new_agent_pubkey (39 bytes) || recovery_request_hash (39 bytes).
    let mut message: Vec<u8> = Vec::with_capacity(new_agent_pubkey_raw.len() + recovery_request_hash_raw.len());
    message.extend_from_slice(new_agent_pubkey_raw);
    message.extend_from_slice(recovery_request_hash_raw);

    let sig = Signature::from_bytes(&sig_bytes);
    match vk.verify(&message, &sig) {
        Ok(()) => ValidateCallbackResult::Valid,
        Err(_) => ValidateCallbackResult::Invalid(
            "CryptographicQuorum signature verification failed".to_string(),
        ),
    }
}

/// Pure-logic rules for freeze-floor check. Takes pre-filtered active freezes
/// targeting this human. Caller is responsible for skipping this check for
/// CryptographicQuorum (orthogonal exemption).
///
/// Returns None if no blocking freeze exists; Some(reason) if one does.
pub fn check_freeze_floor_rules(
    authority: &RecoveryAuthority,
    human_id: &str,
    active_freezes_for_human: &[&super::IdentityFreeze],
) -> Option<String> {
    let rotation_layer = authority_layer_name(authority);
    let rotation_rank = authority_layer_rank(rotation_layer)?;
    // rank is None for cryptographic; caller must have skipped us.

    for freeze in active_freezes_for_human {
        if !freeze.is_active {
            continue;
        }
        if freeze.human_id != human_id {
            continue;
        }

        // None defaults to "intimate" (most restrictive).
        let frozen_layer = freeze.frozen_at_layer.as_deref().unwrap_or(LAYER_INTIMATE);
        let frozen_rank = match authority_layer_rank(frozen_layer) {
            Some(r) => r,
            None => continue, // cryptographic freeze doesn't participate in ordering
        };

        // Rule: rotation rank must strictly exceed freeze rank to proceed.
        if rotation_rank <= frozen_rank {
            return Some(format!(
                "KeyRotation blocked by active IdentityFreeze at layer '{frozen_layer}'; \
                 rotation layer '{rotation_layer}' must exceed frozen layer to proceed",
            ));
        }
    }

    None
}

// =============================================================================
// HDI Wrappers (resolve DHT records, delegate to pure-logic helpers)
// =============================================================================

/// HDI wrapper: resolve witness hashes to records, extract authors, delegate to pure-logic helper.
fn validate_intimate_quorum(
    request: &super::RecoveryRequest,
    witness_hashes: &[ActionHash],
) -> ExternResult<ValidateCallbackResult> {
    if witness_hashes.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "IntimateQuorum witness_hashes cannot be empty".to_string(),
        ));
    }

    let mut resolved: Vec<(super::HumanityWitness, AgentPubKey)> =
        Vec::with_capacity(witness_hashes.len());
    for h in witness_hashes {
        let rec = must_get_valid_record(h.clone())?;
        let author = rec.action().author().clone();
        let witness: super::HumanityWitness = rec
            .entry()
            .to_app_option()
            .map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "witness_hash does not resolve to HumanityWitness: {e:?}"
                )))
            })?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "HumanityWitness entry missing".to_string()
            )))?;
        resolved.push((witness, author));
    }

    Ok(check_intimate_quorum_rules(request, &resolved))
}

/// HDI wrapper: resolve stewardship, extract raw bytes, delegate to pure-logic helper.
fn validate_cryptographic_quorum(
    rotation: &KeyRotation,
    stewardship_hash: &ActionHash,
    quorum_signature: &[u8],
) -> ExternResult<ValidateCallbackResult> {
    let rec = must_get_valid_record(stewardship_hash.clone())?;
    let stewardship: super::KeyStewardship = rec
        .entry()
        .to_app_option()
        .map_err(|e| {
            wasm_error!(WasmErrorInner::Guest(format!(
                "stewardship_hash does not resolve to KeyStewardship: {e:?}"
            )))
        })?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "KeyStewardship entry missing".to_string()
        )))?;

    Ok(check_cryptographic_quorum_rules(
        &stewardship,
        rotation.new_agent_pubkey.get_raw_39(),
        rotation.recovery_request_hash.get_raw_39(),
        quorum_signature,
    ))
}

/// HDI wrapper for the freeze-floor check.
///
/// NOTE (M2 deviation from plan): The integrity zome uses `hdi::prelude::*`, which
/// does NOT expose `get_links` or `GetLinksInputBuilder`. Those are HDK-only (coordinator).
/// The validation callback context supports `must_get_valid_record` and `must_get_entry`
/// for deterministic record retrieval, but NOT link traversal (which is non-deterministic).
///
/// Enforcement model: coordinator-level enforcement (M5) gates `KeyRotation` commits by
/// checking active freezes before allowing the coordinator function to proceed. The pure-logic
/// helper `check_freeze_floor_rules` is fully tested and correct; it will be called from
/// the M5 coordinator with resolved freezes before the entry is committed.
///
/// Returns Ok(None) — no blocking freeze detected at validator time (zero freezes visible).
/// Caller is responsible for skipping this helper for CryptographicQuorum.
fn check_freeze_floor(
    _authority: &RecoveryAuthority,
    _request_human_id: &Option<String>,
) -> ExternResult<Option<String>> {
    // Link traversal (get_links) is not available in integrity zome validation callbacks.
    // Freeze-floor enforcement deferred to coordinator-level gating in M5.
    // Pure-logic rules are in check_freeze_floor_rules (unit-tested, correct).
    Ok(None)
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

