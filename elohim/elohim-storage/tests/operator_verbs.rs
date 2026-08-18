//! Integration tests for the commitment-gated operator reconcile verb
//! (habit `operator-runtime-surface`, slice 1).
//!
//! Mirrors `tests/operation_authorization.rs`: a REAL seeded pool (via
//! `perform_seed`, which synthesises a non-NULL `dht_anchor_hash`), the real
//! `authorize_operation` chain, and a real capacity-1 kick channel — no mocks.
//!
//! The scenarios pinned here are the storage-side halves of the a2o red
//! `@concern:operator-runtime-surface`
//! (genesis/a2o/features/dataplane/operator-commitment-gated-verbs.feature):
//!   1. holder → accepted, attestation names the commitment CID, kick queued
//!   2. no grant → refused, no kick
//!   3. revoked grant → refused immediately, no kick
//!   4. no verified performer → refused, no kick
//!   5. authorized but loop unavailable → honest LoopUnavailable (503), not a
//!      silent 200

use elohim_storage::api::operator_verbs::{
    perform_reconcile_verb, ReconcileVerbOutcome, OPERATOR_RECONCILE_CAPABILITY,
};
use elohim_storage::api::seed_delegates_compute::{perform_seed, SeedDelegatesInput};
use elohim_storage::db::mishpat_commitments;
use elohim_storage::db::AppContext;
use elohim_storage::services::rate_history::{DieselRateHistory, RateHistory};
use elohim_storage::test_util::test_pool;

const MATTHEW: &str = "uhCAk-matthew";
const JAMES: &str = "uhCAk-james";
const SELF_CID: &str = "uhCAk-elohim-host-peer";
const GRANT_CID: &str = "commitment:operator-reconcile-matthew";
/// Wildcard epr_scope, commons ceiling, generous rate, 90-day rotation TTL.
const BOUNDS_JSON: &str = r#"{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":60,"rotation_ttl_days":90,"_provenance":"dev-seed"}"#;
const VALID_FROM: &str = "2026-08-01T00:00:00Z";
const VALID_UNTIL: &str = "2026-11-01T00:00:00Z";
const NOW: &str = "2026-08-18T12:00:00Z";

fn ctx() -> AppContext {
    AppContext::default_elohim()
}

/// Recorded uses of `grant_cid` inside the sliding 60-minute window at `NOW` —
/// exactly what bounds_validator check 6 counts.
async fn uses_in_window(pool: &elohim_storage::db::DbPool, grant_cid: &str) -> u32 {
    DieselRateHistory { pool: pool.clone() }
        .count_in_window(grant_cid, NOW, 60)
        .await
        .expect("rate window query")
}

fn seed_operator_grant(pool: &elohim_storage::db::DbPool) {
    let mut conn = pool.get().expect("conn");
    perform_seed(
        &mut conn,
        &SeedDelegatesInput {
            cid: GRANT_CID,
            scope: OPERATOR_RECONCILE_CAPABILITY,
            provider: MATTHEW,
            recipient: MATTHEW,
            bounds_json: BOUNDS_JSON,
            valid_from: VALID_FROM,
            valid_until: VALID_UNTIL,
        },
    )
    .expect("seed grant");
}

#[tokio::test]
async fn holder_is_accepted_and_attestation_names_the_commitment() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    let outcome = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;

    match outcome {
        ReconcileVerbOutcome::Accepted(att) => {
            assert!(att.accepted);
            assert_eq!(att.verb, "reconcile");
            assert_eq!(att.commitment_cid, GRANT_CID);
            assert_eq!(att.attested_by, SELF_CID);
            assert_eq!(att.outcome, "reconcile-scheduled");
        }
        other => panic!("expected Accepted, got {other:?}"),
    }
    // The kick actually reached the loop's channel.
    assert!(rx.try_recv().is_ok(), "a kick must be queued for the loop");
}

