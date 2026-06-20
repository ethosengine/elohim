//! The operational facing's folds — per-shard → per-node → per-cluster "weave"
//! health, folded from shard-placement + custodian-capacity relations. Pure
//! (DB-free); the diesel rows live in elohim-storage and are mirrored before they
//! reach these folds (the §11 add-a-lens recipe, elohim/elohim-facings/CLAUDE.md).
//! Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use std::collections::BTreeMap;

use elohim_views::PlacementGapView;

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
}
