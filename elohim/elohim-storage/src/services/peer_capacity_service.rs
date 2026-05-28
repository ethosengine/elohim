//! Computes PeerCapacityView per spec §7.1. Read-only projection from
//! rea_commitments + peer_statuses (raw capacity) + peer_blob_inventory
//! (uniqueShardBytes) + constitutional_ratio_registry.
//!
//! Per-tier pledged aggregation: sum capacity_bytes of all active commitments
//! filtered by action and provider. Multi-reach blob accounting is enforced
//! at the uniqueShardBytes computation (dedup across shard CIDs).
//!
//! Sprint 3: the three data-source readers (raw bytes, per-tier pledges,
//! unique shard bytes) are explicit stubs returning zero/empty — the real
//! queries land in the encryption/collective-tier follow-up sprint. The view
//! shape + ratio-compliance computation are complete and testable now.

use diesel::sqlite::SqliteConnection;

use crate::error::StorageError;
use crate::services::constitutional_ratio_registry;
use elohim_views::peer_capacity::{
    ActuallyHeldView, CurrentRatiosView, EffectiveRatiosView, PeerCapacityView, PledgeByRecipientView,
    PledgesView, RatioComplianceView, RatioViolationView, Tier, ViolationKind,
};

pub fn compute_peer_capacity(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<PeerCapacityView, StorageError> {
    let total_raw_bytes = query_latest_total_raw_bytes(conn, peer_cid)?;
    let (pledged_dwelling, pledged_collective, pledged_commons, pledges_by_recipient) =
        aggregate_pledges_by_tier(conn, peer_cid)?;
    let unique_shard_bytes = compute_unique_shard_bytes(conn, peer_cid)?;
    let provenance = constitutional_ratio_registry::effective_ratios();
    let effective = provenance.ratios;
    let total_pledged = pledged_dwelling + pledged_collective + pledged_commons;

    let pledges = PledgesView {
        dwelling_bytes: pledged_dwelling,
        collective_bytes: pledged_collective,
        commons_bytes: pledged_commons,
        total_pledged_bytes: total_pledged,
        pledges_by_recipient,
    };

    let actually_held = ActuallyHeldView {
        unique_shard_bytes,
        free_bytes_remaining: total_raw_bytes as i64 - unique_shard_bytes as i64,
        fragmentation_estimate: 0.0,
    };

    let total_for_pct = total_raw_bytes.max(1);
    let current_dwelling_pct = ((pledged_dwelling * 100) / total_for_pct) as u8;
    let current_collective_pct = ((pledged_collective * 100) / total_for_pct) as u8;
    let current_commons_pct = ((pledged_commons * 100) / total_for_pct) as u8;
    let current_free_pct = 100u8
        .saturating_sub(current_dwelling_pct)
        .saturating_sub(current_collective_pct)
        .saturating_sub(current_commons_pct);

    let mut violations = Vec::new();
    if current_dwelling_pct > effective.dwelling_pct {
        violations.push(RatioViolationView {
            tier: Tier::Dwelling,
            violation_kind: ViolationKind::AboveCeiling,
            current_pct: current_dwelling_pct as i32,
            bound_pct: effective.dwelling_pct as i32,
        });
    }
    if current_free_pct < constitutional_ratio_registry::FREE_MIN_FLOOR_PCT {
        violations.push(RatioViolationView {
            tier: Tier::Free,
            violation_kind: ViolationKind::BelowFloor,
            current_pct: current_free_pct as i32,
            bound_pct: constitutional_ratio_registry::FREE_MIN_FLOOR_PCT as i32,
        });
    }

    let ratio_compliance = RatioComplianceView {
        effective_ratios: EffectiveRatiosView {
            commons_pct: effective.commons_pct as i32,
            dwelling_pct: effective.dwelling_pct as i32,
            collective_pct: effective.collective_pct as i32,
            free_pct: effective.free_pct as i32,
            manifest_cid: provenance.manifest_cid,
        },
        current_ratios: CurrentRatiosView {
            commons_pct: current_commons_pct as i32,
            dwelling_pct: current_dwelling_pct as i32,
            collective_pct: current_collective_pct as i32,
            free_pct: current_free_pct as i32,
        },
        compliant_with_donut: violations.is_empty(),
        violations,
    };

    Ok(PeerCapacityView {
        peer_cid: peer_cid.to_string(),
        computed_at: chrono::Utc::now().to_rfc3339(),
        total_raw_bytes,
        pledges,
        actually_held,
        ratio_compliance,
    })
}

// --- Sprint-3 stubs: real data-source queries land in the follow-up sprint. ---

fn query_latest_total_raw_bytes(_conn: &mut SqliteConnection, _peer_cid: &str) -> Result<u64, StorageError> {
    // Follow-up: pull latest infrastructure:system-sample (available_bytes) for this peer.
    Ok(0)
}

fn aggregate_pledges_by_tier(
    _conn: &mut SqliteConnection,
    _peer_cid: &str,
) -> Result<(u64, u64, u64, Vec<PledgeByRecipientView>), StorageError> {
    // Follow-up: query rea_commitments WHERE provider = peer_cid; sum capacity_bytes per tier.
    Ok((0, 0, 0, Vec::new()))
}

fn compute_unique_shard_bytes(_conn: &mut SqliteConnection, _peer_cid: &str) -> Result<u64, StorageError> {
    // Follow-up: sum DISTINCT blob sizes from peer_blob_inventory for this peer.
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    #[test]
    fn empty_peer_returns_zero_capacity() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:fresh").unwrap();
        assert_eq!(view.peer_cid, "peer:fresh");
        assert_eq!(view.total_raw_bytes, 0);
        assert_eq!(view.pledges.total_pledged_bytes, 0);
        assert_eq!(view.actually_held.unique_shard_bytes, 0);
    }

    #[test]
    fn ratio_compliance_reflects_effective_ratios() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:test").unwrap();
        let r = constitutional_ratio_registry::effective_ratios().ratios;
        assert_eq!(view.ratio_compliance.effective_ratios.commons_pct as u8, r.commons_pct);
        assert_eq!(view.ratio_compliance.effective_ratios.dwelling_pct as u8, r.dwelling_pct);
    }
}
