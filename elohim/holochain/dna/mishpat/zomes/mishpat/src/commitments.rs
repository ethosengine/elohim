//! Mishpat Commitment coordinator — Z.D substrate-correct deploy primitive.
//!
//! Authors the `delegates-compute` and `acknowledges-reach-change` Commitment
//! actions per genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md.
//!
//! See `bounds-validator-pattern` memory: per-instance validators consume the
//! Commitment via `services::commitment_fetcher::CommitmentFetcher` and
//! `services::bounds_validator::validate` in elohim-storage.

use hdk::prelude::*;
use mishpat_integrity::{Commitment, LinkTypes};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentInput {
    pub action: String,
    pub payload_json: String,
    /// Caller-supplied ISO-8601 (or epoch-seconds) timestamp. Replaces the
    /// in-zome `sys_time()` call so the notarized commitment carries a
    /// deterministic, caller-controlled signing time (Slice 2b T1). The
    /// projection writes this onto the `mishpat_commitments` row; the bounds
    /// validator still reads `valid_from`/`valid_until` from `payload_json`.
    pub signed_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentOutput {
    pub action_hash: ActionHash,
    pub entry_hash: EntryHash,
}

#[hdk_extern]
pub fn create_commitment(input: CreateCommitmentInput) -> ExternResult<CommitmentOutput> {
    validate_commitment_payload(&input).map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

    let entry = Commitment {
        action: input.action.clone(),
        payload_json: input.payload_json.clone(),
        signed_at: input.signed_at.clone(),
    };

    let action_hash = create_entry(&mishpat_integrity::EntryTypes::Commitment(entry.clone()))?;
    let entry_hash = hash_entry(&entry)?;
    Ok(CommitmentOutput {
        action_hash,
        entry_hash,
    })
}

// =============================================================================
// CommitmentByState link author (Slice-2b T11)
// =============================================================================
//
// Records a Commitment's lifecycle transition (proposed → active → …) as an
// immutable `CommitmentByState` link on the Mishpat DHT. The SQL `state` column
// in elohim-storage becomes a write-through cache: `graduate_to_active` writes
// the cache; this link is the truth. Peers verify lifecycle by reading the link
// off the commitment anchor — no need to replay every EconomicEvent.

/// Input for `create_commitment_state_link`. `commitment_cid` is the base64
/// `EntryHash` of the Commitment (the same value elohim-storage stores as
/// `mishpat_commitments.cid` and `get_commitment` takes). `event_hash` is the
/// base64 `ActionHash` of the EconomicEvent that justifies the transition.
/// `signed_at` is the deterministic, caller-supplied signing time (Category-A —
/// never `sys_time()` in-zome).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CreateCommitmentStateLinkInput {
    pub commitment_cid: String,
    pub state: String,
    pub event_hash: String,
    pub signed_at: String,
}

/// Output of `create_commitment_state_link` — the new link's `ActionHash`.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentStateLinkOutput {
    pub link_action_hash: ActionHash,
}

/// Author a `CommitmentByState` link recording a commitment's state transition.
///
/// - **base** = the Commitment's `EntryHash` (resolved from `commitment_cid`) —
///   so the link is readable off the commitment anchor by any peer.
/// - **target** = the graduating event's `ActionHash` (resolved from
///   `event_hash`) — the proof a verifier can replay.
/// - **tag** = `"<state>|<signed_at>"` — the new lifecycle state + signing time;
///   the integrity zome enforces both segments are non-empty.
///
/// The link is immutable (links never update). Called by the elohim-storage
/// graduation projection right after `graduate_to_active` flips the SQL cache.
#[hdk_extern]
pub fn create_commitment_state_link(
    input: CreateCommitmentStateLinkInput,
) -> ExternResult<CommitmentStateLinkOutput> {
    if input.state.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_commitment_state_link: state must be non-empty".to_string()
        )));
    }
    if input.signed_at.is_empty() {
        return Err(wasm_error!(WasmErrorInner::Guest(
            "create_commitment_state_link: signed_at must be non-empty".to_string()
        )));
    }

    // The commitment anchor: base64 EntryHash → EntryHash (the link base).
    let base = EntryHash::try_from(input.commitment_cid.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "create_commitment_state_link: invalid commitment_cid (expected base64 EntryHash): {}",
            input.commitment_cid
        )))
    })?;

    // The transition proof: base64 ActionHash → ActionHash (the link target).
    let target = ActionHash::try_from(input.event_hash.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "create_commitment_state_link: invalid event_hash (expected base64 ActionHash): {}",
            input.event_hash
        )))
    })?;

    // Tag carries the new state + signing time: "<state>|<signed_at>".
    let tag_str = format!("{}|{}", input.state, input.signed_at);
    let link_action_hash = create_link(
        base,
        target,
        LinkTypes::CommitmentByState,
        LinkTag::new(tag_str.as_bytes().to_vec()),
    )?;

    Ok(CommitmentStateLinkOutput { link_action_hash })
}

