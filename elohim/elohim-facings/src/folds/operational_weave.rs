//! The operational facing's folds — per-shard → per-node → per-cluster "weave"
//! health, folded from shard-placement + custodian-capacity relations. Pure
//! (DB-free); the diesel rows live in elohim-storage and are mirrored before they
//! reach these folds (the §11 add-a-lens recipe, elohim/elohim-facings/CLAUDE.md).
//! Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use std::collections::BTreeMap;

use elohim_views::ComputeTriptych;
use elohim_views::PlacementGapView;
use elohim_views::RegionalDistributionView;

use crate::folds::resiliency::regional_distribution;
use crate::relation::HolderRow;

/// DB-free mirror of a custodian's capacity (custodian_metrics ⨝ system_metrics),
/// keyed by `agent_cid` (NOT a transport id). The diesel rows are mapped onto this
/// by the storage loader so the folds stay DB-free.
#[derive(Debug, Clone)]
pub struct CustodianRow {
    pub agent_cid: String,
    pub free: Option<u64>,
    pub used: Option<u64>,
    pub stewarded: Option<u64>,
}

/// The durable service posture a pantry has evidence to provide. `drawn` is
/// intentionally absent: it is a caller's transient working-set state, never a
/// custody commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CustodyClass {
    /// An authoritative commitment lookup found no active stewardship promise.
    /// This is distinct from `Option::None`, which remains unknown.
    None,
    Shelved,
    Stocked,
    StockedWarm,
}

/// Why this observer believes a named holder has a shard. The loader classifies
/// evidence at ingestion; a received peer assertion never becomes stronger by
/// being folded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CustodyEvidence {
    DirectPossession,
    VerifiedDraw,
    AcceptedPush,
    PeerAsserted,
    NotObserved,
    ObservedLost,
}

/// DB-free materialization of one observer's current fact about a shard holder.
///
/// `observed_custody_class` is populated only after the storage-side loader has
/// established both the relevant commitment and its evidence obligation.
/// `Some(CustodyClass::None)` means a complete commitment lookup measured no
/// active promise; `None` is an honest unknown, never an inferred zero. The
/// loader owns choosing the latest row for each `(observer, shard, holder)` tuple.
#[derive(Debug, Clone)]
pub struct CustodyObservationRow {
    pub observer_agent_cid: String,
    pub shard_cid: String,
    pub holder_agent_cid: String,
    pub observed_custody_class: Option<CustodyClass>,
    pub evidence: CustodyEvidence,
    pub observed_at: String,
    pub is_fresh: bool,
}

/// Counts current holder observations by custody class. `unknown` includes
/// missing or expired observations; fresh negative evidence remains visible as
/// `observed_lost`. **No unknown is ever silently promoted to a custody class** —
/// a stale or absent observation falls to `unknown`, never to `none`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CustodyClassCounts {
    pub none: usize,
    pub shelved: usize,
    pub stocked: usize,
    pub stocked_warm: usize,
    pub unknown: usize,
    pub observed_lost: usize,
}

/// Fold materialized, latest-per-holder observations into custody-class counts.
/// This does not derive a class from capacity, a draw, a peer announcement, or
/// a missing observation.
pub fn observed_custody_class_counts(rows: &[CustodyObservationRow]) -> CustodyClassCounts {
    let mut counts = CustodyClassCounts::default();
    for row in rows {
        if row.is_fresh && row.evidence == CustodyEvidence::ObservedLost {
            counts.observed_lost += 1;
            continue;
        }
        match (row.is_fresh, row.observed_custody_class) {
            (true, Some(CustodyClass::None)) => counts.none += 1,
            (true, Some(CustodyClass::Shelved)) => counts.shelved += 1,
            (true, Some(CustodyClass::Stocked)) => counts.stocked += 1,
            (true, Some(CustodyClass::StockedWarm)) => counts.stocked_warm += 1,
            _ => counts.unknown += 1,
        }
    }
    counts
}

/// Count fresh observations by evidence class. The buckets preserve what the
/// observer actually knows; callers must not sum them into an unlabeled holder
/// count.
pub fn fresh_evidence_counts(rows: &[CustodyObservationRow]) -> BTreeMap<CustodyEvidence, usize> {
    crate::fold::bucket_by(rows, |row| row.is_fresh.then_some(row.evidence))
        .into_iter()
        .map(|(evidence, rows)| (evidence, rows.len()))
        .collect()
}

