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

use crate::graph::engine::{GraphEngine, GraphError};
use crate::views::{RegionalDistributionView, ResilienceSnapshotView};
use cozo::DataValue;

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
    let stewarding_count = steward_result.rows.len() as i32;

    // Count stewards that also have MEMBER_OF edges (commitment-backed).
    let committed_result = engine.run_script(
        r#"?[steward_cid] :=
            *epr_edge{from_cid: $cid, to_cid: steward_cid, rel_type: 'STEWARDS'},
            *epr_edge{from_cid: steward_cid, to_cid: _collective, rel_type: 'MEMBER_OF'}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let commitment_backed_count = committed_result.rows.len() as i32;

    Ok(ResilienceSnapshotView {
        content_id: cid.to_string(),
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
    })
}
