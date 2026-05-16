//! Shefa view builder — DistributionSummary / DistributionDetails (graph-backed branch)
//!
//! Derives steward counts + reach class from STEWARDS edges on a content atom.
//! Source of truth: CozoDB graph projection (Operational, Category C).
//!
//! Composition note: `replica_count`, `projector_count`, `diversity_hint`, and byte-level
//! fields require peer blob-inventory + doorway projector data (relational reads).
//! Those fields are zero-filled / None here as composition placeholders.

use crate::graph::engine::{GraphEngine, GraphError};
use crate::graph_views::data_value::*;
use crate::views::{
    DistributionDetails, DistributionSummary, DiversityHint, FetchSource, ReachClass, ReplicaHealth,
};
use cozo::DataValue;

/// Build a graph-backed `DistributionSummary` for the given content `cid`.
///
/// `replica_count` is derived from distinct STEWARDS edges. `reach_class` is
/// derived from the qahal reach field of the atom. Byte-level and projector fields
/// are zero-filled composition placeholders.
pub fn build_summary(engine: &GraphEngine, cid: &str) -> Result<DistributionSummary, GraphError> {
    // Count stewards via STEWARDS edges.
    let steward_result = engine.run_script(
        r#"?[steward_cid] :=
            *epr_edge{from_cid: $cid, to_cid: steward_cid, rel_type: 'STEWARDS'}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let replica_count = steward_result.rows.len() as u32;

    // Derive reach class from qahal.reach.
    let reach_result = engine.run_script(
        r#"?[reach] := *epr_qahal{cid: $cid, reach}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    let reach_str = reach_result
        .rows
        .first()
        .map(|r| opt_str_at(r, 0))
        .unwrap_or(None)
        .unwrap_or_default();
    let reach_class = reach_str_to_class(&reach_str);

    let replica_health = match replica_count {
        0 => ReplicaHealth::Critical,
        1..=2 => ReplicaHealth::AtRisk,
        _ => ReplicaHealth::Healthy,
    };

    Ok(DistributionSummary {
        replica_count,
        replica_target: 7, // Default target per RS(7,4) quilt spec
        replica_health,
        projector_count: 0, // Composition placeholder — requires doorway projector relational reads
        reach_class,
        diversity_hint: DiversityHint::None, // Composition placeholder — requires peer diversity data
        this_fetch_source: FetchSource::PeerDirect, // Default assumption
        last_verified_seconds: 0,            // Composition placeholder
        my_role: None,
        reciprocity_hint: None,
    })
}

/// Build a graph-backed `DistributionDetails` for the given content `cid`.
///
/// Wraps `build_summary` and provides empty replica/projector lists as composition
/// placeholders (these require peer blob-inventory reads in a follow-on sprint).
pub fn build_details(engine: &GraphEngine, cid: &str) -> Result<DistributionDetails, GraphError> {
    let summary = build_summary(engine, cid)?;

    Ok(DistributionDetails {
        summary,
        // Composition placeholders — require peer blob-inventory + peer identity relational reads
        replica_peers: vec![],
        projector_identities: vec![],
        placement_gaps: vec![],
        recent_projection_events: vec![],
        commitment_references: None,
    })
}

fn reach_str_to_class(reach: &str) -> ReachClass {
    match reach {
        "private" => ReachClass::Private,
        "intimate" => ReachClass::Intimate,
        "household" => ReachClass::Household,
        "neighborhood" => ReachClass::Neighborhood,
        "collective" => ReachClass::Collective,
        "community" => ReachClass::Community,
        "district" => ReachClass::District,
        _ => ReachClass::Public,
    }
}
