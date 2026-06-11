//! Sprint 1 Task 6 — Storage-side validator for `republish-epr` EconomicEvent.
//!
//! Per-instance validator that wraps Sprint 2's substrate-wide
//! [`bounds_validator::validate`] with the republish-epr-specific schema check.
//! Invoked at the `PUT /api/v1/epr/{cid}` boundary (Sprint 1 Task 7) and by the
//! diagnostic route (Task 6). Per spec §7.3 this is the cheap-no-DHT-round-trip
//! hot path; the elohim-dna integrity validator (Task 5) is the slow-but-DHT
//! resident truth.
//!
//! ## Pattern (per `project_bounds_validator_pattern` memory)
//!
//! 1. Schema-validate the event payload (mirrors elohim-dna's
//!    `content_store::republish_epr::validate_republish_epr_event`).
//! 2. Project to [`EventForValidation`] (the substrate's evaluator-agnostic view).
//! 3. Delegate to [`bounds_validator::validate`] for the 7 substrate-wide checks.
//!
//! On `BoundsViolation`, the caller emits the appropriate FeedbackSignal
//! (rate-limit-exceeded, bad-custody, reach-escalation-pending) — see
//! `signal_weight_registry` for the weight policy.

use crate::services::bounds_validator::{self, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::{CommitmentFetcher, FetchError};
use crate::services::rate_history::RateHistory;

/// Outcomes for the republish-epr validator.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    /// Schema check failed (missing fields, bad enum values, anonymous publish).
    #[error("schema validation failed: {0}")]
    Schema(String),

    /// Substrate bounds check failed (delegate from `bounds_validator::validate`).
    /// A GENUINE policy violation (revoked / out-of-window / out-of-scope /
    /// reach-ceiling / rate-limit / not-found). The HTTP boundary maps this to
    /// 400/404 — the caller is at fault.
    #[error("bounds validation failed: {0:?}")]
    Bounds(BoundsViolation),

    /// INFRA failure reaching the commitment store (db pool checkout / query /
    /// conductor unreachable). NOT a caller fault — the HTTP boundary maps this
    /// to 503 service_unavailable so a transient pool/DB outage is not
    /// mis-reported as a bounds rejection.
    ///
    /// # Why this arm exists (the seam)
    ///
    /// `bounds_validator::validate` deliberately COLLAPSES
    /// `FetchError::ConductorUnreachable` into `ViolationKind::CommitmentNotFound`
    /// (fail-closed: a bounds-gate is never cleared on an unreachable store —
    /// see bounds_validator.rs §1 and diagnostics_bounds.rs, which report the
    /// violation in-band as a 200 view body). That collapse is correct for the
    /// diagnostics route but loses the infra/violation distinction the put_epr
    /// HTTP status needs. Rather than widen the ts-rs-anchored `ViolationKind`
    /// enum (which would ripple through the schema + codegen + 6 callers of
    /// `validate`), we recover the distinction at THIS layer — the single
    /// republish boundary that needs it — with one infra-sensitive probe of the
    /// fetcher before delegating. Smallest correct seam.
    #[error("commitment store unavailable (infra): {0}")]
    Unavailable(String),
}

