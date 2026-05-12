//! Integration tests for observation HTTP routes (Stage 7.1–7.3).
//!
//! Follows the same DB-layer strategy as `api_placement_gaps.rs`:
//! no live HTTP server — tests exercise the DB queries that handlers
//! delegate to and verify `ObservationView` / `ObservationDiversitySummaryView`
//! camelCase wire format.
//!
//! HTTP-server variants are left `#[ignore]`d until Task 17 provides
//! `spawn_test_server`.

use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;
use elohim_storage::test_util::test_pool;
use elohim_storage::views::{ObservationDiversitySummaryView, ObservationView};

use diesel::prelude::*;
use elohim_storage::db::diesel_schema::{observation_diversity_summary, observations};
use elohim_storage::db::models::ObservationDiversitySummaryRow;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

fn obs(observer: &str, log: &str, offset: u64, subject: &str, kind: &str) -> Observation {
    Observation {
        observer_cid: observer.into(),
        log_cid: log.into(),
        log_offset: offset,
        observed_at: 1_715_420_400 + offset as i64,
        seq: offset,
        observation_kind: kind.into(),
        subject_cid: Some(subject.into()),
        subject_kind: Some("doorway".into()),
        payload_json: "{}".into(),
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![],
    }
}

// ---------------------------------------------------------------------------
// by-subject handler path: query_by_subject
// ---------------------------------------------------------------------------

#[tokio::test]
async fn by_subject_returns_empty_when_no_rows() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    let rows = mgr
        .query_by_subject(
            &mut conn,
            "doorway:never",
            "infrastructure:doorway-heartbeat",
        )
        .unwrap();

    assert!(rows.is_empty(), "expected no rows for unknown subject_cid");
}

#[tokio::test]
async fn by_subject_returns_matching_rows_newest_first() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:alpha".into());

    // Seed two observations for the subject, one for a different subject.
    for offset in [0u64, 1] {
        let o = obs(
            "agent:alpha",
            "log:a",
            offset,
            "doorway:target",
            "infrastructure:doorway-heartbeat",
        );
        mgr.project_remote(&mut conn, &o).await.unwrap();
    }
    // Different subject — must not appear in results.
    let other = obs(
        "agent:alpha",
        "log:a",
        2,
        "doorway:other",
        "infrastructure:doorway-heartbeat",
    );
    mgr.project_remote(&mut conn, &other).await.unwrap();

    let rows = mgr
        .query_by_subject(
            &mut conn,
            "doorway:target",
            "infrastructure:doorway-heartbeat",
        )
        .unwrap();

    assert_eq!(rows.len(), 2, "should return 2 rows for doorway:target");
    // Newest first: observed_at descending
    assert!(
        rows[0].observed_at >= rows[1].observed_at,
        "rows must be newest-first"
    );

    // Verify camelCase wire format on ObservationView
    let json = serde_json::to_string(&rows[0]).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        v.get("observerCid").is_some(),
        "wire must have camelCase observerCid"
    );
    assert!(
        v.get("observationKind").is_some(),
        "wire must have camelCase observationKind"
    );
    assert!(
        v.get("observedAt").is_some(),
        "wire must have camelCase observedAt"
    );
}

#[tokio::test]
async fn by_subject_filters_by_kind() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:beta".into());

    // One heartbeat, one different kind — both for same subject.
    let heartbeat = obs(
        "agent:beta",
        "log:b",
        0,
        "doorway:same",
        "infrastructure:doorway-heartbeat",
    );
    mgr.project_remote(&mut conn, &heartbeat).await.unwrap();

    let mut capacity = obs(
        "agent:beta",
        "log:b",
        1,
        "doorway:same",
        "compute:capacity-report",
    );
    capacity.log_offset = 1;
    capacity.seq = 1;
    mgr.project_remote(&mut conn, &capacity).await.unwrap();

    let heartbeat_rows = mgr
        .query_by_subject(
            &mut conn,
            "doorway:same",
            "infrastructure:doorway-heartbeat",
        )
        .unwrap();
    assert_eq!(
        heartbeat_rows.len(),
        1,
        "kind filter must isolate heartbeat"
    );
    assert_eq!(
        heartbeat_rows[0].observation_kind,
        "infrastructure:doorway-heartbeat"
    );

    let capacity_rows = mgr
        .query_by_subject(&mut conn, "doorway:same", "compute:capacity-report")
        .unwrap();
    assert_eq!(
        capacity_rows.len(),
        1,
        "kind filter must isolate capacity-report"
    );
}