#[tokio::test]
async fn caller_without_a_grant_is_refused_and_no_kick_fires() {
    let pool = test_pool();
    seed_operator_grant(&pool); // matthew's grant exists; james holds none
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    let outcome =
        perform_reconcile_verb(&pool, &ctx(), Some(JAMES), Some(&tx), SELF_CID, NOW.into()).await;

    match outcome {
        ReconcileVerbOutcome::Refused(r) => {
            assert!(!r.accepted);
            assert!(
                r.reason.contains("no active delegates-compute grant"),
                "reason was: {}",
                r.reason
            );
        }
        other => panic!("expected Refused, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "refusal must not kick the loop");
}

#[tokio::test]
async fn revoked_grant_stops_working_immediately() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    {
        let mut conn = pool.get().expect("conn");
        mishpat_commitments::set_revoked_at(&mut conn, GRANT_CID, NOW).expect("revoke");
    }
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    let outcome = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;

    match outcome {
        ReconcileVerbOutcome::Refused(r) => assert!(!r.accepted),
        other => panic!("expected Refused after revocation, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "revoked grant must not kick");
}

#[tokio::test]
async fn missing_verified_performer_is_refused_without_touching_the_gate() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    for performer in [None, Some("")] {
        let outcome =
            perform_reconcile_verb(&pool, &ctx(), performer, Some(&tx), SELF_CID, NOW.into()).await;
        match outcome {
            ReconcileVerbOutcome::Refused(r) => {
                assert_eq!(r.reason, "no-verified-performer");
            }
            other => panic!("expected Refused, got {other:?}"),
        }
    }
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn authorized_but_loop_unavailable_is_an_honest_503() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    // Receiver dropped = the reconcile loop never spawned (disabled arm).
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);
    drop(rx);

    let outcome = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;

    assert!(
        matches!(outcome, ReconcileVerbOutcome::LoopUnavailable),
        "an authorized verb on a loopless peer must be an honest 503, got {outcome:?}"
    );
}

// ---------------------------------------------------------------------------
// Slice 2 — emit-then-act: the use is recorded and the rate ceiling is REAL
// ---------------------------------------------------------------------------

/// A grant allowing 2 uses/hour: bounds identical to BOUNDS_JSON except rate.
const BOUNDS_JSON_RATE_2: &str = r#"{"epr_scope":["*"],"reach_ceiling":"commons","rate_per_hour":2,"rotation_ttl_days":90,"_provenance":"dev-seed"}"#;

#[tokio::test]
async fn an_accepted_verb_records_its_use_bounded_by_the_grant() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    assert_eq!(uses_in_window(&pool, GRANT_CID).await, 0);

    let outcome = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;

    let att = match outcome {
        ReconcileVerbOutcome::Accepted(att) => att,
        other => panic!("expected Accepted, got {other:?}"),
    };
    // The attestation names the recorded economic event, and that event is
    // exactly what the rate window (bounds_validator check 6) counts.
    assert!(att.attestation_event_id.starts_with("ee-opverb-"));
    assert_eq!(
        uses_in_window(&pool, GRANT_CID).await,
        1,
        "the accepted use must land in economic_events.bounded_by"
    );
    let _ = rx.try_recv();
}

#[tokio::test]
async fn a_refusal_records_no_use() {
    let pool = test_pool();
    seed_operator_grant(&pool); // james holds no grant
    let (tx, _rx) = tokio::sync::mpsc::channel::<()>(1);

    let outcome =
        perform_reconcile_verb(&pool, &ctx(), Some(JAMES), Some(&tx), SELF_CID, NOW.into()).await;
    assert!(matches!(outcome, ReconcileVerbOutcome::Refused(_)));
    assert_eq!(
        uses_in_window(&pool, GRANT_CID).await,
        0,
        "a refused verb must not charge the grant's rate window"
    );
}

#[tokio::test]
async fn a_loopless_peer_charges_no_use() {
    let pool = test_pool();
    seed_operator_grant(&pool);
    let (tx, rx) = tokio::sync::mpsc::channel::<()>(1);
    drop(rx); // loop never spawned — pre-check fires BEFORE accounting

    let outcome = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;
    assert!(matches!(outcome, ReconcileVerbOutcome::LoopUnavailable));
    assert_eq!(
        uses_in_window(&pool, GRANT_CID).await,
        0,
        "an honest 503 must not charge the rate window (no phantom use)"
    );
}

#[tokio::test]
async fn the_rate_ceiling_refuses_the_use_past_the_grant_limit() {
    let pool = test_pool();
    {
        let mut conn = pool.get().expect("conn");
        perform_seed(
            &mut conn,
            &SeedDelegatesInput {
                cid: GRANT_CID,
                scope: OPERATOR_RECONCILE_CAPABILITY,
                provider: MATTHEW,
                recipient: MATTHEW,
                bounds_json: BOUNDS_JSON_RATE_2,
                valid_from: VALID_FROM,
                valid_until: VALID_UNTIL,
            },
        )
        .expect("seed rate-2 grant");
    }
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);

    // Use 1: window count 0 < 2 → accepted, recorded.
    let first = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;
    assert!(
        matches!(first, ReconcileVerbOutcome::Accepted(_)),
        "{first:?}"
    );
    let _ = rx.try_recv(); // drain so kick outcomes stay Scheduled

    // Use 2: window count 1 < 2 → accepted, recorded.
    let second = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;
    assert!(
        matches!(second, ReconcileVerbOutcome::Accepted(_)),
        "{second:?}"
    );
    let _ = rx.try_recv();

    // Use 3: window count 2 — NOT < 2 → the previously-inert rate check now
    // refuses, and no third use is recorded.
    let third = perform_reconcile_verb(
        &pool,
        &ctx(),
        Some(MATTHEW),
        Some(&tx),
        SELF_CID,
        NOW.into(),
    )
    .await;
    match third {
        ReconcileVerbOutcome::Refused(r) => assert_eq!(
            r.reason, "rate-limit-exceeded",
            "the grant's rate_per_hour ceiling must be the refusal reason"
        ),
        other => panic!("expected rate refusal, got {other:?}"),
    }
    assert!(rx.try_recv().is_err(), "a rate-refused verb must not kick");
    assert_eq!(
        uses_in_window(&pool, GRANT_CID).await,
        2,
        "the refused third use must not be recorded"
    );
}
