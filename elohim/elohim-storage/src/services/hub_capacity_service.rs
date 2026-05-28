//! Computes HubCapacityView per spec §7.2. Hub is a *role* (per
//! project_hub_archetype_abstraction memory); substrate stays kind-agnostic.
//! HubId defaults to peer_id for single-device (Computed kind); future
//! distinguishes dwelling_id / collective_id via binding tables.
//!
//! Sprint 3: `resolve_hub_members` is a single-device stub (hub_id IS the
//! peer_id); real hub-membership resolution (peer_identity_bindings /
//! household_id projection) lands in a follow-up sprint.

use diesel::sqlite::SqliteConnection;

use crate::error::StorageError;
use crate::services::peer_capacity_service::compute_peer_capacity;
use elohim_views::hub_capacity::{HubCapacityAggregate, HubCapacityView, HubKind};

pub fn compute_hub_capacity(
    conn: &mut SqliteConnection,
    hub_id: &str,
) -> Result<HubCapacityView, StorageError> {
    let member_peer_cids = resolve_hub_members(conn, hub_id)?;
    let hub_kind = classify_hub(conn, hub_id, &member_peer_cids);

    if member_peer_cids.is_empty() {
        return Ok(HubCapacityView {
            hub_id: hub_id.to_string(),
            hub_kind,
            display_label: None,
            member_device_count: 0,
            capacity: None,
        });
    }

    let mut capacity_aggregate = HubCapacityAggregate::default();
    for peer_cid in &member_peer_cids {
        let pv = compute_peer_capacity(conn, peer_cid)?;
        capacity_aggregate.total_raw_bytes += pv.total_raw_bytes;
        capacity_aggregate.pledges.dwelling_bytes += pv.pledges.dwelling_bytes;
        capacity_aggregate.pledges.collective_bytes += pv.pledges.collective_bytes;
        capacity_aggregate.pledges.commons_bytes += pv.pledges.commons_bytes;
        capacity_aggregate.pledges.total_pledged_bytes += pv.pledges.total_pledged_bytes;
        capacity_aggregate.actually_held.unique_shard_bytes += pv.actually_held.unique_shard_bytes;
        capacity_aggregate.actually_held.free_bytes_remaining += pv.actually_held.free_bytes_remaining;
    }

    Ok(HubCapacityView {
        hub_id: hub_id.to_string(),
        hub_kind,
        display_label: None,
        member_device_count: member_peer_cids.len() as i32,
        capacity: Some(capacity_aggregate),
    })
}

// --- Sprint-3 stubs: real hub-membership resolution lands in follow-up. ---

fn resolve_hub_members(_conn: &mut SqliteConnection, hub_id: &str) -> Result<Vec<String>, StorageError> {
    // Single-device fallback: hub_id IS the peer_id (Computed kind).
    // Follow-up: query peer_identity_bindings / household_id projection.
    Ok(vec![hub_id.to_string()])
}

fn classify_hub(_conn: &mut SqliteConnection, hub_id: &str, members: &[String]) -> HubKind {
    if hub_id.starts_with("dwelling:") {
        HubKind::Dwelling
    } else if hub_id.starts_with("collective:") {
        HubKind::Collective
    } else if members.len() <= 1 {
        HubKind::Computed
    } else {
        HubKind::Dwelling
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn single_device_hub_classified_as_computed() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_hub_capacity(&mut conn, "peer:solo").unwrap();
        assert_eq!(view.hub_kind, HubKind::Computed);
        assert_eq!(view.member_device_count, 1);
    }

    #[test]
    fn dwelling_prefix_hub_classified_as_dwelling() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_hub_capacity(&mut conn, "dwelling:smith-family").unwrap();
        assert_eq!(view.hub_kind, HubKind::Dwelling);
    }
}