/// A `CommitmentByState` link projected to a wire shape. `state`/`signed_at`
/// are parsed from the LinkTag (`"<state>|<signed_at>"`); `event_hash` is the
/// link target (the graduating event's ActionHash) as base64.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CommitmentStateLink {
    pub state: String,
    pub signed_at: String,
    pub event_hash: String,
}

/// Read all `CommitmentByState` links off a commitment anchor (base64
/// `EntryHash`). Returns the lifecycle transitions notarized on the DHT —
/// the peer-observable lifecycle without replaying every EconomicEvent.
///
/// Used by the Slice-2b T11 sweettest to verify cross-conductor replication,
/// and available to any storage observer that wants to project lifecycle from
/// DHT-truth rather than the SQL write-through cache.
#[hdk_extern]
pub fn get_commitment_state_links(
    commitment_cid: String,
) -> ExternResult<Vec<CommitmentStateLink>> {
    let base = EntryHash::try_from(commitment_cid.clone()).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest(format!(
            "get_commitment_state_links: invalid commitment_cid (expected base64 EntryHash): {commitment_cid}"
        )))
    })?;

    let query = LinkQuery::try_new(base, LinkTypes::CommitmentByState)?;
    let links = get_links(query, GetStrategy::default())?;

    let mut out = Vec::with_capacity(links.len());
    for link in links {
        let raw = String::from_utf8(link.tag.0.clone()).unwrap_or_default();
        let mut parts = raw.splitn(2, '|');
        let state = parts.next().unwrap_or("").to_string();
        let signed_at = parts.next().unwrap_or("").to_string();
        // Target is the graduating event's ActionHash; render as base64.
        let event_hash = ActionHash::try_from(link.target.clone())
            .map(|h| h.to_string())
            .unwrap_or_default();
        out.push(CommitmentStateLink {
            state,
            signed_at,
            event_hash,
        });
    }
    Ok(out)
}

/// Validate the commitment payload against the action-specific schema.
/// For `delegates-compute` action, validates against delegates-compute.schema.json
/// (hand-rolled — HDK WASM size constraints prohibit a full JSON Schema library).
pub fn validate_commitment_payload(input: &CreateCommitmentInput) -> Result<(), String> {
    let payload: serde_json::Value = serde_json::from_str(&input.payload_json)
        .map_err(|e| format!("payload_json not parseable: {e}"))?;

    match input.action.as_str() {
        "delegates-compute" => validate_delegates_compute(&payload),
        // TODO(sprint1-task3): implement acknowledges-reach-change validation
        "acknowledges-reach-change" => validate_acknowledges_reach_change(&payload),
        "replicates-dwelling" => validate_replicates_dwelling(&payload),
        "replicates-commons" => validate_replicates_commons(&payload),
        "revokes-commitment" => validate_revokes_commitment(&payload),
        other => Err(format!(
            "commitments::validate_commitment_payload unhandled action: {other}"
        )),
    }
}