pub fn node_capacity(c: &CustodianRow) -> ComputeTriptych {
    ComputeTriptych {
        free: c.free,
        used: c.used,
        stewarded: c.stewarded,
    }
}

/// Per-field Option-sum rollup across nodes. A field is `Some` iff at least one node
/// reported it; `None`s are skipped (an unsampled node doesn't zero the cluster).
pub fn aggregate_capacity(rows: &[CustodianRow]) -> ComputeTriptych {
    fn sum(vals: impl Iterator<Item = Option<u64>>) -> Option<u64> {
        let mut acc: Option<u64> = None;
        for v in vals.flatten() {
            acc = Some(acc.unwrap_or(0) + v);
        }
        acc
    }
    ComputeTriptych {
        free: sum(rows.iter().map(|r| r.free)),
        used: sum(rows.iter().map(|r| r.used)),
        stewarded: sum(rows.iter().map(|r| r.stewarded)),
    }
}

/// Count the open placement gaps (one row = one under-replicated shard).
pub fn placement_gap_count(gaps: &[PlacementGapView]) -> usize {
    gaps.len()
}

/// Group the gaps by `gap_kind`, counting per kind. `BTreeMap` → deterministic
/// wire order if a caller serializes it.
pub fn gaps_by_kind(gaps: &[PlacementGapView]) -> BTreeMap<String, usize> {
    crate::fold::bucket_by(gaps, |g| Some(g.gap_kind.clone()))
        .into_iter()
        .map(|(kind, rows)| (kind, rows.len()))
        .collect()
}

/// Mean RS contract-coverage across the open gaps (`PlacementGapView.contract_coverage`
/// already = achieved/requested). Empty ⇒ 1.0 (no gaps means nothing under-covered).
pub fn rs_coverage(gaps: &[PlacementGapView]) -> f32 {
    if gaps.is_empty() {
        return 1.0;
    }
    gaps.iter().map(|g| g.contract_coverage).sum::<f32>() / gaps.len() as f32
}

