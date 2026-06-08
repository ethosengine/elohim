//! Mishpat Commitment coordinator — Z.D substrate-correct deploy primitive.
//!
//! Authors the `delegates-compute` and `acknowledges-reach-change` Commitment
//! actions per genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md.
//!
//! See `bounds-validator-pattern` memory: per-instance validators consume the
//! Commitment via `services::commitment_fetcher::CommitmentFetcher` and
//! `services::bounds_validator::validate` in elohim-storage.

use hdk::prelude::*;
use mishpat_integrity::Commitment;

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
    validate_commitment_payload(&input)
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e)))?;

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
        other => Err(format!(
            "commitments::validate_commitment_payload unhandled action: {other}"
        )),
    }
}

/// Minimal gate-passing validator for the `replicates-commons` action so the
/// Slice-2b T1 conductor round-trip notarizes the content-variant payload. The
/// variant-specific checks (capacity donut sum-to-100, closure_rule, etc.) land
/// in their dedicated Slice-2b task; this mirrors `validate_replicates_dwelling`'s
/// hand-rolled style.
fn validate_replicates_commons(payload: &serde_json::Value) -> Result<(), String> {
    if payload["action"] != "replicates-commons" {
        return Err("action field must equal 'replicates-commons'".into());
    }
    // reach must be commons (the variant-specific checks land in a later task;
    // this is the minimal gate-passing guard so the round-trip notarizes).
    if payload.get("reach").and_then(|v| v.as_str()) != Some("commons") {
        return Err("replicates-commons requires reach == 'commons'".into());
    }
    if payload.get("variant").and_then(|v| v.as_str()).is_none() {
        return Err("replicates-commons requires a 'variant' discriminator".into());
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
    for field in ["epr_scope", "reach_ceiling", "rate_per_hour", "rotation_ttl_days"] {
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
    let required = ["action", "acknowledger", "target_epr_cid", "new_reach", "signed_at"];
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
        "private", "self", "intimate", "trusted", "familiar", "community", "public", "commons",
    ];
    let new_reach = payload["new_reach"].as_str().unwrap_or("");
    if !valid_reach.contains(&new_reach) {
        return Err(format!(
            "new_reach '{}' not a known reach value",
            new_reach
        ));
    }
    Ok(())
}

fn validate_replicates_dwelling(payload: &serde_json::Value) -> Result<(), String> {
    let required = [
        "action", "provider_dwelling_hub_id", "recipient_dwelling_hub_id",
        "provider_role", "capacity_bytes", "scope_filter",
        "valid_from", "valid_until", "grace_period_days",
        "rotation_ttl_days", "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("replicates-dwelling missing required field: {field}"));
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
        let via = payload.get("via_collective_hub_id").and_then(|v| v.as_str()).unwrap_or("");
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
    let attestation = payload.get("ratio_attestation").and_then(|v| v.as_object())
        .ok_or("ratio_attestation must be object")?;
    for f in ["commons_pct", "dwelling_pct", "collective_pct", "free_pct", "effective_ratio_cid"] {
        if !attestation.contains_key(f) {
            return Err(format!("ratio_attestation missing field: {f}"));
        }
    }
    let commons    = attestation["commons_pct"].as_u64().unwrap_or(0);
    let dwelling   = attestation["dwelling_pct"].as_u64().unwrap_or(0);
    let collective = attestation["collective_pct"].as_u64().unwrap_or(0);
    let free       = attestation["free_pct"].as_u64().unwrap_or(0);
    if commons + dwelling + collective + free != 100 {
        return Err(format!(
            "ratio_attestation pct sum {} != 100",
            commons + dwelling + collective + free
        ));
    }

    // scope_filter must be object (curation policy; can be empty)
    if !payload.get("scope_filter").map(|v| v.is_object()).unwrap_or(false) {
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
}
