//! The three cross-observer observation GET routes obey the privacy invariant
//! (ruling R-A9 of the post-station-4 sprint; review finding H1).
//!
//! - `by-observer` answers only the observer themself: the explicit
//!   `X-Agent-Cid` must equal `observerCid` (no header → 401, another
//!   observer → 403, never the `local_sessions` fallback);
//! - `by-subject` and `diversity` never list or count a kind the registry
//!   marks `agent-private`: asking for one answers 404 whether or not rows
//!   exist (no existence oracle), and a node with no registry refuses every
//!   cross-observer read rather than guess which kinds are private.
//!
//! The handlers delegate to the `api::observations` functions these tests
//! drive directly (no hyper harness; see `api_observations_write_test.rs`).
//! Errors map through `api::observations::route_error` (R-A12), so the asserted
//! status IS the status the route answers with.

use diesel::prelude::*;
use elohim_storage::api::observations::{
    observation_diversity, observations_by_observer, observations_by_subject, route_error,
};
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::db::models::NewObservationRow;
use elohim_storage::error::StorageError;
use elohim_storage::services::observation_kinds::ObservationKindRegistry;
use elohim_storage::test_util::test_pool;
use hyper::StatusCode;

const JESSICA: &str = "human-jessica";
const JAMES: &str = "human-james";
const VIEWED: &str = "lamad:content-viewed";
const HEARTBEAT: &str = "infrastructure:doorway-heartbeat";
const SUBJECT: &str = "bafy-shared-subject";

/// The status the ROUTE answers with: `observations::handle` maps every
/// handler error through this same `route_error`, so what these tests assert is
/// what a caller receives. Before ruling R-A12 the errors escaped unmapped and
/// every one of them reached the wire as a bare 500 — these assertions held
/// while the route lied.
fn status_of(err: StorageError) -> StatusCode {
    route_error(err).status()
}

fn registry() -> ObservationKindRegistry {
    ObservationKindRegistry::embedded()
}

fn seed(conn: &mut SqliteConnection, observer: &str, offset: i64, kind: &str) {
    let log_cid = format!("blake3:{observer}");
    diesel::insert_into(observations::table)
        .values(NewObservationRow {
            observer_cid: observer,
            log_cid: &log_cid,
            log_offset: offset,
            observed_at: 1_790_000_000 + offset,
            seq: offset + 1,
            observation_kind: kind,
            subject_cid: Some(SUBJECT),
            subject_kind: Some("content"),
            payload_json: "{}",
            observer_household_cid: None,
            observer_collective_cid: None,
            observer_region: None,
            observer_archetype: None,
            observer_compute_class: None,
            signature_b64: "",
        })
        .execute(conn)
        .expect("seed observation");
}

/// Jessica and James each viewed the same subject (agent-private) and each
/// sent a heartbeat about it (community).
fn seed_both(conn: &mut SqliteConnection) {
    seed(conn, JESSICA, 0, VIEWED);
    seed(conn, JESSICA, 1, HEARTBEAT);
    seed(conn, JAMES, 0, VIEWED);
    seed(conn, JAMES, 1, HEARTBEAT);
}

#[test]
fn registry_marks_content_viewed_agent_private() {
    // The premise of every test below.
    assert!(registry().get(VIEWED).unwrap().is_agent_private());
    assert!(!registry().get(HEARTBEAT).unwrap().is_agent_private());
}

#[test]
fn by_observer_without_header_is_401() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_both(&mut conn);
    let q = format!("observerCid={JESSICA}");
    let err = observations_by_observer(&mut conn, None, &q).unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);
    let err = observations_by_observer(&mut conn, Some(""), &q).unwrap_err();
    assert_eq!(status_of(err), StatusCode::UNAUTHORIZED);
}

#[test]
fn by_observer_for_another_observer_is_403() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_both(&mut conn);
    let err = observations_by_observer(&mut conn, Some(JAMES), &format!("observerCid={JESSICA}"))
        .unwrap_err();
    assert_eq!(status_of(err), StatusCode::FORBIDDEN);
    // With a kind filter too — the observer check comes first.
    let err = observations_by_observer(
        &mut conn,
        Some(JAMES),
        &format!("observerCid={JESSICA}&kind={HEARTBEAT}"),
    )
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::FORBIDDEN);
}

#[test]
fn by_observer_self_still_serves() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_both(&mut conn);
    let rows =
        observations_by_observer(&mut conn, Some(JESSICA), &format!("observerCid={JESSICA}"))
            .expect("the observer reads their own rows");
    assert_eq!(rows.len(), 2, "both of her kinds, private ones included");
    assert!(rows.iter().all(|r| r.observer_cid == JESSICA));
    // Newest first.
    assert!(rows[0].observed_at >= rows[1].observed_at);

    let rows = observations_by_observer(
        &mut conn,
        Some(JESSICA),
        &format!("observerCid={JESSICA}&kind={VIEWED}"),
    )
    .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].observation_kind, VIEWED);
}

#[test]
fn by_subject_never_returns_agent_private_kinds() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_both(&mut conn);
    let reg = registry();

    // Asking for a private kind is refused whether or not rows exist.
    for subject in [SUBJECT, "bafy-nobody-looked"] {
        let err = observations_by_subject(
            &mut conn,
            Some(&reg),
            &format!("subjectCid={subject}&kind={VIEWED}"),
        )
        .unwrap_err();
        assert_eq!(status_of(err), StatusCode::NOT_FOUND, "{subject}");
    }

    // A community kind still lists across observers — and nothing private rides along.
    let rows = observations_by_subject(
        &mut conn,
        Some(&reg),
        &format!("subjectCid={SUBJECT}&kind={HEARTBEAT}"),
    )
    .expect("community kinds are listed");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| r.observation_kind == HEARTBEAT));

    // No registry: which kinds are private is unknown, so nothing is listed.
    let err = observations_by_subject(
        &mut conn,
        None,
        &format!("subjectCid={SUBJECT}&kind={HEARTBEAT}"),
    )
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
}

#[test]
fn diversity_never_counts_agent_private_kinds() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    seed_both(&mut conn);
    let reg = registry();

    let err = observation_diversity(
        &mut conn,
        Some(&reg),
        &format!("subjectCid={SUBJECT}&kind={VIEWED}"),
    )
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);

    let summary = observation_diversity(
        &mut conn,
        Some(&reg),
        &format!("subjectCid={SUBJECT}&kind={HEARTBEAT}"),
    )
    .expect("community kinds are counted")
    .expect("two heartbeats seeded");
    assert_eq!(summary.distinct_agents, 2);
    assert_eq!(summary.total_count, 2, "the private views are not counted");

    let err = observation_diversity(
        &mut conn,
        None,
        &format!("subjectCid={SUBJECT}&kind={HEARTBEAT}"),
    )
    .unwrap_err();
    assert_eq!(status_of(err), StatusCode::NOT_FOUND);
}