/// Region occupancy (Slice 4): distinct stewarding hubs bucketed by region, but
/// VIEWER-AGNOSTIC — the cluster/operational weave has no single viewer, so there
/// is no local/regional split. Reuses the resiliency facing's `regional_distribution`
/// with `viewer_region = None` (the deduped re-use the charter §44 names): a hub
/// with a known region buckets `global`, a region-less hub buckets `unknown`.
/// Charter: 2026-06-19-operational-weave-facing-lens-design.md §44/§48.
///
/// NOTE: the sibling `tier_occupancy -> RiskTierDistribution` lens is deliberately
/// NOT built here — the holder relation carries no risk-tier dimension and
/// `RiskTierDistribution` is a spatial-vulnerability type; per the not-selected
/// contract `WeaveView.tier_occupancy` stays absent (never a fabricated tier)
/// until a risk-tier source/derivation is designed.
pub fn region_occupancy(holders: &[HolderRow]) -> RegionalDistributionView {
    regional_distribution(holders, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap(kind: &str) -> PlacementGapView {
        PlacementGapView {
            id: format!("gap-{kind}"),
            content_id: "c".into(),
            shard_hash: "s".into(),
            requested_steward_count: 3,
            achieved_steward_count: 1,
            contract_coverage: 0.33,
            gap_kind: kind.into(),
            first_seen_at: "t0".into(),
            last_seen_at: "t1".into(),
        }
    }

    #[test]
    fn placement_gap_count_counts_rows() {
        let gaps = vec![
            gap("under_replicated"),
            gap("unplaced"),
            gap("under_replicated"),
        ];
        assert_eq!(placement_gap_count(&gaps), 3);
        assert_eq!(placement_gap_count(&[]), 0);
    }

    #[test]
    fn gaps_by_kind_buckets_deterministically() {
        let gaps = vec![
            gap("under_replicated"),
            gap("unplaced"),
            gap("under_replicated"),
        ];
        let by_kind = gaps_by_kind(&gaps);
        assert_eq!(by_kind.get("under_replicated"), Some(&2));
        assert_eq!(by_kind.get("unplaced"), Some(&1));
        // BTreeMap iteration is sorted → first key is "under_replicated" < "unplaced"? no: 'un' tie,
        // 'd' < 'p' so "under_replicated" sorts first — deterministic regardless.
        let keys: Vec<&String> = by_kind.keys().collect();
        assert_eq!(keys, vec!["under_replicated", "unplaced"]);
    }

    #[test]
    fn rs_coverage_is_mean_contract_coverage_and_empty_is_full() {
        let mut a = gap("under_replicated");
        a.contract_coverage = 0.5;
        let mut b = gap("under_replicated");
        b.contract_coverage = 1.0;
        assert!((super::rs_coverage(&[a, b]) - 0.75).abs() < 1e-6);
        assert_eq!(super::rs_coverage(&[]), 1.0, "no gaps ⇒ fully covered");
    }

    fn cust(free: Option<u64>, used: Option<u64>, stewarded: Option<u64>) -> super::CustodianRow {
        super::CustodianRow {
            agent_cid: "a".into(),
            free,
            used,
            stewarded,
        }
    }

    #[test]
    fn aggregate_capacity_sums_per_field_skipping_none() {
        let rows = vec![cust(Some(10), Some(5), None), cust(Some(20), None, Some(3))];
        let t = super::aggregate_capacity(&rows);
        assert_eq!(t.free, Some(30));
        assert_eq!(t.used, Some(5)); // second row's used is None → skipped
        assert_eq!(t.stewarded, Some(3));
    }

    #[test]
    fn aggregate_capacity_all_none_is_none() {
        let t = super::aggregate_capacity(&[cust(None, None, None)]);
        assert!(t.free.is_none() && t.used.is_none() && t.stewarded.is_none());
    }

    #[test]
    fn region_occupancy_buckets_distinct_hubs_viewer_agnostic() {
        let h = |hub: &str, agent: &str, region: Option<&str>| super::HolderRow {
            hub_id: Some(hub.into()),
            agent_id: agent.into(),
            region: region.map(str::to_string),
        };
        let holders = vec![
            h("hub-a", "x", Some("us-east")),
            h("hub-a", "y", Some("us-east")), // same hub — deduped
            h("hub-b", "z", None),            // no region → unknown
        ];
        let d = super::region_occupancy(&holders);
        assert_eq!(d.global, 1, "hub-a known region, no viewer → global");
        assert_eq!(d.unknown, 1, "hub-b no region → unknown");
        assert_eq!(
            d.local + d.regional,
            0,
            "no viewer → no local/regional split"
        );
    }

    fn observation(
        class: Option<CustodyClass>,
        evidence: CustodyEvidence,
        is_fresh: bool,
    ) -> CustodyObservationRow {
        CustodyObservationRow {
            observer_agent_cid: "observer".into(),
            shard_cid: "bafkrei-shard".into(),
            holder_agent_cid: "holder".into(),
            observed_custody_class: class,
            evidence,
            observed_at: "2026-07-25T00:00:00Z".into(),
            is_fresh,
        }
    }

    #[test]
    fn observed_custody_counts_preserve_unknown_without_inference() {
        let rows = vec![
            observation(
                Some(CustodyClass::Shelved),
                CustodyEvidence::DirectPossession,
                true,
            ),
            observation(
                Some(CustodyClass::Stocked),
                CustodyEvidence::VerifiedDraw,
                true,
            ),
            observation(
                Some(CustodyClass::StockedWarm),
                CustodyEvidence::AcceptedPush,
                true,
            ),
            observation(Some(CustodyClass::None), CustodyEvidence::NotObserved, true),
            observation(None, CustodyEvidence::PeerAsserted, true),
            observation(
                Some(CustodyClass::Stocked),
                CustodyEvidence::VerifiedDraw,
                false,
            ),
            observation(None, CustodyEvidence::ObservedLost, true),
        ];

        assert_eq!(
            observed_custody_class_counts(&rows),
            CustodyClassCounts {
                none: 1,
                shelved: 1,
                stocked: 1,
                stocked_warm: 1,
                unknown: 2,
                observed_lost: 1,
            },
        );
    }

    #[test]
    fn fresh_evidence_counts_excludes_expired_observations() {
        let rows = vec![
            observation(
                Some(CustodyClass::Stocked),
                CustodyEvidence::VerifiedDraw,
                true,
            ),
            observation(
                Some(CustodyClass::Stocked),
                CustodyEvidence::VerifiedDraw,
                false,
            ),
            observation(None, CustodyEvidence::PeerAsserted, true),
        ];

        let counts = fresh_evidence_counts(&rows);
        assert_eq!(counts.get(&CustodyEvidence::VerifiedDraw), Some(&1));
        assert_eq!(counts.get(&CustodyEvidence::PeerAsserted), Some(&1));
        assert_eq!(counts.len(), 2);
    }
}
