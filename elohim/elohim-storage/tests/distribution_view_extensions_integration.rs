//! Pure-function integration tests for the Sprint-3 distribution-view extensions:
//! `compute_projection_tier`, `replica_health_for` (OverReplicated branch),
//! and `compute_fault_domain_diversity`.

use elohim_storage::services::distribution_view::{
    compute_fault_domain_diversity, compute_projection_tier, replica_health_for,
};
use elohim_views::infrastructure::{DeviceArchetype, ProjectionTier, ReplicaHealth, ReplicaPeer};

fn peer(peer_id: &str, household: Option<&str>, region: Option<&str>) -> ReplicaPeer {
    ReplicaPeer {
        peer_id: peer_id.into(),
        device_archetype: DeviceArchetype::Node,
        last_seen_seconds: 0,
        hop_hint: None,
        household_id: household.map(|s| s.to_string()),
        region_tier: region.map(|s| s.to_string()),
        shards_held: None,
        shards_by_encoding: None,
    }
}

#[test]
fn projection_tier_local_for_few_projectors() {
    assert_eq!(compute_projection_tier(0, 0), ProjectionTier::Local);
    assert_eq!(compute_projection_tier(1, 0), ProjectionTier::Local);
    assert_eq!(compute_projection_tier(2, 5), ProjectionTier::Local);
    assert_eq!(compute_projection_tier(3, 1), ProjectionTier::Regional);
    assert_eq!(compute_projection_tier(3, 3), ProjectionTier::Global);
    assert_eq!(compute_projection_tier(5, 0), ProjectionTier::Regional);
    assert_eq!(compute_projection_tier(5, 2), ProjectionTier::Global);
    assert_eq!(compute_projection_tier(9, 0), ProjectionTier::Global);
    assert_eq!(compute_projection_tier(6, 0), ProjectionTier::Global);
}

#[test]
fn replica_health_over_replicated_at_2x() {
    // Exactly 2× target → OverReplicated
    assert_eq!(replica_health_for(20, 10), ReplicaHealth::OverReplicated);
    assert_eq!(replica_health_for(2, 1), ReplicaHealth::OverReplicated);
    // One below 2× target → Healthy (ratio 19/10 = 1.9 >= 0.85)
    assert_eq!(replica_health_for(19, 10), ReplicaHealth::Healthy);
    // Exactly at target → Healthy (ratio 1.0 >= 0.85)
    assert_eq!(replica_health_for(10, 10), ReplicaHealth::Healthy);
    // Existing thresholds unaffected
    assert_eq!(replica_health_for(7, 14), ReplicaHealth::AtRisk);
    assert_eq!(replica_health_for(3, 14), ReplicaHealth::Critical);
    assert_eq!(replica_health_for(0, 0), ReplicaHealth::Healthy);
}

#[test]
fn single_fault_domain_risk_when_all_one_household() {
    let peers = vec![
        peer("p1", Some("hub:A"), Some("us-east")),
        peer("p2", Some("hub:A"), Some("us-east")),
    ];
    let d = compute_fault_domain_diversity(&peers);
    assert_eq!(d.distinct_household_count, 1);
    assert_eq!(d.distinct_region_count, 1);
    assert!(d.single_fault_domain_risk);
    assert_eq!(d.distinct_collective_count, 0);
    assert!(d.fault_modes_evaluated.contains(&"household".to_string()));

    let diverse = vec![
        peer("p1", Some("hub:A"), Some("us-east")),
        peer("p2", Some("hub:B"), Some("eu-west")),
    ];
    let d2 = compute_fault_domain_diversity(&diverse);
    assert_eq!(d2.distinct_household_count, 2);
    assert_eq!(d2.distinct_region_count, 2);
    assert!(!d2.single_fault_domain_risk);

    // Zero peers → single_fault_domain_risk true (0 <= 1)
    let d3 = compute_fault_domain_diversity(&[]);
    assert_eq!(d3.distinct_household_count, 0);
    assert!(d3.single_fault_domain_risk);
}
