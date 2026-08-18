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

    let outcome =
        perform_reconcile_verb(&pool, Some(MATTHEW), Some(&tx), SELF_CID, NOW.into()).await;

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

    let outcome = perform_reconcile_verb(&pool, Some(JAMES), Some(&tx), SELF_CID, NOW.into()).await;

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

    let outcome =
        perform_reconcile_verb(&pool, Some(MATTHEW), Some(&tx), SELF_CID, NOW.into()).await;

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
            perform_reconcile_verb(&pool, performer, Some(&tx), SELF_CID, NOW.into()).await;
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

    let outcome =
        perform_reconcile_verb(&pool, Some(MATTHEW), Some(&tx), SELF_CID, NOW.into()).await;

    assert!(
        matches!(outcome, ReconcileVerbOutcome::LoopUnavailable),
        "an authorized verb on a loopless peer must be an honest 503, got {outcome:?}"
    );
}
