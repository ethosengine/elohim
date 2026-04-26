//! submit_specialist_revocation — defender producer for the
//! `trigger_type = "specialist_attestation"` revocation path.
//!
//! # Stage 1 (M5) gate
//!
//! The gate is a local stub `caller_is_defender_for` that returns `Ok(false)`
//! by default — intentionally default-deny per
//! `project_bootstrap_to_elohim_security_gradient` (Stage 1 is social +
//! structural; Stage 3 is full elohim-enforcement).
//!
//! **Task 15** will wire the gate to the real elohim-agent defender manifest via
//! the `gate-client-zome` `check_blocking` surface.  The stub signature is already
//! the correct integration point: replace the body, do not change callers.
//!
//! # KeyRevocation shape
//!
//! `KeyRevocation` uses `trigger_type: String`, not a Rust enum.  This function
//! sets `trigger_type = "specialist_attestation"` to distinguish its entries from
//! M4's `"voluntary"` and `"steward_vote"` paths.  The anomaly attestation body
//! (per `anomaly-attestation.schema.json`) is stored serialised into `votes_json`
//! — the legacy JSON blob field that M4 leaves empty.
//!
//! # Anchor / link pattern
//!
//! Mirrors `create_self_revocation` and `create_revocation_request` exactly:
//! - `IdToKeyRevocation`        — by revocation_id
//! - `HumanToKeyRevocation`     — by human_id (listing)
//! - `RevokedKeyToRevocation`   — by revoked key (hot-gate query)
//! - `EffectiveRevocations`     — specialist attestation is immediately effective
//!                                (single-agent authority, no quorum)
//! - Signal: `KeyRevocationRequested` + `KeyRevocationEffective` (same as
//!   `create_self_revocation`).

use hdk::prelude::*;
use imagodei_integrity::{EntryTypes, KeyRevocation, LinkTypes, StringAnchor};

use crate::{resolve_human_id_for_agent, RecoveryV2Signal, REVOCATION_REASONS};

// =============================================================================
// Input type
// =============================================================================

/// Input for `submit_specialist_revocation`.
///
/// `anomaly_attestation_json` carries the structured attestation body from the
/// elohim defender specialist as a pre-serialised JSON string.  The structured
/// shape is governed by `elohim/sdk/schemas/v1/agent/anomaly-attestation.schema.json`.
/// It is stored verbatim in `KeyRevocation.votes_json` so the elohim-storage
/// projection can surface it to elohim-app without lossy string truncation.
///
/// Caller (storage HTTP layer at Phase 11) is responsible for `JSON.stringify`
/// before invoking this zome function. The WASM zome boundary uses
/// `holochain_serialized_bytes` (MessagePack), which does not round-trip
/// `serde_json::Value` cleanly — hence the pre-serialised string.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSpecialistRevocationInput {
    /// ActionHash of the target Human entry (used to derive `human_id`).
    pub human_action_hash: ActionHash,
    /// The agent public key being revoked.
    pub revoked_pub_key: AgentPubKey,
    /// Free-text reason — must be one of `REVOCATION_REASONS`.
    pub reason: String,
    /// Pre-serialised JSON string of the anomaly attestation
    /// (matches `anomaly-attestation.schema.json`).
    pub anomaly_attestation_json: String,
}

// =============================================================================
// Stage 1 gate stub
// =============================================================================

/// Check whether the calling agent carries a defender role marker for the given
/// human.
///
/// **Stage 1 (M5): Always returns `Ok(false)` — default-deny.**
/// This enforces the Stage 1 security gradient: no agent is granted defender
/// authority until Task 15 wires the real elohim-agent defender manifest via
/// `gate_client_zome::check_blocking`.
///
/// # Task 15 integration point
///
/// Replace this body with:
/// ```rust,ignore
/// use gate_client_zome::{check_blocking, GateStatus, RelationalImpactEvent};
/// let event = RelationalImpactEvent::CapabilityInvoke {
///     capability: "defender".to_string(),
///     requester: agent_info()?.agent_initial_pubkey.to_string(),
///     request_id: format!("defender:{}", human_action_hash),
/// };
/// let decision = check_blocking(event)
///     .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("gate error: {e}"))))?;
/// Ok(decision.is_allowed())
/// ```
/// gate-client-zome must also be added to the coordinator Cargo.toml dependency
/// list (currently absent — adding it is out of scope for Task 4).
///
/// Per memory `project_bootstrap_to_elohim_security_gradient`: Stage 1 default-deny
/// is correct here; full elohim-enforcement (Stage 3) lands with Task 15.
#[allow(unused_variables)]
fn caller_is_defender_for(human_action_hash: &ActionHash) -> ExternResult<bool> {
    // TODO(task-15): replace with real gate_client_zome::check_blocking call.
    // See module-level doc for the integration point signature.
    Ok(false)
}

// =============================================================================
// Coordinator function
// =============================================================================