// ---------------------------------------------------------------------------
// by-observer handler path: direct Diesel query (observer_cid filter)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn by_observer_returns_all_kinds_when_no_kind_filter() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:gamma".into());

    // Two different kinds from the same observer.
    for (offset, kind) in [
        (0u64, "infrastructure:doorway-heartbeat"),
        (1, "compute:capacity-report"),
    ] {
        let o = obs("agent:gamma", "log:g", offset, "doorway:x", kind);
        mgr.project_remote(&mut conn, &o).await.unwrap();
    }

    // Query all by observer (no kind filter) — mirrors the boxed-query handler path.
    let rows: Vec<elohim_storage::db::models::ObservationRow> = observations::table
        .filter(observations::observer_cid.eq("agent:gamma"))
        .order(observations::observed_at.desc())
        .load(&mut conn)
        .unwrap();

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    assert_eq!(
        views.len(),
        2,
        "by-observer with no kind filter returns all kinds"
    );
}

#[tokio::test]
async fn by_observer_filters_by_kind_when_provided() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:delta".into());

    for (offset, kind) in [
        (0u64, "infrastructure:doorway-heartbeat"),
        (1, "compute:capacity-report"),
        (2, "infrastructure:doorway-heartbeat"),
    ] {
        let o = obs("agent:delta", "log:d", offset, "doorway:y", kind);
        mgr.project_remote(&mut conn, &o).await.unwrap();
    }

    // Apply kind filter — mirrors the boxed-query handler path.
    let rows: Vec<elohim_storage::db::models::ObservationRow> = observations::table
        .filter(observations::observer_cid.eq("agent:delta"))
        .filter(observations::observation_kind.eq("infrastructure:doorway-heartbeat"))
        .order(observations::observed_at.desc())
        .load(&mut conn)
        .unwrap();

    let views: Vec<ObservationView> = rows.into_iter().map(ObservationView::from).collect();
    assert_eq!(views.len(), 2, "kind filter returns only matching rows");
    for v in &views {
        assert_eq!(v.observation_kind, "infrastructure:doorway-heartbeat");
    }
}

// ---------------------------------------------------------------------------
// diversity handler path: observation_diversity_summary SQLite view
// ---------------------------------------------------------------------------

#[tokio::test]
async fn diversity_returns_none_when_no_observations() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let row: Option<ObservationDiversitySummaryRow> = observation_diversity_summary::table
        .filter(observation_diversity_summary::subject_cid.eq("doorway:ghost"))
        .filter(
            observation_diversity_summary::observation_kind.eq("infrastructure:doorway-heartbeat"),
        )
        .first(&mut conn)
        .optional()
        .unwrap();

    assert!(
        row.is_none(),
        "diversity summary must return None for unknown subject"
    );
}

#[tokio::test]
async fn diversity_returns_summary_after_observations_seeded() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:epsilon".into());

    // Seed 3 observations from 3 distinct observers for same subject+kind.
    for (i, observer) in ["agent:epsilon", "agent:zeta", "agent:eta"]
        .iter()
        .enumerate()
    {
        let o = obs(
            observer,
            "log:e",
            i as u64,
            "doorway:multi",
            "infrastructure:doorway-heartbeat",
        );
        mgr.project_remote(&mut conn, &o).await.unwrap();
    }

    let row: Option<ObservationDiversitySummaryRow> = observation_diversity_summary::table
        .filter(observation_diversity_summary::subject_cid.eq("doorway:multi"))
        .filter(
            observation_diversity_summary::observation_kind.eq("infrastructure:doorway-heartbeat"),
        )
        .first(&mut conn)
        .optional()
        .unwrap();

    assert!(
        row.is_some(),
        "diversity summary must exist after observations seeded"
    );
    let summary = row.unwrap();
    assert_eq!(summary.total_count, 3, "total_count should be 3");
    assert_eq!(
        summary.distinct_agents, 3,
        "distinct_agents should be 3 for 3 different observer_cids"
    );

    // Verify camelCase wire format on ObservationDiversitySummaryView
    let view = ObservationDiversitySummaryView::from(summary);
    let json = serde_json::to_string(&view).unwrap();
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(
        v.get("subjectCid").is_some(),
        "wire must have camelCase subjectCid"
    );
    assert!(
        v.get("distinctAgents").is_some(),
        "wire must have camelCase distinctAgents"
    );
    assert!(
        v.get("totalCount").is_some(),
        "wire must have camelCase totalCount"
    );
    assert!(
        v.get("firstObservedAt").is_some(),
        "wire must have camelCase firstObservedAt"
    );
}

// ---------------------------------------------------------------------------
// HTTP-server variants — ignored until Task 17 adds spawn_test_server.
// ---------------------------------------------------------------------------

/// Ignored: `spawn_test_server` does not exist yet.
/// Task 17 (A2O) will add the hyper test-server harness and un-ignore this.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn by_subject_returns_matching_rows_http() {
    unimplemented!("spawn_test_server not yet available")
}

/// Ignored: `spawn_test_server` does not exist yet.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn by_observer_returns_matching_rows_http() {
    unimplemented!("spawn_test_server not yet available")
}

/// Ignored: `spawn_test_server` does not exist yet.
#[tokio::test]
#[ignore = "spawn_test_server not yet implemented — see Task 17"]
async fn diversity_returns_summary_http() {
    unimplemented!("spawn_test_server not yet available")
}
