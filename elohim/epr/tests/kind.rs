use elohim_epr::kind::{CouplingLeg, EprKind};

#[test]
fn kind_serializes_pascal() {
    for (variant, expected) in [
        (EprKind::Content, "\"Content\""),
        (EprKind::Agent, "\"Agent\""),
        (EprKind::Manifest, "\"Manifest\""),
        (EprKind::Claim, "\"Claim\""),
        (EprKind::Observation, "\"Observation\""),
        (EprKind::EconomicEvent, "\"EconomicEvent\""),
        (EprKind::Commitment, "\"Commitment\""),
        (EprKind::Attestation, "\"Attestation\""),
        (EprKind::Delegation, "\"Delegation\""),
    ] {
        let s = serde_json::to_string(&variant).unwrap();
        assert_eq!(s, expected);
        let k: EprKind = serde_json::from_str(expected).unwrap();
        assert_eq!(k, variant);
    }
}

#[test]
fn required_coupling_per_kind() {
    // Content requires all three legs
    let c = EprKind::Content.required_coupling();
    assert!(c.contains(&CouplingLeg::Knowledge));
    assert!(c.contains(&CouplingLeg::Value));
    assert!(c.contains(&CouplingLeg::Governance));

    // EconomicEvent requires value only
    assert_eq!(EprKind::EconomicEvent.required_coupling(), &[CouplingLeg::Value]);

    // Manifest requires governance only (self-describing)
    assert_eq!(EprKind::Manifest.required_coupling(), &[CouplingLeg::Governance]);

    // Agent requires governance only (self-describing)
    assert_eq!(EprKind::Agent.required_coupling(), &[CouplingLeg::Governance]);
}
