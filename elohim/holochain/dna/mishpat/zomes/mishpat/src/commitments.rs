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

    let signed_at = sys_time()?.as_seconds_and_nanos().0.to_string();
    let entry = Commitment {
        action: input.action.clone(),
        payload_json: input.payload_json.clone(),
        signed_at,
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
        other => Err(format!(
            "commitments::validate_commitment_payload unhandled action: {other}"
        )),
    }
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
        };
        assert!(validate_commitment_payload(&input).is_err());
    }

    #[test]
    fn malformed_json_rejected() {
        let input = CreateCommitmentInput {
            action: "delegates-compute".to_string(),
            payload_json: "{not valid json".to_string(),
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
        };
        assert!(validate_commitment_payload(&input).is_err());
    }
}
