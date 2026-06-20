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
//! Count aggregates route through `super::coverage::build_stewarding_rollup` — the
//! shared helper that maps each steward CID onto a diversity keyspace slot and rolls
//! up into a `CoverageRollup`. `distribution` is the second caller; the helper lives
//! in `shefa::coverage` so both callers share one mapping (DRY, §3.1).
//!
//! - **Regression-identical counts**: `rollup.constituents.len() as i32` equals
//!   `rows.len() as i32` because `?[steward_cid]` is already DISTINCT and we map
//!   exactly one `ChildCoverage::readable` per row (no filtering, no dedup).
//! - **Descent preserved**: `rollup.descend()` returns the steward CIDs.
//! - **Deficit exposed**: `rollup.deficit.measure()` surfaced as `coverage_shortfall`.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::str_at;
use crate::views::{RegionalDistributionView, ResilienceSnapshotView};
use cozo::DataValue;
use elohim_facings::folds::resiliency::floor_for_tier;

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
    let steward_rollup = super::coverage::build_stewarding_rollup(cid, floor, &steward_cids);
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
    let committed_rollup = super::coverage::build_stewarding_rollup(cid, floor, &committed_cids);
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
