//! ## Source of Truth
//!
//! Helper API (Category C operational). Pure function over replica
//! peer set; archetype field is itself an operational projection of
//! AgentPeerBinding A2 link metadata. No persistence.

use std::collections::HashSet;

use crate::views::{DeviceArchetype, DiversityHint, ReplicaPeer};

/// Compute a `DiversityHint` for a replica peer set based on their device archetypes.
///
/// - 0 replicas → `DiversityHint::None`
/// - 1+ replicas → `DiversityHint::HouseholdArchetypes(distinct_archetype_names)`
///   where the list is sorted for deterministic output.
pub fn diversity_hint_for(replicas: &[ReplicaPeer]) -> DiversityHint {
    if replicas.is_empty() {
        return DiversityHint::None;
    }

    let archetypes: HashSet<String> = replicas
        .iter()
        .map(|r| archetype_to_str(&r.device_archetype).to_string())
        .collect();

    if archetypes.is_empty() {
        return DiversityHint::None;
    }

    let mut archetype_list: Vec<String> = archetypes.into_iter().collect();
    archetype_list.sort();
    DiversityHint::HouseholdArchetypes(archetype_list)
}

/// Compute a `DiversityHint` from a slice of archetype strings (already parsed).
///
/// Used by `distribution_view.rs` when it has archetype strings from SQL
/// but hasn't loaded full `ReplicaPeer` structs.
pub fn diversity_hint_from_archetype_strs(archetypes: &[String]) -> DiversityHint {
    if archetypes.is_empty() {
        return DiversityHint::None;
    }

    let unique: HashSet<&str> = archetypes.iter().map(String::as_str).collect();
    let mut list: Vec<String> = unique.into_iter().map(String::from).collect();
    list.sort();
    DiversityHint::HouseholdArchetypes(list)
}

fn archetype_to_str(a: &DeviceArchetype) -> &'static str {
    match a {
        DeviceArchetype::Node => "node",
        DeviceArchetype::Desktop => "desktop",
        DeviceArchetype::Mobile => "mobile",
        DeviceArchetype::Steward => "steward",
    }
}