/// Validator for the `replicates-commons` action (EPR provide loop, Slice-2b T3).
/// Variant-dispatch on `variant` ("content" | "capacity"), reach-must-be-commons,
/// and (capacity only) ratio_attestation sum-to-100 + effective_ratio_cid present.
/// Mirrors `validate_replicates_dwelling`'s hand-rolled style.
///
/// Note: the author does NOT supply `epr_scope`. For a content-scoped commons
/// commitment the effective `bounds.epr_scope` is derived as `[head_ref]` at
/// projection time (the storage `parse_replicates_commons` projection), so the
/// bounds-validator's epr_scope check is satisfied downstream — never required
/// in the author-facing payload here.
fn validate_replicates_commons(payload: &serde_json::Value) -> Result<(), String> {
    if payload["action"] != "replicates-commons" {
        return Err("action field must equal 'replicates-commons'".into());
    }

    // bounds: required object with rate_per_minute and reach_ceiling="commons".
    let bounds = payload
        .get("bounds")
        .and_then(|b| b.as_object())
        .ok_or_else(|| "replicates-commons bounds must be object".to_string())?;
    for field in ["rate_per_minute", "reach_ceiling"] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // rate_per_minute must be a positive rate — a zero rate is a no-op
    // commitment (mirrors replicates-dwelling's positive-quantity guards).
    if bounds["rate_per_minute"].as_u64().unwrap_or(0) == 0 {
        return Err("replicates-commons bounds.rate_per_minute must be > 0".into());
    }
    if bounds["reach_ceiling"].as_str().unwrap_or("") != "commons" {
        return Err("bounds.reach_ceiling must equal 'commons'".into());
    }

    // variant dispatch.
    let variant = payload["variant"].as_str().unwrap_or("");
    match variant {
        "content" => {
            for field in ["head_ref", "reach"] {
                if payload.get(field).is_none() {
                    return Err(format!(
                        "replicates-commons content variant missing field: {field}"
                    ));
                }
            }
            if payload["head_ref"].as_str().unwrap_or("").is_empty() {
                return Err("replicates-commons head_ref must be non-empty".into());
            }
            // commons-reach is the ONLY admissible reach for the commons provide loop.
            if payload["reach"].as_str().unwrap_or("") != "commons" {
                return Err("replicates-commons content reach must equal 'commons'".into());
            }
            // content variant carries NO ratio_attestation.
            if payload.get("ratio_attestation").is_some() {
                return Err(
                    "replicates-commons content variant must not carry ratio_attestation".into(),
                );
            }
            Ok(())
        }
        "capacity" => {
            // commons_bytes > 0.
            let bytes = payload["commons_bytes"].as_u64().unwrap_or(0);
            if bytes == 0 {
                return Err("replicates-commons commons_bytes must be > 0".into());
            }
            // ratio_attestation: required sub-fields + sum-to-100 (mirrors replicates-dwelling).
            let attestation = payload
                .get("ratio_attestation")
                .and_then(|v| v.as_object())
                .ok_or("replicates-commons capacity variant requires ratio_attestation object")?;
            for f in [
                "commons_pct",
                "dwelling_pct",
                "collective_pct",
                "free_pct",
                "effective_ratio_cid",
            ] {
                if !attestation.contains_key(f) {
                    return Err(format!("ratio_attestation missing field: {f}"));
                }
            }
            if attestation["effective_ratio_cid"]
                .as_str()
                .unwrap_or("")
                .is_empty()
            {
                return Err("ratio_attestation effective_ratio_cid must be non-empty".into());
            }
            let commons = attestation["commons_pct"].as_u64().unwrap_or(0);
            let dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0);
            let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
            let free = attestation["free_pct"].as_u64().unwrap_or(0);
            if commons + dwelling + collective + free != 100 {
                return Err(format!(
                    "ratio_attestation pct sum {} != 100",
                    commons + dwelling + collective + free
                ));
            }
            Ok(())
        }
        other => Err(format!(
            "replicates-commons variant '{other}' not in enum (content|capacity)"
        )),
    }
}

fn validate_revokes_commitment(payload: &serde_json::Value) -> Result<(), String> {
    if payload["action"] != "revokes-commitment" {
        return Err("action field must equal 'revokes-commitment'".into());
    }
    let target = payload
        .get("target_cid")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if target.is_empty() {
        return Err("revokes-commitment target_cid must be non-empty".into());
    }
    let signed = payload
        .get("signed_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if signed.is_empty() {
        return Err("revokes-commitment signed_at must be present".into());
    }
    Ok(())
}

