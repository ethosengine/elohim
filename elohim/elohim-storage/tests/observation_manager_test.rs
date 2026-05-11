use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;
use elohim_storage::test_util::test_pool;

fn fixture_self_authored(subject_cid: Option<&str>) -> Observation {
    Observation {
        observer_cid: "agent:self".into(),
        log_cid: String::new(), // gets stamped by append_local
        log_offset: 0,          // gets stamped by append_local
        observed_at: 1_715_420_400,
        seq: 0,
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        subject_cid: subject_cid.map(|s| s.into()),
        subject_kind: subject_cid.map(|_| "doorway".into()),
        payload_json: "{}".into(),
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![],
    }
}

fn fixture_remote(observer_cid: &str, log_cid: &str, offset: u64) -> Observation {
    Observation {
        observer_cid: observer_cid.into(),
        log_cid: log_cid.into(),
        log_offset: offset,
        observed_at: 1_715_420_400 + offset as i64,
        seq: offset,
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        subject_cid: Some("doorway:abc".into()),
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

#[tokio::test]
async fn append_local_stamps_log_metadata_and_projects() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    let obs = fixture_self_authored(Some("doorway:abc"));
    mgr.append_local(&mut conn, obs).await.unwrap();

    let rows = mgr
        .query_by_subject(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].observer_cid, "agent:self");
    assert!(!rows[0].log_cid.is_empty(), "log_cid must be stamped by append_local");
}

#[tokio::test]
async fn project_remote_writes_without_local_log_advance() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    let remote = fixture_remote("agent:other", "blake3:remote", 7);
    mgr.project_remote(&mut conn, &remote).await.unwrap();

    let rows = mgr
        .query_by_subject(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat")
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].observer_cid, "agent:other");
    assert_eq!(rows[0].log_cid, "blake3:remote");
    assert_eq!(rows[0].log_offset, 7);
}

#[tokio::test]
async fn append_local_offset_increments_per_append() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for _ in 0..3 {
        mgr.append_local(&mut conn, fixture_self_authored(Some("doorway:abc")))
            .await
            .unwrap();
    }

    let rows = mgr
        .query_by_subject(&mut conn, "doorway:abc", "infrastructure:doorway-heartbeat")
        .unwrap();
    assert_eq!(rows.len(), 3);
    let mut offsets: Vec<i64> = rows.iter().map(|r| r.log_offset).collect();
    offsets.sort();
    assert_eq!(offsets, vec![0, 1, 2]);
}
