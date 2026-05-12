use diesel::prelude::*;
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::db::models::NewObservationRow;
use elohim_storage::graduation::attestation::AttestationGraduationSpec;
use elohim_storage::graduation::diversity::DiversityThreshold;
use elohim_storage::test_util::test_pool;

fn insert_obs(
    conn: &mut SqliteConnection,
    observer: &str,
    household: &str,
    subject: &str,
    seq: i64,
) {
    let log_cid = format!("log:{}", seq);
    let observation_kind = "infrastructure:doorway-heartbeat";
    let row = NewObservationRow {
        observer_cid: observer,
        log_cid: &log_cid,
        log_offset: seq,
        observed_at: 1_715_420_400 + seq,
        seq,
        observation_kind,
        subject_cid: Some(subject),
        subject_kind: Some("doorway"),
        payload_json: "{}",
        observer_household_cid: Some(household),
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
fn graduation_emits_plan_when_threshold_met() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    for (i, hh) in [(0i64, "h1"), (1, "h2"), (2, "h3"), (3, "h1"), (4, "h2")].iter() {
        insert_obs(&mut conn, &format!("agent:{}", i), &format!("household:{}", hh), "doorway:abc", *i);
    }

    let spec = AttestationGraduationSpec {
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        attestation_subtype: "doorway-health".into(),
        threshold: DiversityThreshold {
            distinct_households: Some(3),
            min_count: Some(5),
            ..Default::default()
        },
    };
    let plan = spec
        .evaluate(&mut conn, "doorway:abc")
        .unwrap()
        .expect("must produce plan");
    assert_eq!(plan.attestation_content_type, "attestation:doorway-health");
    assert_eq!(plan.subject_cid, "doorway:abc");
    assert_eq!(plan.observation_refs.len(), 5);
    assert_eq!(plan.proof_class, "witness");
}

#[test]
fn graduation_returns_none_below_threshold() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    for i in 0..2i64 {
        insert_obs(&mut conn, &format!("agent:{}", i), "household:single", "doorway:abc", i);
    }

    let spec = AttestationGraduationSpec {
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        attestation_subtype: "doorway-health".into(),
        threshold: DiversityThreshold {
            distinct_households: Some(3),
            min_count: Some(5),
            ..Default::default()
        },
    };
    assert!(spec
        .evaluate(&mut conn, "doorway:abc")
        .unwrap()
        .is_none());
}

#[test]
fn observation_refs_use_iroh_url_format() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();

    for (i, hh) in [(0i64, "h1"), (1, "h2"), (2, "h3")].iter() {
        insert_obs(&mut conn, &format!("agent:{}", i), &format!("household:{}", hh), "doorway:abc", *i);
    }

    let spec = AttestationGraduationSpec {
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        attestation_subtype: "doorway-health".into(),
        threshold: DiversityThreshold {
            distinct_households: Some(3),
            min_count: Some(3),
            ..Default::default()
        },
    };
    let plan = spec
        .evaluate(&mut conn, "doorway:abc")
        .unwrap()
        .unwrap();
    assert!(
        plan.observation_refs[0].starts_with("iroh://"),
        "refs must use iroh URL, got: {}",
        plan.observation_refs[0]
    );
    assert!(
        plan.observation_refs[0].contains("#"),
        "ref must include offset fragment"
    );
}
