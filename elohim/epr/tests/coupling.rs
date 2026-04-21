use cid::Cid;
use elohim_epr::kind::CouplingLeg;
use elohim_epr::Coupling;

fn test_cid(b: u8) -> Cid {
    elohim_epr::cid::compute_cid(&[b])
}

#[test]
fn empty_coupling() {
    let c = Coupling::default();
    assert!(c.knowledge.is_none());
    assert!(c.value.is_none());
    assert!(c.governance.is_none());
    assert!(!c.has(CouplingLeg::Knowledge));
}

#[test]
fn set_and_check_legs() {
    let k = test_cid(1);
    let v = test_cid(2);
    let c = Coupling {
        knowledge: Some(k),
        value: Some(v),
        governance: None,
    };
    assert!(c.has(CouplingLeg::Knowledge));
    assert!(c.has(CouplingLeg::Value));
    assert!(!c.has(CouplingLeg::Governance));
}

#[test]
fn json_roundtrip() {
    let k = test_cid(9);
    let c = Coupling { knowledge: Some(k), value: None, governance: None };
    let s = serde_json::to_string(&c).unwrap();
    let c2: Coupling = serde_json::from_str(&s).unwrap();
    assert_eq!(c, c2);
}
