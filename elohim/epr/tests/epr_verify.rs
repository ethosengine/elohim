use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach,
};

fn cid(b: u8) -> Cid {
    compute_cid(&[b])
}

fn make_valid() -> (AgentKeypair, Epr) {
    let kp = AgentKeypair::from_secret(&[7u8; 32]).unwrap();
    let signer = cid(100);
    let epr = Epr::builder()
        .kind(EprKind::Content)
        .schema_ref(cid(1))
        .schema_key("concept")
        .reach(Reach::Commons)
        .coupling(Coupling {
            knowledge: Some(cid(2)),
            value: Some(cid(3)),
            governance: Some(cid(4)),
        })
        .claim(cid(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(b"hi".to_vec())
        .sign(&kp, signer)
        .unwrap();
    (kp, epr)
}

#[test]
fn verify_with_correct_key_passes() {
    let (kp, epr) = make_valid();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_ok());
}

#[test]
fn verify_with_wrong_key_fails() {
    let (_, epr) = make_valid();
    let other = AgentKeypair::from_secret(&[8u8; 32]).unwrap();
    assert!(epr.verify_with_key(&other.public_key_bytes()).is_err());
}

#[test]
fn verify_fails_on_tampered_payload() {
    let (kp, mut epr) = make_valid();
    epr.payload = b"tampered".to_vec();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}

#[test]
fn verify_fails_on_tampered_envelope() {
    let (kp, mut epr) = make_valid();
    epr.envelope.schema_key = "lesson".into();
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}

#[test]
fn verify_checks_cid_match() {
    let (kp, mut epr) = make_valid();
    epr.envelope.cid = cid(255);
    assert!(epr.verify_with_key(&kp.public_key_bytes()).is_err());
}
