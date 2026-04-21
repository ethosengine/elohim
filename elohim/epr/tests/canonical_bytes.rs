use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, Coupling, Envelope, EprKind, Reach, Signature};

fn test_cid(b: u8) -> Cid {
    compute_cid(&[b])
}

fn env() -> Envelope {
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
fn canonical_bytes_excludes_cid_proof_superseded_by() {
    let a = env();
    let mut b = env();

    // Different cid, proof, superseded_by — canonical bytes must still match
    b.cid = test_cid(42);
    b.proof = Signature::ed25519(test_cid(7), vec![1u8; 64]);
    b.superseded_by = Some(test_cid(99));

    let payload = b"hello";
    let ba = a.canonical_bytes(payload).unwrap();
    let bb = b.canonical_bytes(payload).unwrap();
    assert_eq!(ba, bb, "cid/proof/supersededBy must not affect canonical bytes");
}

#[test]
fn canonical_bytes_changes_when_schema_key_changes() {
    let a = env();
    let mut b = env();
    b.schema_key = "lesson".into();

    let payload = b"hello";
    assert_ne!(
        a.canonical_bytes(payload).unwrap(),
        b.canonical_bytes(payload).unwrap()
    );
}

#[test]
fn canonical_bytes_changes_when_payload_changes() {
    let a = env();
    assert_ne!(
        a.canonical_bytes(b"foo").unwrap(),
        a.canonical_bytes(b"bar").unwrap()
    );
}

#[test]
fn canonical_bytes_includes_supersedes_when_present() {
    let mut a = env();
    a.supersedes = Some(test_cid(77));
    let b = env(); // supersedes: None

    // With vs without supersedes must differ
    assert_ne!(
        a.canonical_bytes(b"hello").unwrap(),
        b.canonical_bytes(b"hello").unwrap()
    );
}

#[test]
fn canonical_bytes_deterministic() {
    let a = env();
    let ba = a.canonical_bytes(b"stable").unwrap();
    let bb = a.canonical_bytes(b"stable").unwrap();
    assert_eq!(ba, bb);
}
