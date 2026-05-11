//! Phase 4 T8 — tests for peer_diversity::diversity_hint_for.

use elohim_storage::services::peer_diversity::{
    diversity_hint_for, diversity_hint_from_archetype_strs,
};
use elohim_storage::views::{DeviceArchetype, DiversityHint, ReplicaPeer};

fn replica(peer_id: &str, archetype: DeviceArchetype) -> ReplicaPeer {
    ReplicaPeer {
        peer_id: peer_id.into(),
        device_archetype: archetype,
        last_seen_seconds: 0,
        hop_hint: None,
        household_id: None,
        region_tier: None,
    }
}

#[test]
fn no_replicas_returns_none() {
    assert_eq!(diversity_hint_for(&[]), DiversityHint::None);
}

#[test]
fn single_archetype_returns_single_entry_list() {
    let replicas = vec![
        replica("p1", DeviceArchetype::Desktop),
        replica("p2", DeviceArchetype::Desktop),
    ];
    match diversity_hint_for(&replicas) {
        DiversityHint::HouseholdArchetypes(list) => {
            assert_eq!(list, vec!["desktop"]);
        }
        other => panic!("expected HouseholdArchetypes, got {:?}", other),
    }
}

#[test]
fn three_archetypes_returns_sorted_list() {
    let replicas = vec![
        replica("p1", DeviceArchetype::Desktop),
        replica("p2", DeviceArchetype::Mobile),
        replica("p3", DeviceArchetype::Steward),
        replica("p4", DeviceArchetype::Node),
    ];
    match diversity_hint_for(&replicas) {
        DiversityHint::HouseholdArchetypes(list) => {
            assert_eq!(list, vec!["desktop", "mobile", "node", "steward"]);
        }
        other => panic!("expected HouseholdArchetypes, got {:?}", other),
    }
}

#[test]
fn from_archetype_strs_deduplicates() {
    let archetypes = vec!["desktop".into(), "mobile".into(), "desktop".into()];
    match diversity_hint_from_archetype_strs(&archetypes) {
        DiversityHint::HouseholdArchetypes(list) => {
            assert_eq!(list, vec!["desktop", "mobile"]);
        }
        other => panic!("expected HouseholdArchetypes, got {:?}", other),
    }
}

#[test]
fn from_archetype_strs_empty_returns_none() {
    assert_eq!(diversity_hint_from_archetype_strs(&[]), DiversityHint::None);
}
