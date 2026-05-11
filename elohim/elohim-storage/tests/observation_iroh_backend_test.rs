#![cfg(feature = "p2p-iroh")]

use elohim_storage::observation::log::ObservationLog;
use elohim_storage::observation::wire::Observation;
use elohim_storage::p2p_iroh::observation_backend::IrohObservationBackend;

fn fixture(observer_cid: &str, seq: u64) -> Observation {
    Observation {
        observer_cid: observer_cid.into(),
        log_cid: "blake3:test".into(),
        log_offset: seq,
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
async fn fetch_segment_returns_appended_observations() {
    let mut log = ObservationLog::new_in_memory("agent:obs-A".into());
    for i in 0..3u64 {
        log.append(fixture("agent:obs-A", i)).await.unwrap();
    }
    let backend = IrohObservationBackend::new_in_memory(log);
    let segment = backend.fetch_segment("agent:obs-A", 0, 3).await.unwrap();
    assert_eq!(segment.len(), 3);
    assert_eq!(segment[0].seq, 0);
    assert_eq!(segment[2].seq, 2);
}

#[tokio::test]
async fn fetch_segment_filters_by_observer_cid() {
    let mut log = ObservationLog::new_in_memory("agent:obs-A".into());
    log.append(fixture("agent:obs-A", 0)).await.unwrap();
    let backend = IrohObservationBackend::new_in_memory(log);
    let mismatch = backend.fetch_segment("agent:obs-B", 0, 1).await.unwrap();
    assert!(mismatch.is_empty(), "different observer_cid must return empty");
}

#[tokio::test]
async fn fetch_segment_respects_to_offset() {
    let mut log = ObservationLog::new_in_memory("agent:obs-A".into());
    for i in 0..5u64 {
        log.append(fixture("agent:obs-A", i)).await.unwrap();
    }
    let backend = IrohObservationBackend::new_in_memory(log);
    let segment = backend.fetch_segment("agent:obs-A", 1, 3).await.unwrap();
    assert_eq!(segment.len(), 2);
    assert_eq!(segment[0].seq, 1);
    assert_eq!(segment[1].seq, 2);
}
