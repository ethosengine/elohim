#![cfg(feature = "p2p-iroh")]

use elohim_storage::p2p_iroh::peer_map::Plane;

#[test]
fn plane_has_observation_variant() {
    let _ = Plane::Observation;
}

#[test]
fn observation_plane_as_str_returns_consistent_name() {
    let obs = Plane::Observation;
    assert_eq!(obs.as_str(), "observation");
}

#[test]
fn observation_plane_parses_from_string() {
    let parsed = Plane::parse("observation");
    assert_eq!(parsed, Some(Plane::Observation));
}
