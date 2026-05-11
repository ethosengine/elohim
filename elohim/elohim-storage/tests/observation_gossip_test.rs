use elohim_storage::p2p::observation_gossip::{observation_topic, CursorAnnouncement};

#[test]
fn observation_topic_for_infrastructure_kind() {
    assert_eq!(
        observation_topic("infrastructure:doorway-heartbeat"),
        "elohim/observations/infrastructure"
    );
}

#[test]
fn observation_topic_for_lamad_kind() {
    assert_eq!(
        observation_topic("lamad:mastery-check-result"),
        "elohim/observations/lamad"
    );
}

#[test]
fn observation_topic_falls_back_to_unknown_for_malformed_kind() {
    let topic = observation_topic("no-colon-here");
    assert_eq!(topic, "elohim/observations/no-colon-here");
}

#[test]
fn cursor_announcement_msgpack_under_512_bytes() {
    let ann = CursorAnnouncement {
        observer_cid: "agent:bafyreigdyrzt5sfbtgnnwphhbofgw57x3sjyiyq2r4f4tymqxznkjkzgfa".into(),
        kind: "infrastructure:doorway-heartbeat".into(),
        log_cid: "blake3:abc123def456".into(),
        latest_offset: 12345,
        subject_cid: Some(
            "doorway:bafyreigdyrzt5sfbtgnnwphhbofgw57x3sjyiyq2r4f4tymqxznkjkzgfa".into(),
        ),
        observed_at_window: 1715420400,
    };
    let bytes = rmp_serde::to_vec(&ann).unwrap();
    assert!(bytes.len() < 512, "announcement was {} bytes", bytes.len());
}

#[test]
fn cursor_announcement_roundtrips_msgpack() {
    let ann = CursorAnnouncement {
        observer_cid: "agent:test".into(),
        kind: "infrastructure:blob-served".into(),
        log_cid: "blake3:xyz".into(),
        latest_offset: 7,
        subject_cid: None,
        observed_at_window: 0,
    };
    let bytes = rmp_serde::to_vec(&ann).unwrap();
    let decoded: CursorAnnouncement = rmp_serde::from_slice(&bytes).unwrap();
    assert_eq!(decoded.observer_cid, ann.observer_cid);
    assert_eq!(decoded.latest_offset, 7);
    assert!(decoded.subject_cid.is_none());
}
