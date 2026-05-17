use chrono::{TimeZone, Utc};
use cid::Cid;
use elohim_epr::{
    cid::compute_cid, proof::AgentKeypair, validate_coupling, Coupling, Epr, EprKind, Reach,
};

fn cid(b: u8) -> Cid {
    compute_cid(&[b])
}

fn build(kind: EprKind, coupling: Coupling) -> Epr {
    let kp = AgentKeypair::from_secret(&[9u8; 32]).unwrap();
    Epr::builder()
        .kind(kind)
        .schema_ref(cid(1))
        .schema_key("x")
        .reach(Reach::Commons)
        .coupling(coupling)
        .claim(cid(5))
        .issued_at(Utc.with_ymd_and_hms(2026, 4, 21, 12, 0, 0).unwrap())
        .payload(vec![])
        .sign(&kp, cid(100))
        .unwrap()
}

#[test]
fn content_requires_all_three_legs() {
    let missing = Coupling {
        knowledge: Some(cid(2)),
        value: Some(cid(3)),
        governance: None,
    };
    let epr = build(EprKind::Content, missing);
    assert!(validate_coupling(&epr.envelope).is_err());

    let ok = Coupling {
        knowledge: Some(cid(2)),
        value: Some(cid(3)),
        governance: Some(cid(4)),
    };
    let epr = build(EprKind::Content, ok);
    assert!(validate_coupling(&epr.envelope).is_ok());
}

#[test]
fn economic_event_requires_value_only() {
    let value_only = Coupling {
        knowledge: None,
        value: Some(cid(3)),
        governance: None,
    };
    let epr = build(EprKind::EconomicEvent, value_only);
    assert!(validate_coupling(&epr.envelope).is_ok());

    let missing = Coupling::default();
    let epr = build(EprKind::EconomicEvent, missing);
    assert!(validate_coupling(&epr.envelope).is_err());
}

#[test]
fn manifest_requires_governance_only() {
    let gov_only = Coupling {
        knowledge: None,
        value: None,
        governance: Some(cid(4)),
    };
    let epr = build(EprKind::Manifest, gov_only);
    assert!(validate_coupling(&epr.envelope).is_ok());
}