fn validate_delegates_compute(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "scope",
        "provider",
        "recipient",
        "bounds",
        "valid_from",
        "valid_until",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("delegates-compute missing required field: {field}"));
        }
    }
    if payload["action"] != "delegates-compute" {
        return Err("action field must equal 'delegates-compute'".into());
    }
    let bounds = payload
        .get("bounds")
        .and_then(|b| b.as_object())
        .ok_or_else(|| "bounds must be object".to_string())?;
    for field in [
        "epr_scope",
        "reach_ceiling",
        "rate_per_hour",
        "rotation_ttl_days",
    ] {
        if !bounds.contains_key(field) {
            return Err(format!("bounds missing required field: {field}"));
        }
    }
    // reach_ceiling above commons/community requires reach_elevation_acknowledged=true.
    // commons/community are the default-allowed ceilings; anything more permissive
    // (public, or higher in the protocol's reach hierarchy) is an escalation.
    let ceiling = bounds["reach_ceiling"].as_str().unwrap_or("");
    if !matches!(ceiling, "commons" | "community") {
        let acked = bounds
            .get("reach_elevation_acknowledged")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        if !acked {
            return Err(format!(
                "reach_ceiling='{}' requires reach_elevation_acknowledged=true",
                ceiling
            ));
        }
    }
    Ok(())
}

fn validate_acknowledges_reach_change(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "acknowledger",
        "target_epr_cid",
        "new_reach",
        "signed_at",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!(
                "acknowledges-reach-change missing required field: {field}"
            ));
        }
    }
    if payload["action"] != "acknowledges-reach-change" {
        return Err("action field must equal 'acknowledges-reach-change'".into());
    }
    let valid_reach = [
        "private",
        "self",
        "intimate",
        "trusted",
        "familiar",
        "community",
        "public",
        "commons",
    ];
    let new_reach = payload["new_reach"].as_str().unwrap_or("");
    if !valid_reach.contains(&new_reach) {
        return Err(format!("new_reach '{}' not a known reach value", new_reach));
    }
    Ok(())
}

