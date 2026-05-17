use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{cid::compute_cid, proof::AgentKeypair, Coupling, Epr, EprKind, Reach};

fn cid(b: u8) -> Cid {
    compute_cid(&[b])
}

#[test]
fn builder_produces_valid_epr() {
    let mut rng = rand::thread_rng();
    let kp = AgentKeypair::generate(&mut rng);
    let agent_cid = cid(100);

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
        .payload(b"hello world".to_vec())
        .sign(&kp, agent_cid)
        .expect("sign");

    assert_eq!(epr.envelope.proof.signer, cid(100));
    assert_eq!(epr.payload, b"hello world");
    assert!(!epr.envelope.cid.to_string().is_empty());
}

#[test]
fn builder_cid_is_stable_for_same_inputs() {
    // ed25519 signatures are deterministic in ed25519-dalek, so the full EPR is deterministic.
    // Use a fixed 32-byte seed so the keypair is identical across both constructions.
    let kp1 = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let kp2 = AgentKeypair::from_secret(&[42u8; 32]).unwrap();
    let agent_cid = cid(100);

    let mk = |kp: &AgentKeypair| {
        Epr::builder()
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
            .payload(b"hello world".to_vec())
            .sign(kp, cid(100))
            .unwrap()
    };

    let a = mk(&kp1);
    let b = mk(&kp2);
    assert_eq!(a.envelope.cid, b.envelope.cid);
    assert_eq!(a.envelope.proof.signature, b.envelope.proof.signature);
    let _ = agent_cid; // suppress unused warning
}
