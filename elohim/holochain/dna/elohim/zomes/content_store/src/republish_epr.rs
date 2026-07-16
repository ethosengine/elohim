//! republish-epr EconomicEvent validator — Z.D substrate-correct deploy.
//!
//! See genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md §2.
//! Schema: elohim/sdk/schemas/v1/economic-events/republish-epr.schema.json.
//!
//! Per gospel memory `project_rea_compute_commitment_primitive`: every
//! substrate-correct EprHead publish carries a `republish-epr` EconomicEvent
//! whose `bounded_by` field references the authorizing delegates-compute
//! Commitment. The storage-side bounds_validator walks this back-reference
//! to verify standing, scope, reach ceiling, rate limit, and key rotation.
//!
//! `anonymous publish forbidden` (canon §6 rule 2): `bounded_by` is REQUIRED;
//! the validator rejects events without it. First-publish of a new EPR is
//! still substrate-correct — the steward authors a `delegates-compute`
//! Commitment for the new EPR's identity and emits republish-epr against it.

use serde_json::Value;

/// Validate a republish-epr EconomicEvent payload (schema-shape JSON).
///
/// Returns `Ok(())` on success; `Err(reason)` on any schema violation.
pub fn validate_republish_epr_event(payload: &Value) -> Result<(), String> {
    let required = [
        "action",
        "performer",
        "bounded_by",
        "target",
        "payload",
        "signed_at",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(format!("republish-epr missing required field: {field}"));
        }
    }
    if payload["action"] != "republish-epr" {
        return Err("action must equal 'republish-epr'".into());
    }

    // bounded_by must be non-empty (anonymous publish forbidden — canon §6 rule 2).
    let bounded_by = payload["bounded_by"].as_str().unwrap_or("");
    if bounded_by.is_empty() {
        return Err("bounded_by must be non-empty (anonymous publish forbidden)".into());
    }

    let inner = payload
        .get("payload")
        .and_then(|p| p.as_object())
        .ok_or_else(|| "payload must be object".to_string())?;
    for field in ["blob_cid", "epr_kind", "reach"] {
        if !inner.contains_key(field) {
            return Err(format!("payload missing field: {field}"));
        }
    }

    let epr_kind = inner["epr_kind"].as_str().unwrap_or("");
    let valid_kinds = [
        "Content",
        "Agent",
        "Manifest",
        "Claim",
        "Observation",
        "EconomicEvent",
        "Commitment",
        "Attestation",
        "Delegation",
        "FeedbackSignal",
    ];
    if !valid_kinds.contains(&epr_kind) {
        return Err(format!("payload.epr_kind '{}' not in enum", epr_kind));
    }

    let reach = inner["reach"].as_str().unwrap_or("");
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
    if !valid_reach.contains(&reach) {
        return Err(format!("payload.reach '{}' not in enum", reach));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn well_formed_payload() -> Value {
        serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc-matthew",
            "bounded_by": "comm-cid-abc",
            "target": "epr-head-cid-new",
            "supersedes": "epr-head-cid-prev",
            "payload": {
                "blob_cid": "bafkreiabc",
                "epr_kind": "Content",
                "reach": "commons",
                "bundle_path": "lamad-spa"
            },
            "signed_at": "2026-05-28T12:00:00Z"
        })
    }

    #[test]
    fn validate_republish_epr_payload_well_formed() {
        assert!(validate_republish_epr_event(&well_formed_payload()).is_ok());
    }

    #[test]
    fn validate_republish_epr_rejects_missing_bounded_by() {
        let mut payload = well_formed_payload();
        payload.as_object_mut().unwrap().remove("bounded_by");
        assert!(validate_republish_epr_event(&payload).is_err());
    }

    #[test]
    fn validate_republish_epr_rejects_empty_bounded_by() {
        let mut payload = well_formed_payload();
        payload["bounded_by"] = serde_json::json!("");
        let err = validate_republish_epr_event(&payload).expect_err("empty bounded_by must reject");
        assert!(err.contains("anonymous publish forbidden"));
    }

    #[test]
    fn validate_republish_epr_rejects_unknown_epr_kind() {
        let mut payload = well_formed_payload();
        payload["payload"]["epr_kind"] = serde_json::json!("NotARealKind");
        assert!(validate_republish_epr_event(&payload).is_err());
    }

    #[test]
    fn validate_republish_epr_rejects_unknown_reach() {
        let mut payload = well_formed_payload();
        payload["payload"]["reach"] = serde_json::json!("not-a-real-reach");
        assert!(validate_republish_epr_event(&payload).is_err());
    }

    #[test]
    fn validate_republish_epr_rejects_wrong_action() {
        let mut payload = well_formed_payload();
        payload["action"] = serde_json::json!("something-else");
        assert!(validate_republish_epr_event(&payload).is_err());
    }
}
