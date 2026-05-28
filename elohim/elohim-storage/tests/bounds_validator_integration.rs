//! Adversarial integration tests for bounds_validator.
//!
//! Tests attack surface that unit-test mocks don't catch — race conditions
//! between fetch+validate, malformed CommitmentRecords, sliding-window edges,
//! and the trust assumption that fetcher returns accurate revoked state.
//!
//! See `genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md`.
//!
//! # Plan spec correction — reach ceiling in adversarial_silent_reach_escalation_blocked
//!
//! The plan specified event.reach="public" failing against reach_ceiling="commons".
//! However reach_rank("public") = 6 and reach_rank("commons") = 7, so 6 <= 7 → PASS.
//! The test is corrected to use reach_ceiling="community" (rank 5) so that
//! event.reach="public" (rank 6) genuinely exceeds the ceiling.  The validator
//! is NOT modified — this is a spec bug, not an implementation bug.

use elohim_storage::services::bounds_validator::{validate, BoundsViolation, EventForValidation};
use elohim_storage::services::commitment_fetcher::{CommitmentRecord, MockCommitmentFetcher};
use elohim_storage::services::rate_history::MockRateHistory;
use elohim_views::bounds::ViolationKind;

fn deploy_svc_commitment(revoked: bool) -> CommitmentRecord {
    CommitmentRecord {
        cid: "comm-deploy-svc".into(),
        action: "delegates-compute".into(),
        scope: "republish-epr".into(),
        provider: "agent:matthew-steward".into(),
        recipient: "agent:deploy-svc-matthew".into(),
        bounds: serde_json::json!({
            "epr_scope": ["epr:lamad-spa", "epr:elohim-host-landing"],
            "reach_ceiling": "commons",
            "rate_per_hour": 30,
            "rotation_ttl_days": 90
        }),
        valid_from: "2026-05-01T00:00:00Z".into(),
        valid_until: "2026-08-01T00:00:00Z".into(),
        revoked_at: if revoked {
            Some("2026-05-15T00:00:00Z".into())
        } else {
            None
        },
    }
}

fn event_against(commitment_cid: &str, action: &str, target: &str, reach: &str) -> EventForValidation {
    EventForValidation {
        action: action.into(),
        performer: "agent:deploy-svc-matthew".into(),
        bounded_by: commitment_cid.into(),
        target_epr_id: target.into(),
        reach: reach.into(),
        signed_at: "2026-05-28T12:00:00Z".into(),
    }
}

/// A revoked commitment must be rejected immediately regardless of all other
/// fields being valid.  Models the race where a commitment is revoked between
/// the time an action is initiated and when the validator runs.
#[tokio::test]
async fn adversarial_revocation_race_blocks_immediately() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(true));
    let rate = MockRateHistory::new();
    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        matches!(result, Err(BoundsViolation { kind: ViolationKind::CommitmentRevoked, .. })),
        "revoked commitment must be rejected; got: {:?}",
        result
    );
}

/// A forged event that points bounded_by at a commitment whose scope does not
/// cover the attempted action must be rejected at the scope check.
#[tokio::test]
async fn adversarial_forged_bounded_by_pointing_to_unrelated_scope() {
    let mut c = deploy_svc_commitment(false);
    c.scope = "serve-url-projection".into();
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", c);
    let rate = MockRateHistory::new();
    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        matches!(result, Err(BoundsViolation { kind: ViolationKind::ScopeNotIncluded, .. })),
        "scope mismatch must be rejected; got: {:?}",
        result
    );
}

