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
        let gaps = vec![gap("under_replicated"), gap("unplaced"), gap("under_replicated")];
        assert_eq!(placement_gap_count(&gaps), 3);
        assert_eq!(placement_gap_count(&[]), 0);
    }

    #[test]
    fn gaps_by_kind_buckets_deterministically() {
        let gaps = vec![gap("under_replicated"), gap("unplaced"), gap("under_replicated")];
        let by_kind = gaps_by_kind(&gaps);
        assert_eq!(by_kind.get("under_replicated"), Some(&2));
        assert_eq!(by_kind.get("unplaced"), Some(&1));
        // BTreeMap iteration is sorted → first key is "under_replicated" < "unplaced"? no: 'un' tie,
        // 'd' < 'p' so "under_replicated" sorts first — deterministic regardless.
        let keys: Vec<&String> = by_kind.keys().collect();
        assert_eq!(keys, vec!["under_replicated", "unplaced"]);
    }
}
