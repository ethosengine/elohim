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
//! # Recovery M4 Task 14 — cross-DNA bridge
//!
//! The canonical revocation record is now a
//! `governance-action:key-revocation` Content entry on the elohim DNA,
//! committed via `call_elohim_propose_recovery_governance_action`. Defender
//! authority is unilateral, so the entry is written with
//! `threshold_reached: true` and a non-null `effective_at` on the initial
//! CREATE — the Task 4 `query_effective_revocation_for_key` gate returns it
//! directly without ever needing a vote / supersession (CREATE-only
//! constraint, Task 4).
//!
//! The legacy local `KeyRevocation` entry + four anchor links
//! (`IdToKeyRevocation`, `HumanToKeyRevocation`, `RevokedKeyToRevocation`,
//! `EffectiveRevocations`) are no longer written here. Task 15 will retire
//! the integrity definitions entirely. Signals (`KeyRevocationRequested` +
//! `KeyRevocationEffective`) continue to emit for back-compat; Task 17 will
//! re-align the payloads to the `dna-signals/*` schemas.

use hdk::prelude::*;

use crate::{
    call_elohim_propose_recovery_governance_action, resolve_human_id_for_agent,
    ConsolidatedProposeRecoveryGovernanceActionInput, RecoveryV2Signal, REVOCATION_REASONS,
};

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

/// Output of `submit_specialist_revocation` — Recovery M4 Task 14.
///
/// The canonical record is a `governance-action:key-revocation` Content
/// entry on the elohim DNA (no local ActionHash). Mirrors
/// `KeyRevocationOutput` from `create_self_revocation` /
/// `create_revocation_request` post-T14.
#[derive(Serialize, Deserialize, SerializedBytes, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SubmitSpecialistRevocationOutput {
    pub revocation_id: String,
    pub revocation_cid: String,
}

/// Submit a specialist-attested key revocation.
///
/// The calling agent must carry a defender role marker (checked via
/// `caller_is_defender_for` — stub in Stage 1, real gate in Stage 3 / Task 15).
///
/// Recovery M4 Task 14: writes the revocation as a
/// `governance-action:key-revocation` Content entry on the elohim DNA via
/// `call_elohim_propose_recovery_governance_action`. Defender authority is
/// unilateral (single-agent attestation), so the entry is committed with
/// `threshold_reached: true` and a non-null `effective_at` on the initial
/// CREATE — the Task 4 gate `query_effective_revocation_for_key` returns it
/// directly without ever needing a vote or supersession.
///
/// The legacy local `KeyRevocation` entry + four anchor links are no longer
/// written. Both `KeyRevocationRequested` and `KeyRevocationEffective`
/// signals are emitted atomically for back-compat (Task 17 will re-align
/// payloads to the dna-signals schemas).
#[hdk_extern]
pub fn submit_specialist_revocation(
    input: SubmitSpecialistRevocationInput,
) -> ExternResult<SubmitSpecialistRevocationOutput> {
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

    // anomaly_attestation_json arrives pre-serialised — preserve verbatim in
    // the cross-DNA Content metadata so the elohim-storage projection can
    // surface it without lossy truncation.
    let attestation_json = input.anomaly_attestation_json.clone();

    let now = sys_time()?;
    let timestamp = format!("{:?}", now);
    let revocation_id = format!("rev-{}-{}", target_human_id, timestamp);
    let revoked_key_str = input.revoked_pub_key.to_string();

    // --- Bridge: commit governance-action:key-revocation on elohim DNA ---
    let bridge_input = ConsolidatedProposeRecoveryGovernanceActionInput {
        governance_kind: "governance-action:key-revocation".to_string(),
        subject_human_id: target_human_id.clone(),
        title: format!(
            "Specialist revocation of {} for {}",
            revoked_key_str, target_human_id
        ),
        description: Some(format!(
            "trigger_type=specialist_attestation; reason={} (defender attestation)",
            input.reason
        )),
        reach: "intimate".to_string(),
        threshold: serde_json::json!({"m": 1}),
        // Defender attestation is immediately effective; closes_at is
        // informational only — set to far-future so projectors that honour
        // closes_at for sweep windows do not prematurely expire it.
        closes_at: "2099-01-01T00:00:00Z".to_string(),
        metadata: serde_json::json!({
            "id": revocation_id,
            "human_id": target_human_id,
            "revoked_key": revoked_key_str,
            "reason": input.reason,
            "trigger_type": "specialist_attestation",
            "initiated_by": caller_human_id,
            "required_votes": 1,
            "current_votes": 1,
            // Defender authority = immediately effective on the initial
            // CREATE (no update_entry per Task 4 CREATE-only constraint).
            "threshold_reached": true,
            "effective_at": timestamp,
            "anomaly_attestation_json": attestation_json,
        }),
        supersedes_cid: None,
    };
    let consolidated = call_elohim_propose_recovery_governance_action(bridge_input)?;
    let revocation_cid = consolidated.cid;

    // --- Emit signals (Requested + Effective atomically) ---
    emit_signal(RecoveryV2Signal::KeyRevocationRequested {
        id: revocation_id.clone(),
        human_id: target_human_id.clone(),
        revoked_key: revoked_key_str.clone(),
        reason: input.reason.clone(),
        trigger_type: "specialist_attestation".to_string(),
        initiated_by: caller_human_id.clone(),
        required_votes: 1,
        current_votes: 1,
        threshold_reached: true,
        effective_at: Some(timestamp.clone()),
        created_at: timestamp.clone(),
    })?;

    emit_signal(RecoveryV2Signal::KeyRevocationEffective {
        revocation_id: revocation_id.clone(),
        revoked_key: revoked_key_str,
        human_id: target_human_id,
        effective_at: timestamp,
        triggering_vote_id: None,
    })?;

    Ok(SubmitSpecialistRevocationOutput {
        revocation_id,
        revocation_cid,
    })
}
