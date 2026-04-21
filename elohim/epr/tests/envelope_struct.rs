use cid::Cid;
use chrono::{TimeZone, Utc};
use elohim_epr::{cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

fn test_cid(b: u8) -> Cid { compute_cid(&[b]) }

fn sample_envelope() -> Envelope {
    Envelope {
        cid: test_cid(0),
        kind: EprKind::Content,
        schema_ref: test_cid(1),
        schema_key: "concept".into(),
        reach: Reach::Commons,
        coupling: Coupling {
            knowledge: Some(test_cid(2)),
            value: Some(test_cid(3)),
            governance: Some(test_cid(4)),
        },
        claims: vec![test_cid(5)],
        supersedes: None,
        superseded_by: None,
        issued_at: Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap(),
        proof: Signature::ed25519(test_cid(6), vec![0u8; 64]),
    }
}

#[test]
fn envelope_json_roundtrip() {
    let env = sample_envelope();
    let s = serde_json::to_string(&env).unwrap();
    let e2: Envelope = serde_json::from_str(&s).unwrap();
    assert_eq!(env, e2);
}

#[test]
fn envelope_serializes_camelcase() {
    let env = sample_envelope();
    let s = serde_json::to_string(&env).unwrap();
    assert!(s.contains("\"schemaRef\""), "expected camelCase schemaRef, got: {s}");
    assert!(s.contains("\"schemaKey\""));
    assert!(s.contains("\"issuedAt\""));
    assert!(!s.contains("\"schema_ref\""), "snake_case should not appear in wire: {s}");
}
