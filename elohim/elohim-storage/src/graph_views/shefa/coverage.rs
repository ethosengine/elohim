//! Shared CoverageRollup helper — both shefa first-callers route through here.
//!
//! `build_stewarding_rollup` is the single mapping from a set of steward CIDs onto the
//! diversity keyspace. Both `resilience_snapshot` and `distribution` call it; keeping it
//! here enforces the DRY invariant: one mapping, two callers (recursive-architecture §3.1).
//!
//! ## Regression invariant
//!
//! `rollup.constituents.len() as u32 == steward_cids.len() as u32`.
//! The Cozo `?[steward_cid]` projection is already DISTINCT, so
//! `constituents.len() == rows.len()` — the existing view numbers are byte-identical.

use crate::recursion::{ChildCoverage, CoverageDomain, CoverageRollup, CoverageSet};

/// Map `steward_cids` onto the diversity keyspace and roll up into a `CoverageRollup`.
///
/// The required coverage is `[0, target)` where `target = max(floor, len)` — so it is
/// always at least as large as the achieved set.
///
/// **Callers:**
/// - `resilience_snapshot::build` — floor = `floor_for_tier("standard")` (3)
/// - `distribution::build_summary` — target = RS(7,4) replica_target (7)
///
/// Regression invariant: `rollup.constituents.len() == steward_cids.len()` (each caller casts to its own field type — i32 for resilience_snapshot, u32 for distribution).
pub(crate) fn build_stewarding_rollup(
    content_cid: &str,
    target: u64,
    steward_cids: &[String],
) -> CoverageRollup {
    let len = steward_cids.len() as u64;
    let children: Vec<ChildCoverage> = steward_cids
        .iter()
        .enumerate()
        .map(|(i, cid)| {
            ChildCoverage::readable(cid.clone(), CoverageSet::interval(i as u64, i as u64 + 1))
        })
        .collect();
    CoverageRollup::rollup(
        content_cid,
        CoverageDomain::CorpusBytes,
        CoverageSet::full(target.max(len)),
        &children,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Golden regression: count preserved AND descent available.
    ///
    /// For a content with floor=3 and 2 achieved stewards, `constituents.len()` must
    /// equal `rows.len()` (2) and `deficit.measure()` must be 1 (one slot short).
    #[test]
    fn stewarding_aggregate_preserves_count_and_exposes_descent() {
        let stewards = vec!["steward-a".to_string(), "steward-c".to_string()]; // 2 of 3
        let target = 3u64;
        let rollup = build_stewarding_rollup("content-x", target, &stewards);

        // (1) Regression: the count the view exposes is unchanged.
        assert_eq!(
            rollup.constituents.len() as i32,
            2,
            "stewarding_count must equal rows.len() — byte-identical to pre-rollup"
        );

        // (2) Descent NEW: steward CIDs are preserved in the constituents
        // (sorted by rollup internals, so check via sort-stable set equality).
        let mut got = rollup.descend().to_vec();
        got.sort();
        let mut want = stewards.clone();
        want.sort();
        assert_eq!(got, want, "descend() must return the steward CIDs");

        // (3) The externality: 1 slot short of the floor — descent target for a consumer.
        assert_eq!(
            rollup.deficit.measure(),
            1,
            "deficit must be floor - achieved = 3 - 2 = 1"
        );
    }

    /// Floor exactly met: zero deficit.
    #[test]
    fn stewarding_aggregate_at_floor_has_zero_deficit() {
        let stewards = vec![
            "steward-a".to_string(),
            "steward-b".to_string(),
            "steward-c".to_string(),
        ];
        let rollup = build_stewarding_rollup("content-y", 3, &stewards);
        assert_eq!(rollup.constituents.len() as i32, 3);
        assert_eq!(
            rollup.deficit.measure(),
            0,
            "at floor: deficit must be zero"
        );
        assert!(rollup.is_covered(), "at floor: is_covered must be true");
    }

    /// Empty steward set: fully unmet — deficit == floor.
    #[test]
    fn stewarding_aggregate_empty_stewards_deficit_equals_floor() {
        let rollup = build_stewarding_rollup("content-z", 3, &[]);
        assert_eq!(rollup.constituents.len() as i32, 0);
        assert_eq!(
            rollup.deficit.measure(),
            3,
            "zero stewards: deficit must equal floor"
        );
    }

    /// When len > floor, required grows to len — no deficit even with surplus stewards.
    #[test]
    fn stewarding_aggregate_surplus_stewards_no_deficit() {
        let stewards: Vec<String> = (0..5).map(|i| format!("steward-{i}")).collect();
        let rollup = build_stewarding_rollup("content-w", 3, &stewards); // floor=3, len=5
        assert_eq!(rollup.constituents.len() as i32, 5);
        assert_eq!(
            rollup.deficit.measure(),
            0,
            "surplus stewards: required expands to len, so no deficit"
        );
    }

    /// Distribution target=7 (RS(7,4)): count identity at the distribution's specific target.
    ///
    /// This is the golden regression for `distribution::build_summary`: for 4 stewards
    /// against target=7, `constituents.len()` must equal 4 — byte-identical to the old
    /// `rows.len()`. `replica_count` and `replica_health` are downstream of this count
    /// (count identical → health identical by construction).
    #[test]
    fn distribution_target_7_count_identical() {
        let stewards: Vec<String> = (0..4).map(|i| format!("steward-{i}")).collect();
        let rollup = build_stewarding_rollup("content-dist", 7, &stewards);

        // Count byte-identical to old rows.len().
        assert_eq!(
            rollup.constituents.len() as u32,
            4,
            "replica_count must equal rows.len() — byte-identical to pre-rollup"
        );
        // With 4 < 7, shortfall is 3 (exposed for future consumption).
        assert_eq!(rollup.deficit.measure(), 3, "deficit = 7 - 4 = 3");
        // Health check: 4 stewards → Healthy (>2 is Healthy per distribution.rs match).
        // (Downstream of an identical count — included for documentary value.)
        let replica_count = rollup.constituents.len() as u32;
        let health = match replica_count {
            0 => "Critical",
            1..=2 => "AtRisk",
            _ => "Healthy",
        };
        assert_eq!(health, "Healthy");
    }

    /// Distribution: zero stewards → Critical (regression guard on the 0-case).
    #[test]
    fn distribution_zero_stewards_critical() {
        let rollup = build_stewarding_rollup("content-dist-zero", 7, &[]);
        assert_eq!(rollup.constituents.len() as u32, 0);
        assert_eq!(rollup.deficit.measure(), 7);
        let replica_count = rollup.constituents.len() as u32;
        let health = match replica_count {
            0 => "Critical",
            1..=2 => "AtRisk",
            _ => "Healthy",
        };
        assert_eq!(health, "Critical");
    }
}
