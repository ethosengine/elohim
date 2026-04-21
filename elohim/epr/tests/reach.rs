use elohim_epr::Reach;

#[test]
fn reach_serializes_lowercase() {
    for (variant, expected) in [
        (Reach::Private, "\"private\""),
        (Reach::SelfScope, "\"self\""),
        (Reach::Intimate, "\"intimate\""),
        (Reach::Trusted, "\"trusted\""),
        (Reach::Familiar, "\"familiar\""),
        (Reach::Community, "\"community\""),
        (Reach::Public, "\"public\""),
        (Reach::Commons, "\"commons\""),
    ] {
        let s = serde_json::to_string(&variant).unwrap();
        assert_eq!(s, expected);
        let r: Reach = serde_json::from_str(expected).unwrap();
        assert_eq!(r, variant);
    }
}

#[test]
fn reach_ordering() {
    // openness() returns monotonically increasing values: private=1, commons=8
    assert!(Reach::Commons.openness() > Reach::Public.openness());
    assert!(Reach::Public.openness() > Reach::Community.openness());
    assert!(Reach::Community.openness() > Reach::Familiar.openness());
    assert!(Reach::Familiar.openness() > Reach::Trusted.openness());
    assert!(Reach::Trusted.openness() > Reach::Intimate.openness());
    assert!(Reach::Intimate.openness() > Reach::SelfScope.openness());
    assert!(Reach::SelfScope.openness() > Reach::Private.openness());
}
