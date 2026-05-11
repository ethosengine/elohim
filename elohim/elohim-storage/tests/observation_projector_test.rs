use diesel::prelude::*;
use diesel::RunQueryDsl;
use elohim_storage::db::diesel_schema::observations;
use elohim_storage::observation::projector::ObservationProjector;
use elohim_storage::observation::wire::Observation;
use elohim_storage::test_util::test_pool;

fn fixture(subject_cid: Option<&str>, observer_cid: &str, offset: i64) -> Observation {
    Observation {
        observer_cid: observer_cid.into(),
        log_cid: "blake3:test".into(),
        log_offset: offset as u64,
        observed_at: 1_715_420_400 + offset,
        seq: offset as u64,
        observation_kind: "infrastructure:doorway-heartbeat".into(),
        subject_cid: subject_cid.map(|s| s.into()),
        subject_kind: subject_cid.map(|_| "doorway".into()),
        payload_json: "{}".into(),
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![0x01, 0x02, 0x03, 0x04],
    }
}

#[test]
fn project_writes_row_then_query_returns_it() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let projector = ObservationProjector::new();

    let obs = fixture(Some("doorway:abc"), "agent:test", 0);
    projector.project(&mut conn, &obs).expect("project");

    let count: i64 = observations::table
        .filter(observations::subject_cid.eq("doorway:abc"))
        .filter(observations::observation_kind.eq("infrastructure:doorway-heartbeat"))
        .count()
        .get_result(&mut conn)
        .expect("count");
    assert_eq!(count, 1);
}

#[test]
fn project_idempotent_on_same_primary_key() {
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let projector = ObservationProjector::new();

    let obs = fixture(Some("doorway:abc"), "agent:test", 0);
    projector.project(&mut conn, &obs).unwrap();
    projector.project(&mut conn, &obs).unwrap();

    let count: i64 = observations::table.count().get_result(&mut conn).unwrap();
    assert_eq!(count, 1, "duplicate (observer_cid, log_cid, log_offset) must not produce a second row");
}

#[test]
fn project_encodes_signature_as_base64() {
    use base64::Engine;
    let pool = test_pool();
    let mut conn = pool.get().unwrap();
    let projector = ObservationProjector::new();

    let mut obs = fixture(None, "agent:test", 0);
    obs.signature = vec![0xff, 0x00, 0xab];
    projector.project(&mut conn, &obs).unwrap();

    let stored: String = observations::table
        .select(observations::signature_b64)
        .first(&mut conn)
        .unwrap();
    assert_eq!(stored, base64::engine::general_purpose::STANDARD.encode([0xff, 0x00, 0xab]));
}
