use elohim_storage::graduation::summary_event::SummaryEventSpec;
use elohim_storage::test_util::test_pool;
use diesel::RunQueryDsl;

#[test]
fn summarise_blob_served_aggregates_three_observations() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    // Insert three observations using raw SQL (simpler than ORM with type inference)
    for i in 0..3i64 {
        let payload = format!(
            r#"{{"blob_cid":"b{}","bytes":1000,"peer_cid":"r1"}}"#,
            i
        );
        let sql = format!(
            "INSERT INTO observations (observer_cid, log_cid, log_offset, observed_at, seq, \
             observation_kind, subject_cid, subject_kind, payload_json, signature_b64) \
             VALUES ('provider:x', 'log_abc{}', {}, {}, {}, 'infrastructure:blob-served', 'blob:b{}', 'blob', '{}', '')",
            i,
            i,
            1_715_420_400i64 + i,
            i,
            i,
            payload.replace("'", "''")
        );
        diesel::sql_query(sql)
            .execute(&mut conn)
            .unwrap();
    }

    let spec = SummaryEventSpec {
        observation_kind: "infrastructure:blob-served".into(),
        action_verb: "served-blob-summary".into(),
        resource: "blob-bytes".into(),
        window_seconds: 3600,
    };
    let summary = spec
        .evaluate(&mut conn, 1_715_420_400 + 3600)
        .unwrap()
        .expect("summary must produce when observations exist in window");

    assert_eq!(summary.action, "served-blob-summary");
    assert_eq!(summary.observation_refs.len(), 3);
    assert!(summary.total_quantity > 0.0, "total bytes must sum");
}

#[test]
fn summarise_returns_none_when_no_observations_in_window() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let spec = SummaryEventSpec {
        observation_kind: "infrastructure:blob-served".into(),
        action_verb: "served-blob-summary".into(),
        resource: "blob-bytes".into(),
        window_seconds: 3600,
    };
    let summary = spec.evaluate(&mut conn, 1_715_420_400).unwrap();
    assert!(summary.is_none());
}

#[test]
fn observation_refs_use_iroh_url_format() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    let payload = r#"{"blob_cid":"b0","bytes":500,"peer_cid":"r1"}"#;
    let sql = format!(
        "INSERT INTO observations (observer_cid, log_cid, log_offset, observed_at, seq, \
         observation_kind, subject_cid, subject_kind, payload_json, signature_b64) \
         VALUES ('provider:x', 'log_abc123', 42, 1715420400, 0, 'infrastructure:blob-served', \
         'blob:b0', 'blob', '{}', '')",
        payload.replace("'", "''")
    );
    diesel::sql_query(sql)
        .execute(&mut conn)
        .unwrap();

    let spec = SummaryEventSpec {
        observation_kind: "infrastructure:blob-served".into(),
        action_verb: "served-blob-summary".into(),
        resource: "blob-bytes".into(),
        window_seconds: 3600,
    };
    let summary = spec
        .evaluate(&mut conn, 1_715_420_400 + 3600)
        .unwrap()
        .unwrap();

    assert!(
        summary.observation_refs[0].starts_with("iroh://provider:x@"),
        "ref must be iroh-URL with provider observer, got: {}",
        summary.observation_refs[0]
    );
    assert!(
        summary.observation_refs[0].contains("#42"),
        "ref must have offset fragment #42, got: {}",
        summary.observation_refs[0]
    );
}
