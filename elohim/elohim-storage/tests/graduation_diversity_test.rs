use elohim_storage::graduation::diversity::{threshold_met, DiversityThreshold};
use elohim_storage::observation::manager::ObservationManagerBackend;
use elohim_storage::observation::wire::Observation;
use elohim_storage::test_util::test_pool;

fn obs(observer: &str, household: Option<&str>, subject: &str, kind: &str, seq: u64) -> Observation {
    Observation {
        observer_cid: observer.into(),
        log_cid: String::new(),
        log_offset: 0,
        observed_at: 1_715_420_400 + seq as i64,
        seq,
        observation_kind: kind.into(),
        subject_cid: Some(subject.into()),
        subject_kind: Some("doorway".into()),
        payload_json: "{}".into(),
        observer_household_cid: household.map(|s| s.into()),
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![],
    }
}

#[tokio::test]
async fn threshold_not_met_with_single_household() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for i in 0..5u64 {
        let obs = obs(
            &format!("agent:{}", i),
            Some("household:single"),
            "doorway:abc",
            "infrastructure:doorway-heartbeat",
            i,
        );
        mgr.append_local(&mut conn, obs).await.unwrap();
    }

    let threshold = DiversityThreshold {
        distinct_households: Some(3),
        min_count: Some(5),
        ..Default::default()
    };
    assert!(!threshold_met(
        &mut conn,
        "doorway:abc",
        "infrastructure:doorway-heartbeat",
        &threshold
    )
    .unwrap());
}

#[tokio::test]
async fn threshold_met_with_three_households() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    for (i, hh) in [
        (0u64, "h1"),
        (1, "h1"),
        (2, "h2"),
        (3, "h2"),
        (4, "h3"),
    ]
    .iter()
    {
        let obs = obs(
            &format!("agent:{}", i),
            Some(&format!("household:{}", hh)),
            "doorway:abc",
            "infrastructure:doorway-heartbeat",
            *i,
        );
        mgr.append_local(&mut conn, obs).await.unwrap();
    }

    let threshold = DiversityThreshold {
        distinct_households: Some(3),
        min_count: Some(5),
        ..Default::default()
    };
    assert!(threshold_met(
        &mut conn,
        "doorway:abc",
        "infrastructure:doorway-heartbeat",
        &threshold
    )
    .unwrap());
}

#[tokio::test]
async fn threshold_returns_false_when_no_observations() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let threshold = DiversityThreshold {
        min_count: Some(1),
        ..Default::default()
    };
    assert!(!threshold_met(
        &mut conn,
        "doorway:absent",
        "infrastructure:doorway-heartbeat",
        &threshold
    )
    .unwrap());
}

#[tokio::test]
async fn threshold_with_all_dimensions_evaluates_each() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let mgr = ObservationManagerBackend::new_in_memory("agent:self".into());

    let mut obs = obs(
        "agent:0",
        Some("household:h1"),
        "doorway:abc",
        "infrastructure:doorway-heartbeat",
        0,
    );
    obs.observer_region = Some("us-west".into());
    obs.observer_archetype = Some("tier-1-hub".into());
    obs.observer_compute_class = Some("rpi4-arm64".into());
    obs.observer_collective_cid = Some("collective:home".into());
    mgr.append_local(&mut conn, obs).await.unwrap();

    let threshold = DiversityThreshold {
        distinct_households: Some(1),
        distinct_collectives: Some(1),
        distinct_regions: Some(1),
        distinct_archetypes: Some(1),
        min_count: Some(1),
    };
    assert!(threshold_met(
        &mut conn,
        "doorway:abc",
        "infrastructure:doorway-heartbeat",
        &threshold
    )
    .unwrap());
}
