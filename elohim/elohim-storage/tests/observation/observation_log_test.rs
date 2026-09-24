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

/// Review W4 (ruling R-A10): the log keeps its hasher and offset, and at most
/// a bounded tail of recent observations for `read_from` — never every
/// observation ever appended. The SQL projection is where the history lives.
#[tokio::test]
async fn log_memory_does_not_grow_with_appends() {
    use elohim_storage::observation::log::TAIL_CAPACITY;
    let mut log = ObservationLog::new_in_memory("agent:test".into());
    let appended = (TAIL_CAPACITY as u64) * 4;
    for i in 0..appended {
        log.append(fixture_obs(i)).await.unwrap();
        assert!(log.retained_len() <= TAIL_CAPACITY, "after {i} appends");
    }
    assert_eq!(log.latest_offset(), appended, "the offset still counts every append");

    // read_from serves the retained tail; earlier offsets are skipped, as for a resumed log.
    let all = log.read_from(0).await.unwrap();
    assert_eq!(all.len(), TAIL_CAPACITY);
    assert_eq!(all[0].seq, appended - TAIL_CAPACITY as u64);
    let last = log.read_from(appended - 1).await.unwrap();
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].seq, appended - 1);

    // A log that keeps no tail keeps nothing at all, and its root still advances.
    let mut bare = ObservationLog::new_in_memory("agent:test".into()).with_tail_capacity(0);
    let before = bare.current_log_cid();
    for i in 0..100u64 {
        bare.append(fixture_obs(i)).await.unwrap();
    }
    assert_eq!(bare.retained_len(), 0);
    assert_eq!(bare.latest_offset(), 100);
    assert_ne!(bare.current_log_cid(), before);
    assert!(bare.read_from(0).await.unwrap().is_empty());
}
