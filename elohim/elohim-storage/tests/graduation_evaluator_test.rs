//! Integration tests for `GraduationEvaluator` — the unified tick entrypoint
//! that drives both attestation and summary graduation paths.

use diesel::prelude::*;
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::db::models::NewObservationRow;
use elohim_storage::graduation::diversity::DiversityThreshold;
use elohim_storage::graduation::evaluator::{GraduationConfig, GraduationEvaluator};
use elohim_storage::test_util::test_pool;

/// Insert a minimal observation row directly into the projection table.
/// Mirrors the helper pattern used in `graduation_attestation_test.rs`.
fn insert_obs(
    conn: &mut SqliteConnection,
    observer: &str,
    household: Option<&str>,
    subject: Option<&str>,
    kind: &str,
    seq: i64,
    payload_json: &str,
) {
    let log_cid = format!("log:{}", seq);
    let row = NewObservationRow {
        observer_cid: observer,
        log_cid: &log_cid,
        log_offset: seq,
        observed_at: 1_715_420_400 + seq,
        seq,
        observation_kind: kind,
        subject_cid: subject,
        subject_kind: subject.map(|_| "doorway"),
        payload_json,
        observer_household_cid: household,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature_b64: "",
    };
    diesel::insert_into(observations::table)
        .values(&row)
        .execute(conn)
        .expect("insert observation");
}

#[test]
fn tick_emits_attestation_plan_when_threshold_met() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // 5 observations across 3 distinct households → threshold met
    for (i, hh) in [(0i64, "h1"), (1, "h2"), (2, "h3"), (3, "h1"), (4, "h2")].iter() {
        insert_obs(
            &mut conn,
            &format!("agent:{}", i),
            Some(&format!("household:{}", hh)),
            Some("doorway:abc"),
            "infrastructure:doorway-heartbeat",
            *i,
            "{}",
        );
    }

    let mut evaluator = GraduationEvaluator::new(vec![GraduationConfig::attestation(
        "infrastructure:doorway-heartbeat",
        "doorway-health",
        DiversityThreshold {
            distinct_households: Some(3),
            min_count: Some(5),
            ..Default::default()
        },
    )]);
    let plans = evaluator.tick(&mut conn, 1_715_420_400 + 10).unwrap();
    assert_eq!(plans.attestations.len(), 1);
    assert_eq!(plans.attestations[0].subject_cid, "doorway:abc");
    assert!(plans.summaries.is_empty());
}

#[test]
fn tick_emits_summary_when_observations_in_window() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // 3 blob-served observations with a bytes payload
    for i in 0..3i64 {
        insert_obs(
            &mut conn,
            "provider:x",
            None,
            None,
            "infrastructure:blob-served",
            i,
            r#"{"bytes":1000}"#,
        );
    }

    let mut evaluator = GraduationEvaluator::new(vec![GraduationConfig::summary(
        "infrastructure:blob-served",
        "served-blob-summary",
        "blob-bytes",
        3600,
    )]);
    // window_end = window_start + 3600; observations are at base+0..2, all < window_end
    let plans = evaluator.tick(&mut conn, 1_715_420_400 + 3600).unwrap();
    assert_eq!(plans.summaries.len(), 1);
    assert_eq!(plans.summaries[0].action, "served-blob-summary");
    assert!(plans.attestations.is_empty());
}

#[test]
fn tick_handles_multiple_subjects_for_same_attestation_kind() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // doorway:abc gets 3 distinct households
    for (i, hh) in [(0i64, "h1"), (1, "h2"), (2, "h3")].iter() {
        insert_obs(
            &mut conn,
            &format!("agent:{}", i),
            Some(&format!("household:{}", hh)),
            Some("doorway:abc"),
            "infrastructure:doorway-heartbeat",
            *i,
            "{}",
        );
    }
    // doorway:xyz also gets 3 distinct households
    for (i, hh) in [(3i64, "h4"), (4, "h5"), (5, "h6")].iter() {
        insert_obs(
            &mut conn,
            &format!("agent:{}", i),
            Some(&format!("household:{}", hh)),
            Some("doorway:xyz"),
            "infrastructure:doorway-heartbeat",
            *i,
            "{}",
        );
    }

    let mut evaluator = GraduationEvaluator::new(vec![GraduationConfig::attestation(
        "infrastructure:doorway-heartbeat",
        "doorway-health",
        DiversityThreshold {
            distinct_households: Some(3),
            min_count: Some(3),
            ..Default::default()
        },
    )]);
    let plans = evaluator.tick(&mut conn, 1_715_420_400 + 100).unwrap();
    assert_eq!(
        plans.attestations.len(),
        2,
        "must produce one plan per qualifying subject"
    );
    let subjects: Vec<String> = plans
        .attestations
        .iter()
        .map(|p| p.subject_cid.clone())
        .collect();
    assert!(subjects.contains(&"doorway:abc".to_string()));
    assert!(subjects.contains(&"doorway:xyz".to_string()));
    assert!(plans.summaries.is_empty());
}

#[test]
fn tick_with_empty_configs_returns_empty_plans() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mut evaluator = GraduationEvaluator::new(vec![]);
    let plans = evaluator.tick(&mut conn, 0).unwrap();
    assert!(plans.attestations.is_empty());
    assert!(plans.summaries.is_empty());
}
