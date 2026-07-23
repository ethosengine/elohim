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
    DistributionDetails, DistributionSummary, DiversityHint, FaultDomainDiversity, FetchSource,
    ProjectionTier, ReachClass, ReplicaHealth,
};
use cozo::DataValue;

/// Build a graph-backed `DistributionSummary` for the given content `cid`.
///
/// `replica_count` is derived from distinct STEWARDS edges via `CoverageRollup`
/// (second shefa first-caller, recursive-architecture §3.1). `reach_class` is
/// derived from the qahal reach field of the atom. Byte-level and projector fields
/// are zero-filled composition placeholders.
///
/// ## Regression invariant
///
/// `rollup.constituents.len() as u32 == rows.len() as u32` — the Cozo
/// `?[steward_cid]` projection is already DISTINCT, so `constituents.len() ==
/// rows.len()`. `replica_count` and `replica_health` are byte-identical to the
/// pre-rollup implementation.
pub fn build_summary(engine: &GraphEngine, cid: &str) -> Result<DistributionSummary, GraphError> {
    // Count stewards via STEWARDS edges.
    let steward_result = engine.run_script(
        r#"?[steward_cid] :=
            *epr_edge{from_cid: $cid, to_cid: steward_cid, rel_type: 'STEWARDS'}"#,
        &[("cid", DataValue::from(cid))],
    )?;
    // Extract steward CIDs and route through the shared rollup helper.
    // target = replica_target = 7 (RS(7,4) quilt spec).
    let steward_cids: Vec<String> = steward_result.rows.iter().map(|r| str_at(r, 0)).collect();
    let rollup = super::coverage::build_stewarding_rollup(cid, 7, &steward_cids);
    let replica_count = rollup.constituents.len() as u32;

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
        projection_tier: ProjectionTier::Local, // T15: computed
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
        replication_commitments: Vec::new(), // T15: computed
        fault_domain_diversity: FaultDomainDiversity::default(), // T15: computed
    })
}

/// Schema-8 vocabulary + legacy kebab-8-era aliases — delegates to
/// `services::distribution_view::parse_reach_class` (the single source of
/// truth for the mapping table) rather than duplicating the match arms.
/// Unknown/empty strings (including the no-Cozo-row composition-placeholder
/// case) now fall back to `ReachClass::Private` — the conservative,
/// most-closed reading — matching the DB-backed sibling's fallback. This
/// deliberately flips the prior `Public` (most-open) default; see slice-3
/// task-5 for the rationale.
fn reach_str_to_class(reach: &str) -> ReachClass {
    crate::services::distribution_view::parse_reach_class(reach).unwrap_or(ReachClass::Private)
}
