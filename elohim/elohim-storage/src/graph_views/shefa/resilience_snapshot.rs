//! Shefa view builder — ResilienceSnapshotView (graph-backed branch)
//!
//! Composes STEWARDS + MEMBER_OF edges to derive collective resilience counts
//! for a content atom.
//! Source of truth: CozoDB graph projection (Operational, Category C).
//!
//! Composition note: `diversity_score`, `regional_distribution`, `placement_gaps`,
//! and `protection_status` require placement-gap relational data. Zero-filled here
//! as composition placeholders. The graph contribution is the stewarding collective
//! count from STEWARDS edges.
//!
//! ## CoverageRollup first-caller (recursive-architecture §2.1)
//!
//! The two `rows.len()` aggregates that previously erased descent now route through
//! `build_stewarding_rollup` — a pure helper that maps each steward CID onto a
//! diversity keyspace slot and rolls up into a `CoverageRollup`. This means:
//!
//! - **Regression-identical counts**: `rollup.constituents.len() as i32` equals
//!   `rows.len() as i32` because `?[steward_cid]` is already DISTINCT and we map
//!   exactly one `ChildCoverage::readable` per row (no filtering, no dedup).
//! - **Descent preserved**: `rollup.descend()` returns the steward CIDs; an AI
//!   (or another layer) can walk from this aggregate down to each held atom.
//! - **Deficit exposed**: `rollup.deficit.measure()` is the count of additional
//!   distinct-collective slots needed to meet the floor — surfaced as
//!   `coverage_shortfall` on the view (not-selected-field contract: `None` on the
//!   relational path, `Some(n)` here).

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::str_at;
use crate::recursion::{ChildCoverage, CoverageDomain, CoverageRollup, CoverageSet};
use crate::views::{RegionalDistributionView, ResilienceSnapshotView};
use cozo::DataValue;
use elohim_facings::folds::resiliency::floor_for_tier;

/// Map `steward_cids` onto the diversity keyspace and roll up into a
/// `CoverageRollup`. The required coverage is `[0, target)` where `target =
/// max(floor, len)` — so it is always at least as large as the achieved set.
///
/// Regression invariant: `rollup.constituents.len() as i32 == steward_cids.len() as i32`.
/// The Cozo `?[steward_cid]` projection is already DISTINCT, so
/// `constituents.len() == rows.len()` — the existing view numbers are byte-identical.
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

/// Build a graph-backed `ResilienceSnapshotView` for the given content `cid`.
///
/// Counts distinct collectives reachable via STEWARDS edges from the content node.
/// Steward agents that have MEMBER_OF edges contribute to `commitment_backed_collectives`.
/// Regional distribution, placement gaps, and diversity scores are zero-filled
/// composition placeholders requiring relational placement-gap data.
pub fn build(engine: &GraphEngine, cid: &str) -> Result<ResilienceSnapshotView, GraphError> {
    // Count distinct steward nodes reachable via STEWARDS edges.
    let steward_result = engine.run_script(
        r#"?[steward_cid] :=
            *epr_edge{from_cid: $cid, to_cid: steward_cid, rel_type: 'STEWARDS'}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    // Extract steward CIDs from Cozo rows (column 0 is steward_cid).
    let steward_cids: Vec<String> = steward_result.rows.iter().map(|row| str_at(row, 0)).collect();

    // Diversity floor for the "standard" tier (default when undeclared). Returns i32=3.
    let floor = floor_for_tier("standard") as u64;

    // Roll up: counts route through the primitive, descent preserved.
    let steward_rollup = build_stewarding_rollup(cid, floor, &steward_cids);
    let stewarding_count = steward_rollup.constituents.len() as i32;

    // Coverage shortfall: how many additional slots needed to meet the floor.
    let shortfall = steward_rollup.deficit.measure() as u32;

    // Count stewards that also have MEMBER_OF edges (commitment-backed).
    let committed_result = engine.run_script(
        r#"?[steward_cid] :=
            *epr_edge{from_cid: $cid, to_cid: steward_cid, rel_type: 'STEWARDS'},
            *epr_edge{from_cid: steward_cid, to_cid: _collective, rel_type: 'MEMBER_OF'}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    // Extract committed CIDs (same pattern — column 0).
    let committed_cids: Vec<String> = committed_result
        .rows
        .iter()
        .map(|row| str_at(row, 0))
        .collect();
    // Roll up the commitment-backed set (floor is same — parallel primitive).
    let committed_rollup = build_stewarding_rollup(cid, floor, &committed_cids);
    let commitment_backed_count = committed_rollup.constituents.len() as i32;

    Ok(ResilienceSnapshotView {
        content_id: cid.to_string(),
        // Graph-backed branch measures STEWARDS edges directly: an atom with
        // zero edges is honestly unmeasured (no distribution-plane entry),
        // one with edges is measured.
        distribution_state: if stewarding_count > 0 {
            "measured".to_string()
        } else {
            "unmeasured".to_string()
        },
        stewarding_collectives: stewarding_count,
        commitment_backed_collectives: commitment_backed_count,
        // Composition placeholders — require relational placement-gap + peer diversity reads
        diversity_score: 0.0,
        regional_distribution: RegionalDistributionView {
            local: 0,
            regional: 0,
            global: 0,
            unknown: stewarding_count,
        },
        placement_gaps: vec![],
        protection_status: if stewarding_count >= 3 {
            "protected".to_string()
        } else if stewarding_count > 0 {
            "partial".to_string()
        } else {
            "at-risk".to_string()
        },
        reciprocating_collectives: None,
        details: None,
        // The felt projection is computed in the relational `snapshot()` path
        // (it needs the collectives label join + placement gaps). The graph
        // branch leaves it None rather than emit an un-labeled felt block.
        felt_status: None,
        // Descent: how many additional distinct-collective slots short of the floor.
        // Derived from steward_rollup.deficit — the genuine cure surfaced to callers.
        coverage_shortfall: Some(shortfall),
    })
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
        assert_eq!(rollup.deficit.measure(), 0, "at floor: deficit must be zero");
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
}
