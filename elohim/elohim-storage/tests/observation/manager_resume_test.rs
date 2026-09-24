//! The observation manager resumes each observer's log from its persisted
//! head (`observation_logs`) instead of re-minting offset 0 after a restart,
//! and keeps one log per observer.
//!
//! Serves `attention-witnessed-privately` (plan task A2).

use diesel::prelude::*;
use elohim_storage::db::diesel_schema::observation_logs;
use elohim_storage::db::models::ObservationLogRow;
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;
use elohim_storage::services::observation_kinds::ObservationKindRegistry;
use elohim_storage::test_util::test_pool;
use std::sync::Arc;

fn viewed(observer: &str, seq: u64) -> Observation {
    Observation {
        observer_cid: observer.into(),
        log_cid: String::new(),
        log_offset: 0,
        observed_at: 1_758_700_000 + seq as i64,
        seq,
        observation_kind: "lamad:content-viewed".into(),
        subject_cid: Some("bafy-node".into()),
        subject_kind: Some("content".into()),
        payload_json: r#"{"ref_cid":"bafy-node","dwell_ms":3200,"scroll_depth_pct":64}"#.into(),
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![],
    }
}

fn head(conn: &mut SqliteConnection, observer: &str) -> ObservationLogRow {
    observation_logs::table
        .filter(observation_logs::observer_cid.eq(observer))
        .select(ObservationLogRow::as_select())
        .first(conn)
        .expect("observation_logs row written by append_local")
}

#[tokio::test]
async fn offsets_continue_after_restart() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    {
        let mgr = ObservationManagerBackend::new();
        mgr.append_local(&mut conn, viewed("agent:jessica", 0))
            .await
            .unwrap();
        mgr.append_local(&mut conn, viewed("agent:jessica", 1))
            .await
            .unwrap();
        let before = head(&mut conn, "agent:jessica");
        assert_eq!(before.latest_offset, 2);
        // No registry: an undeclared retention falls back to the shortest class.
        assert_eq!(before.retention_class, "operational");
    } // manager dropped — the in-memory log is gone, the head row stays

    // The restarted manager is built the way the server builds it: with the
    // manifest-declared kinds, so the head row carries the kind's retention.
    let registry = ObservationKindRegistry::load(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sdk/domains"),
    );
    let restarted = ObservationManagerBackend::new().with_kind_registry(Arc::new(registry));
    let third = restarted
        .append_local(&mut conn, viewed("agent:jessica", 2))
        .await
        .unwrap();

    assert_eq!(
        third.log_offset, 2,
        "a restarted manager must not re-mint offset 0"
    );
    let row = head(&mut conn, "agent:jessica");
    assert_eq!(row.latest_offset, 3);
    assert_eq!(row.latest_log_cid, third.log_cid);
    assert_eq!(row.retention_class, "contextual");
}

#[tokio::test]
async fn resumed_log_cid_differs_from_fresh() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let first = {
        let mgr = ObservationManagerBackend::new();
        mgr.append_local(&mut conn, viewed("agent:jessica", 0))
            .await
            .unwrap()
    };

    // Same observation appended by a restarted manager (chained from the
    // stored root) and by a manager over an empty database (fresh chain).
    let resumed = ObservationManagerBackend::new()
        .append_local(&mut conn, viewed("agent:jessica", 1))
        .await
        .unwrap();

    let fresh_pool = test_pool();
    let mut fresh_conn = fresh_pool.get().unwrap();
    let fresh = ObservationManagerBackend::new()
        .append_local(&mut fresh_conn, viewed("agent:jessica", 1))
        .await
        .unwrap();

    assert!(resumed.log_cid.starts_with("blake3:"));
    assert_ne!(
        resumed.log_cid, first.log_cid,
        "the root advances on resume"
    );
    assert_ne!(
        resumed.log_cid, fresh.log_cid,
        "a resumed chain folds in the stored root, so it cannot equal a fresh chain"
    );
    assert_eq!(fresh.log_offset, 0);
    assert_eq!(resumed.log_offset, 1);
}

#[tokio::test]
async fn two_observers_have_independent_offsets() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new();

    for seq in 0..3 {
        mgr.append_local(&mut conn, viewed("agent:jessica", seq))
            .await
            .unwrap();
    }
    let james_first = mgr
        .append_local(&mut conn, viewed("agent:james", 0))
        .await
        .unwrap();

    assert_eq!(james_first.log_offset, 0, "james's log starts at 0");
    assert_eq!(head(&mut conn, "agent:jessica").latest_offset, 3);
    assert_eq!(head(&mut conn, "agent:james").latest_offset, 1);
    assert_ne!(
        head(&mut conn, "agent:jessica").latest_log_cid,
        head(&mut conn, "agent:james").latest_log_cid
    );
}
