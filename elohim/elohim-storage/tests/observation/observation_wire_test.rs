use elohim_storage::observation::wire::Observation;

#[test]
fn observation_roundtrips_msgpack() {
    let obs = Observation {
        observer_cid: "agent:bafkrei".to_string(),
        log_cid: "blake3:abc123".to_string(),
        log_offset: 42,
        observed_at: 1715420400,
        seq: 100,
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        subject_cid: Some("doorway:bafkrei".to_string()),
        subject_kind: Some("doorway".to_string()),
        payload_json: r#"{"peer_count":12,"uptime_secs":86400}"#.to_string(),
        observer_household_cid: Some("household:bafkrei".to_string()),
        observer_collective_cid: None,
        observer_region: Some("us-west".to_string()),
        observer_archetype: Some("tier-1-hub".to_string()),
        observer_compute_class: Some("rpi4-arm64".to_string()),
        signature: vec![0x01, 0x02, 0x03, 0x04],
    };
    let bytes = rmp_serde::to_vec(&obs).expect("encode");
    let decoded: Observation = rmp_serde::from_slice(&bytes).expect("decode");
    assert_eq!(decoded.observer_cid, obs.observer_cid);
    assert_eq!(decoded.log_offset, 42);
    assert_eq!(decoded.signature, vec![0x01, 0x02, 0x03, 0x04]);
    assert_eq!(decoded.subject_kind, Some("doorway".to_string()));
}

#[test]
fn deterministic_encoding_for_identical_values() {
    let obs = Observation {
        observer_cid: "agent:test".into(),
        log_cid: "blake3:test".into(),
        log_offset: 0,
        observed_at: 1715420400,
        seq: 0,
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
    };
    let bytes1 = rmp_serde::to_vec(&obs).unwrap();
    let bytes2 = rmp_serde::to_vec(&obs).unwrap();
    assert_eq!(
        bytes1, bytes2,
        "encoding must be deterministic for identical struct values"
    );
}

#[test]
fn canonical_signing_bytes_do_not_depend_on_signature_field() {
    let base = Observation {
        observer_cid: "agent:bafkrei".to_string(),
        log_cid: "blake3:abc123".to_string(),
        log_offset: 42,
        observed_at: 1715420400,
        seq: 100,
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        subject_cid: Some("doorway:bafkrei".to_string()),
        subject_kind: Some("doorway".to_string()),
        payload_json: r#"{"peer_count":12,"uptime_secs":86400}"#.to_string(),
        observer_household_cid: Some("household:bafkrei".to_string()),
        observer_collective_cid: None,
        observer_region: Some("us-west".to_string()),
        observer_archetype: Some("tier-1-hub".to_string()),
        observer_compute_class: Some("rpi4-arm64".to_string()),
        signature: vec![],
    };

    let with_empty_sig = base.canonical_signing_bytes();

    let with_dummy_sig = Observation {
        signature: vec![0xff; 64],
        ..base.clone()
    };
    let with_dummy_sig_bytes = with_dummy_sig.canonical_signing_bytes();

    let with_real_sig = Observation {
        signature: vec![0x01, 0x02, 0x03, 0x04],
        ..base.clone()
    };
    let with_real_sig_bytes = with_real_sig.canonical_signing_bytes();

    assert_eq!(
        with_empty_sig, with_dummy_sig_bytes,
        "canonical_signing_bytes must ignore the signature field"
    );
    assert_eq!(
        with_empty_sig, with_real_sig_bytes,
        "canonical_signing_bytes must ignore the signature field regardless of contents"
    );
}

#[test]
fn canonical_signing_bytes_change_when_meaningful_field_changes() {
    let base = Observation {
        observer_cid: "agent:bafkrei".to_string(),
        log_cid: "blake3:abc123".to_string(),
        log_offset: 42,
        observed_at: 1715420400,
        seq: 100,
        observation_kind: "infrastructure:doorway-heartbeat".to_string(),
        subject_cid: None,
        subject_kind: None,
        payload_json: "{}".to_string(),
        observer_household_cid: None,
        observer_collective_cid: None,
        observer_region: None,
        observer_archetype: None,
        observer_compute_class: None,
        signature: vec![],
    };

    let bytes_a = base.canonical_signing_bytes();
    let mut mutated = base.clone();
    mutated.log_offset = 43;
    let bytes_b = mutated.canonical_signing_bytes();

    assert_ne!(
        bytes_a, bytes_b,
        "canonical_signing_bytes must change when a signed field changes"
    );
}