/// A reach value within the ceiling must pass; a reach value above the ceiling
/// must be blocked.  Models silent reach escalation — an agent attempting to
/// publish wider than their commitment authorises.
///
/// # Spec correction
/// The plan specified reach_ceiling="commons" with event.reach="public" expecting
/// failure. reach_rank("public")=6 <= reach_rank("commons")=7 → that would PASS.
/// Test corrected: commitment uses reach_ceiling="community" (rank 5) so that
/// "private" (rank 0) passes and "public" (rank 6) fails.  Validator unchanged.
#[tokio::test]
async fn adversarial_silent_reach_escalation_blocked() {
    let mut c_lower = deploy_svc_commitment(false);
    c_lower.bounds = serde_json::json!({
        "epr_scope": ["epr:lamad-spa", "epr:elohim-host-landing"],
        "reach_ceiling": "community", // rank 5
        "rate_per_hour": 30,
        "rotation_ttl_days": 90
    });

    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", c_lower);
    let rate = MockRateHistory::new();

    // Reach <= ceiling must PASS: private(0) <= community(5)
    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "private");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        result.is_ok(),
        "lower reach within ceiling must pass; got: {:?}",
        result
    );

    // Reach > ceiling must FAIL: public(6) > community(5)
    let event_up = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "public");
    let result_up = validate(&event_up, &fetcher, &rate).await;
    assert!(
        matches!(result_up, Err(BoundsViolation { kind: ViolationKind::ReachCeilingExceeded, .. })),
        "reach above ceiling must be blocked; got: {:?}",
        result_up
    );
}

/// The sliding-window rate limit is enforced at the boundary: 29 events in the
/// window must pass; exactly 30 (== rate_per_hour limit) must fail.
///
/// The validator uses `>=` for the comparison, so the limit is exclusive of the
/// ceiling: count < rate_per_hour passes, count >= rate_per_hour fails.
#[tokio::test]
async fn adversarial_rate_limit_exact_boundary() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(false));
    let rate = MockRateHistory::new();
    let event = event_against("comm-deploy-svc", "republish-epr", "epr:lamad-spa", "commons");

    // 29 events in window → strictly less than limit (30) → must pass
    rate.seed("comm-deploy-svc", "2026-05-28T12:00:00Z", 29);
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        result.is_ok(),
        "29 events in window must pass; got: {:?}",
        result
    );

    // 30 events in window → equals limit (30) → must fail (>= check)
    rate.seed("comm-deploy-svc", "2026-05-28T12:00:00Z", 30);
    let result_at_limit = validate(&event, &fetcher, &rate).await;
    assert!(
        matches!(result_at_limit, Err(BoundsViolation { kind: ViolationKind::RateLimitExceeded, .. })),
        "30 events (at limit) must fail; got: {:?}",
        result_at_limit
    );
}

/// An event targeting an EPR not listed in the commitment's epr_scope must be
/// rejected even if the action, reach, and rate are all valid.
#[tokio::test]
async fn adversarial_out_of_epr_scope_rejected() {
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", deploy_svc_commitment(false));
    let rate = MockRateHistory::new();
    let event = event_against("comm-deploy-svc", "republish-epr", "epr:not-in-scope", "commons");
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        matches!(result, Err(BoundsViolation { kind: ViolationKind::ScopeNotIncluded, .. })),
        "EPR not in scope must be rejected; got: {:?}",
        result
    );
}

/// A commitment with epr_scope = ["*"] must allow any EPR target identifier,
/// including ones not known at commitment creation time.
///
/// This verifies the `s == "*"` wildcard branch in the `scope_matches` closure
/// in bounds_validator.rs.
#[tokio::test]
async fn adversarial_wildcard_epr_scope_allows_unknown_target() {
    let mut c = deploy_svc_commitment(false);
    c.bounds["epr_scope"] = serde_json::json!(["*"]);
    let fetcher = MockCommitmentFetcher::new();
    fetcher.seed("comm-deploy-svc", c);
    let rate = MockRateHistory::new();
    let event = event_against(
        "comm-deploy-svc",
        "republish-epr",
        "epr:any-future-bundle",
        "commons",
    );
    let result = validate(&event, &fetcher, &rate).await;
    assert!(
        result.is_ok(),
        "wildcard epr_scope must allow unknown target; got: {:?}",
        result
    );
}