fn validate_replicates_dwelling(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action",
        "provider_dwelling_hub_id",
        "recipient_dwelling_hub_id",
        "provider_role",
        "capacity_bytes",
        "scope_filter",
        "valid_from",
        "valid_until",
        "grace_period_days",
        "rotation_ttl_days",
        "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!(
                "replicates-dwelling missing required field: {field}"
            ));
        }
    }
    if payload["action"] != "replicates-dwelling" {
        return Err("action field must equal 'replicates-dwelling'".into());
    }

    // provider_role enum
    let provider_role = payload["provider_role"].as_str().unwrap_or("");
    if provider_role != "steward_mutual" && provider_role != "collective_steward" {
        return Err(format!("provider_role '{provider_role}' not in enum"));
    }
    if provider_role == "collective_steward" {
        let via = payload
            .get("via_collective_hub_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if via.is_empty() {
            return Err("collective_steward requires non-empty via_collective_hub_id".into());
        }
    }

    // capacity_bytes positive
    let capacity = payload["capacity_bytes"].as_u64().unwrap_or(0);
    if capacity == 0 {
        return Err("capacity_bytes must be > 0".into());
    }

    // ratio_attestation: required sub-fields + sum-to-100
    let attestation = payload
        .get("ratio_attestation")
        .and_then(|v| v.as_object())
        .ok_or("ratio_attestation must be object")?;
    for f in [
        "commons_pct",
        "dwelling_pct",
        "collective_pct",
        "free_pct",
        "effective_ratio_cid",
    ] {
        if !attestation.contains_key(f) {
            return Err(format!("ratio_attestation missing field: {f}"));
        }
    }
    let commons = attestation["commons_pct"].as_u64().unwrap_or(0);
    let dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0);
    let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
    let free = attestation["free_pct"].as_u64().unwrap_or(0);
    if commons + dwelling + collective + free != 100 {
        return Err(format!(
            "ratio_attestation pct sum {} != 100",
            commons + dwelling + collective + free
        ));
    }

    // scope_filter must be object (curation policy; can be empty)
    if !payload
        .get("scope_filter")
        .map(|v| v.is_object())
        .unwrap_or(false)
    {
        return Err("scope_filter must be object".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_delegates_compute_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "delegates-compute",
            "scope": "republish-epr",
            "provider": "agent:matthew-steward",
            "recipient": "agent:deploy-svc-matthew",
            "bounds": {
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            },
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z"
        })
    }

    #[test]
    fn delegates_compute_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: well_formed_delegates_compute_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_ok(),
            "well-formed delegates-compute payload must validate"
        );
    }

    #[test]
    fn delegates_compute_missing_fields_rejected() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: serde_json::json!({"action": "delegates-compute"}).to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "incomplete payload must fail validation"
        );
    }

    #[test]
    fn delegates_compute_wrong_action_rejected() {
        let mut payload = well_formed_delegates_compute_payload();
        payload["action"] = serde_json::json!("not-delegates-compute");
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "action field must equal action discriminator"
        );
    }

    #[test]
    fn delegates_compute_unacknowledged_reach_elevation_rejected() {
        let mut payload = well_formed_delegates_compute_payload();
        payload["bounds"]["reach_ceiling"] = serde_json::json!("public");
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(
            validate_commitment_payload(&input).is_err(),
            "reach_ceiling above commons/community requires reach_elevation_acknowledged=true"
        );
    }

    #[test]
    fn unhandled_action_rejected() {
        let input = CreateCommitmentInput {
            action: "totally-bogus-action".to_string(),
            payload_json: serde_json::json!({"action": "totally-bogus-action"}).to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn malformed_json_rejected() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: "{not valid json".to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    fn well_formed_acknowledges_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "acknowledges-reach-change",
            "acknowledger": "agent:matthew-steward",
            "target_epr_cid": "bafy-new-epr-head-cid",
            "new_reach": "community",
            "signed_at": "2026-05-29T00:00:00Z"
        })
    }

    #[test]
    fn acknowledges_reach_change_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: well_formed_acknowledges_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn acknowledges_reach_change_missing_target_epr_cid_rejected() {
        let mut payload = well_formed_acknowledges_payload();
        payload.as_object_mut().unwrap().remove("target_epr_cid");
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn acknowledges_reach_change_unknown_reach_value_rejected() {
        let mut payload = well_formed_acknowledges_payload();
        payload["new_reach"] = serde_json::json!("totally-bogus-reach");
        let input = CreateCommitmentInput {
            action: "acknowledges-reach-change".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    fn well_formed_replicates_dwelling_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "replicates-dwelling",
            "provider_dwelling_hub_id": "hub:A",
            "recipient_dwelling_hub_id": "hub:B",
            "provider_role": "steward_mutual",
            "capacity_bytes": 50_000_000_000u64,
            "scope_filter": {"epr_kinds": ["Content"]},
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z",
            "grace_period_days": 14,
            "rotation_ttl_days": 90,
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-x"
            }
        })
    }

    #[test]
    fn replicates_dwelling_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: well_formed_replicates_dwelling_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_dwelling_unknown_role_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("totally-bogus");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_collective_steward_requires_via_collective() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("collective_steward");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_collective_steward_with_via_validates() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["provider_role"] = serde_json::json!("collective_steward");
        payload["via_collective_hub_id"] = serde_json::json!("collective:church");
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_dwelling_zero_capacity_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["capacity_bytes"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_dwelling_ratio_sum_not_100_rejected() {
        let mut payload = well_formed_replicates_dwelling_payload();
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(30); // sum becomes 110
        let input = CreateCommitmentInput {
            action: "replicates-dwelling".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // replicates-commons tests (content + capacity variants)
    // =========================================================================

    fn well_formed_commons_content_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "content",
            "head_ref": "bafyhead-lamad-spa",
            "closure_rule": "transitive-1",
            "reach": "commons",
            "bounds": { "rate_per_minute": 30, "reach_ceiling": "commons" }
        })
    }

    fn well_formed_commons_capacity_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "replicates-commons",
            "variant": "capacity",
            "commons_bytes": 50_000_000_000u64,
            "bounds": { "rate_per_minute": 30, "reach_ceiling": "commons" },
            "ratio_attestation": {
                "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
                "effective_ratio_cid": "bafkrei-x"
            }
        })
    }

    #[test]
    fn replicates_commons_content_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: well_formed_commons_content_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_commons_capacity_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: well_formed_commons_capacity_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn replicates_commons_content_reach_not_commons_rejected() {
        let mut payload = well_formed_commons_content_payload();
        payload["reach"] = serde_json::json!("community");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_content_missing_head_ref_rejected() {
        let mut payload = well_formed_commons_content_payload();
        payload.as_object_mut().unwrap().remove("head_ref");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_content_carrying_ratio_attestation_rejected() {
        // The content variant carries no ratio_attestation; a stray one is rejected.
        let mut payload = well_formed_commons_content_payload();
        payload["ratio_attestation"] = serde_json::json!({
            "commons_pct": 20, "dwelling_pct": 40, "collective_pct": 25, "free_pct": 15,
            "effective_ratio_cid": "bafkrei-x"
        });
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_zero_rate_rejected() {
        // bounds.rate_per_minute == 0 is a no-op commitment and is rejected.
        let mut payload = well_formed_commons_content_payload();
        payload["bounds"]["rate_per_minute"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_zero_bytes_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["commons_bytes"] = serde_json::json!(0);
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_ratio_sum_not_100_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(30); // sum 110
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_capacity_missing_effective_ratio_cid_rejected() {
        let mut payload = well_formed_commons_capacity_payload();
        payload["ratio_attestation"]
            .as_object_mut()
            .unwrap()
            .remove("effective_ratio_cid");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn replicates_commons_unknown_variant_rejected() {
        let mut payload = well_formed_commons_content_payload();
        payload["variant"] = serde_json::json!("bogus");
        let input = CreateCommitmentInput {
            action: "replicates-commons".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // revokes-commitment tests
    // =========================================================================

    fn well_formed_revokes_payload() -> serde_json::Value {
        serde_json::json!({
            "action": "revokes-commitment",
            "target_cid": "bafyhead-target-commitment",
            "reason": "pin removed",
            "signed_at": "2026-06-10T00:00:00Z"
        })
    }

    #[test]
    fn revokes_commitment_well_formed_validates() {
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: well_formed_revokes_payload().to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_ok());
    }

    #[test]
    fn revokes_commitment_empty_target_cid_rejected() {
        let mut payload = well_formed_revokes_payload();
        payload["target_cid"] = serde_json::json!("");
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn revokes_commitment_missing_signed_at_rejected() {
        let mut payload = well_formed_revokes_payload();
        payload.as_object_mut().unwrap().remove("signed_at");
        let input = CreateCommitmentInput {
            action: "revokes-commitment".to_string(),
            payload_json: payload.to_string(),
            signed_at: "2026-06-10T00:00:00Z".to_string(),
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    // =========================================================================
    // CommitmentByState link author input (Slice-2b T11)
    // =========================================================================

    /// The state-link author input must survive a serde round-trip — the wire
    /// contract with the elohim-storage `call_create_commitment_state_link`
    /// caller. A dropped field would fail the zome call at runtime.
    #[test]
    fn create_commitment_state_link_input_serde_roundtrip() {
        let original = CreateCommitmentStateLinkInput {
            commitment_cid: "uhCEk-commitment-1".to_string(),
            state: "active".to_string(),
            event_hash: "uhCkk-graduating-event".to_string(),
            signed_at: "2026-06-11T10:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let decoded: CreateCommitmentStateLinkInput = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.commitment_cid, original.commitment_cid);
        assert_eq!(decoded.state, original.state);
        assert_eq!(decoded.event_hash, original.event_hash);
        assert_eq!(decoded.signed_at, original.signed_at);
    }
}