/// Validate a republish-epr EconomicEvent payload at the elohim-storage HTTP
/// boundary. Runs the schema check first (cheap), then delegates to the
/// substrate bounds_validator (mock-friendly fetch + rate-history traits).
///
/// `target_epr_id` is the canonical EPR identity (typically derived from the
/// envelope's `coupling.knowledge` field). The bounds_validator uses it to
/// verify against the Commitment's `bounds.epr_scope` array.
pub async fn validate_republish_epr<F: CommitmentFetcher, R: RateHistory>(
    event_payload: &serde_json::Value,
    fetcher: &F,
    rate_history: &R,
    target_epr_id: &str,
) -> Result<(), ValidationError> {
    // 1. Schema check.
    validate_payload_schema(event_payload)?;

    // 2. Project to EventForValidation.
    let event = EventForValidation {
        action: event_payload["action"].as_str().unwrap_or("").to_string(),
        performer: event_payload["performer"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        bounded_by: event_payload["bounded_by"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        target_epr_id: target_epr_id.to_string(),
        reach: event_payload["payload"]["reach"]
            .as_str()
            .unwrap_or("")
            .to_string(),
        signed_at: event_payload["signed_at"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    // 3. Infra-class probe (the seam): one fetch of the bounding commitment.
    //    `bounds_validator::validate` will fetch again, but it collapses an
    //    unreachable store into CommitmentNotFound (fail-closed). We probe here
    //    FIRST so a genuine infra failure surfaces as `Unavailable` (→ 503)
    //    rather than being mis-reported as a bounds rejection (→ 400/404).
    //    `Ok(None)` (genuinely absent) and `Ok(Some(_))` (present) both fall
    //    through to the real bounds check below — only the infra error arm
    //    short-circuits. The probe is a cheap local diesel read (spawn_blocking)
    //    on the projection table; the fail-closed semantics are unchanged.
    if let Err(FetchError::ConductorUnreachable(msg)) = fetcher.fetch(&event.bounded_by).await {
        return Err(ValidationError::Unavailable(msg));
    }

    // 4. Substrate bounds check (delegates to Sprint 2's primitive).
    bounds_validator::validate(&event, fetcher, rate_history)
        .await
        .map(|_checks| ())
        .map_err(ValidationError::Bounds)
}

fn validate_payload_schema(payload: &serde_json::Value) -> Result<(), ValidationError> {
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
            return Err(ValidationError::Schema(format!("missing field: {field}")));
        }
    }
    if payload["action"] != "republish-epr" {
        return Err(ValidationError::Schema(
            "action must be 'republish-epr'".into(),
        ));
    }
    let bounded_by = payload["bounded_by"].as_str().unwrap_or("");
    if bounded_by.is_empty() {
        return Err(ValidationError::Schema(
            "bounded_by must be non-empty (anonymous publish forbidden)".into(),
        ));
    }
    let inner = payload
        .get("payload")
        .and_then(|p| p.as_object())
        .ok_or_else(|| ValidationError::Schema("payload must be object".into()))?;
    for field in ["blob_cid", "epr_kind", "reach"] {
        if !inner.contains_key(field) {
            return Err(ValidationError::Schema(format!("payload missing: {field}")));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::commitment_fetcher::{CommitmentRecord, MockCommitmentFetcher};
    use crate::services::rate_history::MockRateHistory;

    fn deploy_svc_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "comm-abc".into(),
            action: "delegates-compute".into(),
            scope: "republish-epr".into(),
            provider: "agent:matthew-steward".into(),
            recipient: "agent:deploy-svc".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    fn well_formed_event() -> serde_json::Value {
        serde_json::json!({
            "action": "republish-epr",
            "performer": "agent:deploy-svc",
            "bounded_by": "comm-abc",
            "target": "epr-head-new",
            "payload": {
                "blob_cid": "bafkrei-blob-cid",
                "epr_kind": "Content",
                "reach": "commons",
                "bundle_path": "lamad-spa"
            },
            "signed_at": "2026-05-28T12:00:00Z"
        })
    }

    #[tokio::test]
    async fn valid_republish_event_passes() {
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("comm-abc", deploy_svc_commitment());
        let rate = MockRateHistory::new();
        let result =
            validate_republish_epr(&well_formed_event(), &fetcher, &rate, "epr:lamad-spa").await;
        assert!(result.is_ok(), "well-formed republish-epr should pass");
    }

    #[tokio::test]
    async fn schema_violation_rejected_before_bounds_check() {
        let mut event = well_formed_event();
        event.as_object_mut().unwrap().remove("bounded_by");
        // Empty fetcher: if bounds were checked, this would fail at CommitmentNotFound.
        // Instead the schema check fires first.
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_republish_epr(&event, &fetcher, &rate, "epr:lamad-spa").await;
        assert!(matches!(result, Err(ValidationError::Schema(_))));
    }

    #[tokio::test]
    async fn bounds_violation_emitted_when_commitment_stale() {
        // Make the commitment rotation-stale: valid_from way too long ago.
        // 2026-01-01 to 2026-05-28 is 147 days, exceeding the 90-day TTL.
        let mut c = deploy_svc_commitment();
        c.valid_from = "2026-01-01T00:00:00Z".into();
        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed("comm-abc", c);
        let rate = MockRateHistory::new();
        let result =
            validate_republish_epr(&well_formed_event(), &fetcher, &rate, "epr:lamad-spa").await;
        assert!(matches!(result, Err(ValidationError::Bounds(_))));
    }

    #[tokio::test]
    async fn anonymous_publish_rejected_at_schema_layer() {
        let mut event = well_formed_event();
        event["bounded_by"] = serde_json::json!("");
        let fetcher = MockCommitmentFetcher::new();
        let rate = MockRateHistory::new();
        let result = validate_republish_epr(&event, &fetcher, &rate, "epr:lamad-spa").await;
        match result {
            Err(ValidationError::Schema(msg)) => {
                assert!(
                    msg.contains("bounded_by"),
                    "must name bounded_by in error: {msg}"
                );
            }
            other => panic!("expected ValidationError::Schema, got {:?}", other),
        }
    }
}
