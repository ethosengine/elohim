use elohim_storage::observation::log::ObservationLog;
use elohim_storage::observation::wire::Observation;

fn fixture_obs(seq: u64) -> Observation {
    Observation {
        observer_cid: "agent:test".into(),
        log_cid: String::new(),
        log_offset: 0,
        observed_at: 1_715_420_400 + seq as i64,
        seq,
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        subject_cid: None,
        subject_kind: None,
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
async fn append_then_read_returns_observations_in_order() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    log.append(fixture_obs(1)).await.unwrap();
    log.append(fixture_obs(2)).await.unwrap();

    let all = log.read_from(0).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].seq, 1);
    assert_eq!(all[1].seq, 2);
}

#[tokio::test]
async fn appending_advances_log_cid() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    let initial_root = log.current_log_cid();
    log.append(fixture_obs(0)).await.unwrap();
    let after_append = log.current_log_cid();
    assert_ne!(
        initial_root, after_append,
        "log_cid must advance after append"
    );
}

#[tokio::test]
async fn read_from_offset_skips_earlier_rows() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    for i in 0..5u64 {
        log.append(fixture_obs(i)).await.unwrap();
    }
    let tail = log.read_from(3).await.unwrap();
    assert_eq!(tail.len(), 2);
    assert_eq!(tail[0].seq, 3);
}

#[tokio::test]
async fn latest_offset_reflects_append_count() {
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    assert_eq!(log.latest_offset(), 0);
    log.append(fixture_obs(0)).await.unwrap();
    log.append(fixture_obs(1)).await.unwrap();
    log.append(fixture_obs(2)).await.unwrap();
    assert_eq!(log.latest_offset(), 3);
}

#[tokio::test]
async fn current_log_cid_uses_blake3_prefix() {
    let log = ObservationLog::new_in_memory("agent:test".into());
    let cid = log.current_log_cid();
    assert!(
        cid.starts_with("blake3:"),
        "log_cid must use blake3: prefix, got: {}",
        cid
    );
}
