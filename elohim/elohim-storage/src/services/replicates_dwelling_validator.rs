//! Sprint 3 — per-instance validator for `replicates-dwelling` events.
//!
//! Three-stage validation:
//!   1. Schema check (cheap, structural).
//!   2. Donut check (constitutional_ratio_registry vs ratio_attestation + proposed pledge).
//!   3. Substrate bounds check (delegate to bounds_validator::validate for the 7 substrate-wide checks).
//!
//! Pattern: per project_bounds_validator_pattern memory. Sprint 3 is the
//! FIRST concrete instance proving the pattern; Sprints N+1+ (collective tier,
//! commons tier, doorway projection compute, distributed workloads) mirror
//! this shape for their per-instance validators.

use crate::services::bounds_validator::{self, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::CommitmentFetcher;
use crate::services::constitutional_ratio_registry;
use crate::services::rate_history::RateHistory;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("schema validation failed: {0}")]
    Schema(String),
    #[error("constitutional ratio breach: {0}")]
    ConstitutionalRatio(String),
    #[error("collective_steward mode pending follow-up sprint — not yet supported")]
    CollectiveStewardModeNotYetSupported,
    #[error("bounds validation failed: {0:?}")]
    Bounds(BoundsViolation),
}

/// Provider's existing per-tier pledged bytes summary (input to donut check).
/// Computed by peer_capacity_service before calling this validator.
#[derive(Debug, Clone, Default)]
pub struct ProviderPledgedState {
    pub total_raw_bytes:        u64,
    pub pledged_dwelling_bytes: u64,
    pub pledged_collective_bytes: u64,
    pub pledged_commons_bytes:    u64,
}

pub async fn validate_replicates_dwelling<F: CommitmentFetcher, R: RateHistory>(
    event_payload: &serde_json::Value,
    fetcher: &F,
    rate_history: &R,
    provider_state: &ProviderPledgedState,
) -> Result<(), ValidationError> {
    // 1. Schema check
    validate_payload_schema(event_payload)?;

    // 2. Collective_steward mode is schema-reserved this sprint; reject explicitly.
    let provider_role = event_payload["provider_role"].as_str().unwrap_or("");
    if provider_role == "collective_steward" {
        return Err(ValidationError::CollectiveStewardModeNotYetSupported);
    }

    // 3. Donut check (ceiling enforced via pledges; floor via ratio_attestation declaration this sprint)
    donut_check(event_payload, provider_state)?;

    // 4. Substrate bounds check via Sprint 2 primitive
    let event = project_to_event_for_validation(event_payload);
    bounds_validator::validate(&event, fetcher, rate_history)
        .await
        .map(|_checks| ())
        .map_err(ValidationError::Bounds)
}

fn validate_payload_schema(payload: &serde_json::Value) -> Result<(), ValidationError> {
    let required = [
        "action", "provider_dwelling_hub_id", "recipient_dwelling_hub_id",
        "provider_role", "capacity_bytes", "scope_filter",
        "valid_from", "valid_until", "grace_period_days",
        "rotation_ttl_days", "ratio_attestation",
    ];
    for field in required {
        if payload.get(field).is_none() {
            return Err(ValidationError::Schema(format!("missing field: {field}")));
        }
    }
    if payload["action"] != "replicates-dwelling" {
        return Err(ValidationError::Schema("action must be 'replicates-dwelling'".into()));
    }
    let provider_role = payload["provider_role"].as_str().unwrap_or("");
    if provider_role != "steward_mutual" && provider_role != "collective_steward" {
        return Err(ValidationError::Schema(format!("unknown provider_role: {provider_role}")));
    }
    if provider_role == "collective_steward" {
        let via = payload.get("via_collective_hub_id").and_then(|v| v.as_str()).unwrap_or("");
        if via.is_empty() {
            return Err(ValidationError::Schema("collective_steward requires via_collective_hub_id".into()));
        }
    }
    Ok(())
}

fn donut_check(
    payload: &serde_json::Value,
    state: &ProviderPledgedState,
) -> Result<(), ValidationError> {
    let provenance = constitutional_ratio_registry::effective_ratios();
    let effective = provenance.ratios;
    let manifest_cid = provenance.manifest_cid;

    let capacity_bytes = payload["capacity_bytes"].as_u64().unwrap_or(0);
    let attestation = payload.get("ratio_attestation")
        .ok_or_else(|| ValidationError::ConstitutionalRatio("missing ratio_attestation".into()))?;
    let attested_commons = attestation["commons_pct"].as_u64().unwrap_or(0) as u8;
    let attested_dwelling = attestation["dwelling_pct"].as_u64().unwrap_or(0) as u8;
    let attested_collective = attestation["collective_pct"].as_u64().unwrap_or(0) as u8;
    let attested_free = attestation["free_pct"].as_u64().unwrap_or(0) as u8;
    let attested_cid = attestation["effective_ratio_cid"].as_str().unwrap_or("");

    // (a) Sum-to-100
    let sum = attested_commons as u16 + attested_dwelling as u16 + attested_collective as u16 + attested_free as u16;
    if sum != 100 {
        return Err(ValidationError::ConstitutionalRatio(format!("ratio_attestation pct sum {sum} != 100")));
    }

    // (b) Attested values must match clamped effective_ratios (declaration matches manifest)
    if attested_commons != effective.commons_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested commons_pct {attested_commons} != effective {} (manifest {})",
            effective.commons_pct, manifest_cid
        )));
    }
    if attested_dwelling != effective.dwelling_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested dwelling_pct {attested_dwelling} != effective {}",
            effective.dwelling_pct
        )));
    }
    if attested_collective != effective.collective_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested collective_pct {attested_collective} != effective {}",
            effective.collective_pct
        )));
    }
    if attested_free != effective.free_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested free_pct {attested_free} != effective {}",
            effective.free_pct
        )));
    }

    // (c) Ceiling check: new dwelling pledge cannot push dwelling-tier above effective ceiling
    let total = state.total_raw_bytes.max(1);
    let new_dwelling = state.pledged_dwelling_bytes + capacity_bytes;
    let new_dwelling_pct = ((new_dwelling as u128 * 100) / total as u128) as u64;
    if new_dwelling_pct as u8 > effective.dwelling_pct {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "adding {capacity_bytes} would push dwelling_pct to {new_dwelling_pct}, above effective ceiling {}",
            effective.dwelling_pct
        )));
    }

    // (d) Floor check via declaration (Sprint 3 design choice; follow-up sprint adds backing-pledge requirement)
    if attested_commons < crate::services::constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "attested commons_pct {attested_commons} below DNA floor {}",
            crate::services::constitutional_ratio_registry::COMMONS_MIN_FLOOR_PCT
        )));
    }

    // (e) Provenance match
    if attested_cid != manifest_cid {
        return Err(ValidationError::ConstitutionalRatio(format!(
            "ratio_attestation effective_ratio_cid {attested_cid} != current manifest {manifest_cid}"
        )));
    }

    Ok(())
}