/// Submit a specialist-attested key revocation.
///
/// The calling agent must carry a defender role marker (checked via
/// `caller_is_defender_for` — stub in Stage 1, real gate in Stage 3 / Task 15).
///
/// On success the revocation is immediately effective (no quorum required for a
/// defender attestation); both `KeyRevocationRequested` and
/// `KeyRevocationEffective` signals are emitted atomically.
#[hdk_extern]
pub fn submit_specialist_revocation(
    input: SubmitSpecialistRevocationInput,
) -> ExternResult<ActionHash> {
    // --- Gate: defender role marker ---
    if !caller_is_defender_for(&input.human_action_hash)? {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "submit_specialist_revocation: caller is not a configured defender for this human \
             (Stage 1: gate is default-deny; Task 15 will wire the elohim-agent manifest)"
                .to_string()
        )));
    }

    // --- Validate reason ---
    if !REVOCATION_REASONS.contains(&input.reason.as_str()) {
        return Err(wasm_error!(WasmErrorInner::Guest(format!(
            "submit_specialist_revocation: invalid reason '{}'. Must be one of {:?}",
            input.reason, REVOCATION_REASONS
        ))));
    }

    // --- Resolve human_id from the Human record ---
    let caller_pubkey = agent_info()?.agent_initial_pubkey;
    let caller_human_id = resolve_human_id_for_agent(&caller_pubkey)?;

    // Confirm the target ActionHash resolves to a valid record (deterministic
    // integrity check; does not traverse links).
    let human_record = must_get_valid_record(input.human_action_hash.clone())?;
    let target_human_id: String = {
        use imagodei_integrity::Human;
        let h: Human = human_record
            .entry()
            .to_app_option()
            .map_err(|e| {
                wasm_error!(WasmErrorInner::Guest(format!(
                    "human_action_hash does not resolve to a Human entry: {e:?}"
                )))
            })?
            .ok_or_else(|| {
                wasm_error!(WasmErrorInner::Guest(
                    "Human entry missing from human_action_hash record".to_string()
                ))
            })?;
        h.id
    };

    // --- Verify the revoked key belongs to the target human ---
    let owner_human_id = resolve_human_id_for_agent(&input.revoked_pub_key)?;
    if owner_human_id != target_human_id {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "submit_specialist_revocation: revoked_pub_key does not belong to target human"
                .to_string()
        )));
    }

    // anomaly_attestation_json arrives pre-serialised — store it verbatim.
    let attestation_json = input.anomaly_attestation_json.clone();

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let revocation_id = format!("rev-{}-{}", target_human_id, timestamp);
    let revoked_key_str = input.revoked_pub_key.to_string();

    // --- Build KeyRevocation entry ---
    let revocation = KeyRevocation {
        id: revocation_id.clone(),
        human_id: target_human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        initiated_by: caller_human_id.clone(),
        trigger_type: "specialist_attestation".to_string(),
        // Defender attestation is single-agent authority; no quorum needed.
        required_votes: 1,
        current_votes: 1,
        // votes_json stores the anomaly_attestation body (legacy blob field).
        votes_json: attestation_json,
        threshold_reached: true,
        effective_at: Some(timestamp.clone()),
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };

    let action_hash = create_entry(&EntryTypes::KeyRevocation(revocation.clone()))?;

    // --- IdToKeyRevocation anchor ---
    let id_anchor = StringAnchor::new("revocation_id", &revocation_id);
    let id_anchor_hash = hash_entry(&EntryTypes::StringAnchor(id_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(id_anchor))?;
    create_link(
        id_anchor_hash,
        action_hash.clone(),
        LinkTypes::IdToKeyRevocation,
        (),
    )?;

    // --- HumanToKeyRevocation anchor (listing by human) ---
    let human_anchor = StringAnchor::new("human_revocations", &target_human_id);
    let human_anchor_hash = hash_entry(&EntryTypes::StringAnchor(human_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(human_anchor))?;
    create_link(
        human_anchor_hash,
        action_hash.clone(),
        LinkTypes::HumanToKeyRevocation,
        (),
    )?;

    // --- RevokedKeyToRevocation anchor (hot-gate query) ---
    let revoked_key_anchor = StringAnchor::new("revoked_key", &revoked_key_str);
    let revoked_key_anchor_hash =
        hash_entry(&EntryTypes::StringAnchor(revoked_key_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(revoked_key_anchor))?;
    create_link(
        revoked_key_anchor_hash,
        action_hash.clone(),
        LinkTypes::RevokedKeyToRevocation,
        (),
    )?;

    // --- EffectiveRevocations anchor — defender attestation is immediately effective ---
    let effective_anchor = StringAnchor::new("effective_revocations", "global");
    let effective_anchor_hash = hash_entry(&EntryTypes::StringAnchor(effective_anchor.clone()))?;
    create_entry(&EntryTypes::StringAnchor(effective_anchor))?;
    create_link(
        effective_anchor_hash,
        action_hash.clone(),
        LinkTypes::EffectiveRevocations,
        (),
    )?;

    // --- Emit signals (Requested + Effective atomically, same as create_self_revocation) ---
    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation.id.clone(),
        human_id: revocation.human_id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        reason: revocation.reason.clone(),
        trigger_type: revocation.trigger_type.clone(),
        initiated_by: revocation.initiated_by.clone(),
        required_votes: revocation.required_votes,
        current_votes: revocation.current_votes,
        threshold_reached: revocation.threshold_reached,
        effective_at: revocation.effective_at.clone(),
        created_at: revocation.created_at.clone(),
    })?;

    emit_signal(RecoveryV2Signal::KeyRevocationEffective {
        revocation_id: revocation.id.clone(),
        revoked_key: revocation.revoked_key.clone(),
        human_id: revocation.human_id.clone(),
        effective_at: timestamp,
        triggering_vote_id: None,
    })?;

    Ok(action_hash)
}
