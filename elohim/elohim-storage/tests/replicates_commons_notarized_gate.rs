//! Integration seatbelt — replicates-commons notarized-provenance gate (Slice 2b T1).
//!
//! The sweettest (`replicates_commons_round_trip_test`) proves the DNA notarizes
//! the commitment and `get_commitment` reads it back. THIS test proves the
//! storage half: the projection parses a replicates-commons content-variant
//! payload into a row with a NON-NULL dht_anchor_hash, the
//! ProjectionCommitmentFetcher returns it (the same path ConductorCommitmentFetcher
//! feeds), and the bounds_validator clears an event bound to it. Together they
//! are the two-leg gate for the provide loop. Spec §6.5.

use elohim_storage::db::mishpat_commitments;
use elohim_storage::mishpat_projection::{parse_commitment_payload, CommitmentProjection};
use elohim_storage::services::bounds_validator::{self, EventForValidation};
use elohim_storage::services::commitment_fetcher::{
    CommitmentFetcher, ProjectionCommitmentFetcher,
};
use elohim_storage::services::rate_history::MockRateHistory;
use elohim_storage::test_util::test_pool;
use elohim_views::bounds::ViolationKind;

/// The replicates-commons content-variant payload the coordinator notarizes —
/// reach == commons, bounds with rate_per_minute + reach_ceiling, no ratio_attestation.
/// The validator's `valid_from`/`valid_until` window must bracket the event time,
/// so we include them (the bounds 7-check reads them from the same payload).
fn replicates_commons_payload() -> String {
    serde_json::json!({
        "action": "replicates-commons",
        "variant": "content",
        "head_ref": "epr:lamad-spa-head-cid",
        "reach": "commons",
        "bounds": { "rate_per_minute": 60, "reach_ceiling": "commons" },
        "scope": "republish-epr",
        "provider": "agent:provider-x",
        "recipient": "epr:lamad-spa-head-cid",
        "valid_from": "2026-06-01T00:00:00Z",
        "valid_until": "2026-09-01T00:00:00Z"
    })
    .to_string()
}

#[tokio::test]
async fn notarized_replicates_commons_clears_bounds_via_projection_fetcher() {
    // 1. Parse the projection row from the DNA wire shape. The action_hash arg
    //    is the post-commit `dht_anchor_hash` — exactly what the sweettest's
    //    get_commitment returns as `action_hash`.
    let entry_hash = "uhCEk-commons-entry-1";
    let action_hash = "uhCkk-commons-action-1";
    let row = match parse_commitment_payload(
        "replicates-commons",
        &replicates_commons_payload(),
        entry_hash,
        action_hash,
    )
    .expect("replicates-commons payload must parse into a NewMishpatCommitment")
    {
        CommitmentProjection::Upsert(r) => r,
        other => panic!("expected Upsert, got {other:?}"),
    };

    assert_eq!(row.cid, entry_hash);
    assert_eq!(
        row.dht_anchor_hash.as_deref(),
        Some(action_hash),
        "projection must write the action_hash as a NON-NULL dht_anchor_hash"
    );

    // 2. Insert it and read it back through the ProjectionCommitmentFetcher
    //    (the production fetcher; ConductorCommitmentFetcher feeds the same row).
    let pool = test_pool();
    {
        let mut conn = pool.get().expect("pool conn");
        mishpat_commitments::upsert_with_anchor(&mut conn, row).expect("upsert");
    }
    let fetcher = ProjectionCommitmentFetcher::new(pool);
    let record = fetcher
        .fetch(entry_hash)
        .await
        .expect("fetch must not error")
        .expect("notarized row (non-NULL anchor) must be present");
    assert_eq!(record.action, "replicates-commons");
    assert!(record.revoked_at.is_none());

    // 3. The bounds_validator clears an event bound to this commitment.
    let event = EventForValidation {
        action: "replicates-commons".to_string(),
        performer: "agent:provider-x".to_string(),
        bounded_by: entry_hash.to_string(),
        target_epr_id: "epr:lamad-spa-head-cid".to_string(),
        reach: "commons".to_string(),
        signed_at: "2026-06-15T12:00:00Z".to_string(),
    };
    let rate = MockRateHistory::new();
    let result = bounds_validator::validate(&event, &fetcher, &rate).await;
    assert!(
        result.is_ok(),
        "a notarized, in-bounds replicates-commons commitment must clear the bounds gate; got {:?}",
        result.err().map(|v| v.kind)
    );
}

/// Companion guard: a row whose anchor is NULL (un-notarized / storage-only)
/// must NOT clear the gate — the fetcher fails closed and the validator maps it
/// to CommitmentNotFound. Proves the gate is real, not vacuously green.
#[tokio::test]
async fn unnotarized_replicates_commons_is_refused() {
    let pool = test_pool();
    {
        let mut conn = pool.get().expect("pool conn");
        // Hand-craft a NULL-anchor row (what a storage-only insert would look like).
        let row = match parse_commitment_payload(
            "replicates-commons",
            &replicates_commons_payload(),
            "uhCEk-unanchored",
            "ignored",
        )
        .expect("parse")
        {
            CommitmentProjection::Upsert(mut r) => {
                r.dht_anchor_hash = None;
                r
            }
            other => panic!("expected Upsert, got {other:?}"),
        };
        mishpat_commitments::upsert_with_anchor(&mut conn, row).expect("upsert");
    }
    let fetcher = ProjectionCommitmentFetcher::new(pool);
    let event = EventForValidation {
        action: "replicates-commons".to_string(),
        performer: "agent:provider-x".to_string(),
        bounded_by: "uhCEk-unanchored".to_string(),
        target_epr_id: "epr:lamad-spa-head-cid".to_string(),
        reach: "commons".to_string(),
        signed_at: "2026-06-15T12:00:00Z".to_string(),
    };
    let err = bounds_validator::validate(&event, &fetcher, &MockRateHistory::new())
        .await
        .expect_err("un-notarized (NULL anchor) must NOT clear the bounds gate");
    assert_eq!(
        err.kind,
        ViolationKind::CommitmentNotFound,
        "fail-closed: NULL dht_anchor_hash maps to CommitmentNotFound"
    );
}
