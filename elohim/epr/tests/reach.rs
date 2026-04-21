use elohim_epr::Reach;
use serde_json;

#[test]
fn reach_serializes_lowercase() {
    for (variant, expected) in [
        (Reach::Commons, "\"commons\""),
        (Reach::Community, "\"community\""),
        (Reach::Collective, "\"collective\""),
        (Reach::Steward, "\"steward\""),
        (Reach::Private, "\"private\""),
    ] {
        let s = serde_json::to_string(&variant).unwrap();
        assert_eq!(s, expected);
        let r: Reach = serde_json::from_str(expected).unwrap();
        assert_eq!(r, variant);
    }
}

#[test]
fn reach_ordering() {
    // Design intent: commons is most-open, private is most-closed.
    assert!(Reach::Commons.openness() > Reach::Community.openness());
    assert!(Reach::Community.openness() > Reach::Collective.openness());
    assert!(Reach::Collective.openness() > Reach::Steward.openness());
    assert!(Reach::Steward.openness() > Reach::Private.openness());
}