fn project_to_event_for_validation(payload: &serde_json::Value) -> EventForValidation {
    EventForValidation {
        action: payload["action"].as_str().unwrap_or("").to_string(),
        performer: payload["provider_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        bounded_by: payload["recipient_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        target_epr_id: payload["recipient_dwelling_hub_id"].as_str().unwrap_or("").to_string(),
        reach: "household".into(),
        signed_at: payload.get("signed_at").and_then(|v| v.as_str()).unwrap_or("").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::MockCommitmentFetcher;
    use crate::services::rate_history::MockRateHistory;
    use elohim_views::bounds::ViolationKind;

    fn well_formed_payload(provider_role: &str, capacity_bytes: u64) -> serde_json::Value {
        let provenance = constitutional_ratio_registry::effective_ratios();
        let r = provenance.ratios;
        serde_json::json!({
            "action": "replicates-dwelling",
            "provider_dwelling_hub_id": "hub:A",
            "recipient_dwelling_hub_id": "hub:B",
            "provider_role": provider_role,
            "capacity_bytes": capacity_bytes,
            "scope_filter": {"epr_kinds": ["Content"]},
            "valid_from": "2026-05-28T00:00:00Z",
            "valid_until": "2026-08-26T00:00:00Z",
            "grace_period_days": 14,
            "rotation_ttl_days": 90,
            "ratio_attestation": {
                "commons_pct": r.commons_pct,
                "dwelling_pct": r.dwelling_pct,
                "collective_pct": r.collective_pct,
                "free_pct": r.free_pct,
                "effective_ratio_cid": provenance.manifest_cid
            },
            "signed_at": "2026-05-28T12:00:00Z"
        })
    }

    fn fresh_state() -> ProviderPledgedState {
        ProviderPledgedState {
            total_raw_bytes: 100_000_000_000,
            pledged_dwelling_bytes: 0,
            pledged_collective_bytes: 0,
            pledged_commons_bytes: 0,
        }
    }

    #[tokio::test]
    async fn valid_steward_mutual_passes_donut_then_bounds() {
        // bounds_validator will fail at CommitmentNotFound (fetcher empty),
        // which proves we passed schema + donut checks before reaching bounds.
        let payload = well_formed_payload("steward_mutual", 30_000_000_000);
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        match result {
            Err(ValidationError::Bounds(b)) => assert_eq!(b.kind, ViolationKind::CommitmentNotFound),
            other => panic!("expected Bounds(CommitmentNotFound), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn collective_steward_rejected_explicitly() {
        let mut payload = well_formed_payload("collective_steward", 30_000_000_000);
        payload["via_collective_hub_id"] = serde_json::json!("collective:church");
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::CollectiveStewardModeNotYetSupported)));
    }

    #[tokio::test]
    async fn ratio_attestation_below_floor_rejected() {
        let mut payload = well_formed_payload("steward_mutual", 30_000_000_000);
        payload["ratio_attestation"]["commons_pct"] = serde_json::json!(5);  // below 10 floor
        payload["ratio_attestation"]["free_pct"] = serde_json::json!(30);    // make sum=100
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::ConstitutionalRatio(_))));
    }

    #[tokio::test]
    async fn dwelling_ceiling_breach_rejected() {
        // Provider already pledged 70GB dwelling on 100GB device; effective dwelling ceiling=40%; new 30GB pushes over.
        let mut state = fresh_state();
        state.pledged_dwelling_bytes = 70_000_000_000;
        let payload = well_formed_payload("steward_mutual", 30_000_000_000);
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &state).await;
        assert!(matches!(result, Err(ValidationError::ConstitutionalRatio(_))));
    }

    #[tokio::test]
    async fn schema_missing_field_rejected() {
        let mut payload = well_formed_payload("steward_mutual", 30_000_000_000);
        payload.as_object_mut().unwrap().remove("capacity_bytes");
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_replicates_dwelling(&payload, &fetcher, &rate, &fresh_state()).await;
        assert!(matches!(result, Err(ValidationError::Schema(_))));
    }
}
