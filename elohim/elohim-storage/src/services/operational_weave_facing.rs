//! Operational-weave facing — impure loaders + the two adapters that project the
//! pure folds (elohim_facings::folds::operational_weave) as a typed WeaveView AND
//! as Prometheus gauges. The fold returns numbers; the adapter emits both wire
//! shapes. Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Double, Nullable};
use elohim_facings::folds::operational_weave::{
    aggregate_capacity, placement_gap_count, region_occupancy, rs_coverage, CustodianRow,
};
use elohim_facings::relation::HolderRow;
use elohim_views::{CustodianStorageMetricsView, PlacementGapView, WeaveView};

use crate::db::models::CustodianMetrics;

/// Load all custodian rows and fold them into the DB-free `CustodianRow` mirror
/// struct that the pure `node_capacity` / `aggregate_capacity` folds consume.
///
/// ## Capacity fields
/// - `free` / `used`: parsed from `storage_json` (CustodianStorageMetricsView).
///   A parse failure for an individual row yields `None` for that node's fields
///   (unsampled-node semantics — does NOT zero the cluster aggregate).
/// - `stewarded`: SUM of `resource_quantity_value` from `rea_commitments` where
///   `action='custody-blob'` and `provider=custodian_id`. Mirrors
///   `reciprocity_view::aggregate_stewarded_bytes_by_peer`.
pub fn load_custodian_relation(conn: &mut SqliteConnection) -> Vec<CustodianRow> {
    use crate::db::diesel_schema::custodian_metrics::dsl as cm;
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    // --- 1. Load all custodian_metrics rows --------------------------------
    let metrics: Vec<CustodianMetrics> = match cm::custodian_metrics
        .select(CustodianMetrics::as_select())
        .load::<CustodianMetrics>(conn)
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("load_custodian_relation: custodian_metrics query failed: {e}");
            return Vec::new();
        }
    };

    if metrics.is_empty() {
        return Vec::new();
    }

    // --- 2. Stewarded bytes per custodian_id (custody-blob commitments) ----
    let custodian_ids: Vec<&str> = metrics.iter().map(|m| m.custodian_id.as_str()).collect();

    // Join key: custodian_metrics.custodian_id == rea_commitments.provider == agent_cid (the canonical agent identity).
    //
    // `state = 'active'` is load-bearing, not cosmetic: without it a CANCELLED
    // (or `proposed`, or `terminated`) custody promise summed into the stewarded
    // byte total, so a withdrawn commitment kept reading as live stewardship on
    // the weave gauges. `.eq("active")` is the dominant convention across this
    // crate's `rea_commitments` readers (peer_selection, serve_routing,
    // household_resilience, membership_identity_reconcile) — the `.ne("cancelled")
    // .ne("terminated")` form used by replication_prioritizer/peer_capacity_service
    // is the minority spelling and admits `proposed` rows, which are promises not
    // yet live.
    let stewarded_rows: Vec<(String, Option<f64>)> = match rc::rea_commitments
        .filter(rc::action.eq("custody-blob"))
        .filter(rc::state.eq("active"))
        .filter(rc::provider.eq_any(&custodian_ids))
        .group_by(rc::provider)
        .select((
            rc::provider,
            sql::<Nullable<Double>>("SUM(resource_quantity_value)"),
        ))
        .load::<(String, Option<f64>)>(conn)
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("load_custodian_relation: rea_commitments query failed: {e}");
            Vec::new()
        }
    };

    let stewarded_map: std::collections::HashMap<String, u64> = stewarded_rows
        .into_iter()
        .filter_map(|(peer, bytes)| bytes.map(|b| (peer, b.max(0.0) as u64)))
        .collect();

    // --- 3. Map to CustodianRow --------------------------------------------
    metrics
        .into_iter()
        .map(|m| {
            let (free, used) =
                match serde_json::from_str::<CustodianStorageMetricsView>(&m.storage_json) {
                    Ok(s) => (
                        Some(s.free_bytes.max(0) as u64),
                        Some(s.used_bytes.max(0) as u64),
                    ),
                    Err(_) => (None, None), // unsampled node — skipped in aggregate
                };
            let stewarded = stewarded_map.get(&m.custodian_id).copied();
            CustodianRow {
                agent_cid: m.custodian_id,
                free,
                used,
                stewarded,
            }
        })
        .collect()
}

/// Adapter: fold the gaps, then publish the count to the /metrics gauge.
/// NEVER call `.set()` inside the fold — the fold is pure; this adapter is the
/// only place a gauge is touched.
pub fn emit_placement_gap_gauge(gaps: &[PlacementGapView]) {
    let count = placement_gap_count(gaps);
    crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.set(count as i64);
}

/// Adapter: publish the already-folded cluster capacity to the /metrics gauges.
/// Takes the `aggregate_capacity` fold OUTPUT (the caller folds; this only emits)
/// so the gauge and `WeaveView.cluster_capacity` are the SAME number. An unreported
/// field (`None`) projects as `0` — the cluster gauge is a scalar floor, not a
/// nullable; a node that never reported simply contributes nothing.
/// `u64 → i64`: capacities never approach `i64::MAX`, but saturate defensively.
pub fn emit_capacity_gauges(cap: &elohim_views::ComputeTriptych) {
    let as_i64 = |v: Option<u64>| v.unwrap_or(0).min(i64::MAX as u64) as i64;
    crate::metrics::ELOHIM_CUSTODIAN_FREE_BYTES.set(as_i64(cap.free));
    crate::metrics::ELOHIM_CUSTODIAN_USED_BYTES.set(as_i64(cap.used));
    crate::metrics::ELOHIM_CUSTODIAN_STEWARDED_BYTES.set(as_i64(cap.stewarded));
}

/// Load ALL placement gaps (node-level, not app-scoped) from the database.
///
/// The operational-weave lens is cluster-scoped and viewer-less: the gauge and
/// the WeaveView reflect the whole node's state, not a single app's.  Using
/// `list_gaps(conn, h_app_id, …)` would under-count by scoping to one app.
fn load_all_placement_gaps(conn: &mut SqliteConnection) -> Vec<PlacementGapView> {
    use crate::db::diesel_schema::placement_gaps::dsl as pg;
    use crate::db::models::PlacementGapRow;

    match pg::placement_gaps
        .order(pg::last_seen_at.desc())
        .load::<PlacementGapRow>(conn)
    {
        Ok(rows) => rows.into_iter().map(Into::into).collect(),
        Err(e) => {
            tracing::warn!("load_all_placement_gaps: query failed: {e}");
            Vec::new()
        }
    }
}

/// Load the WHOLE-NODE holder relation — every `(hub, agent, region)` that holds
/// ANY shard on this node — for the viewer-less `region_occupancy` fold.
///
/// This is the cluster-scoped sibling of `household_resilience::load_holder_relation`,
/// which is content-scoped (filters by a content's `shard_hashes` + `h_app_id`). The
/// operational weave has no single content and no single viewer, so we load ALL
/// `shard_locations` with NO shard/content/app filter (mirroring
/// `load_all_placement_gaps`).
///
/// ## Identity join (the misnamed-column trap)
/// `shard_locations.peer_id` HOLDS an `agent_cid` (NOT a libp2p transport id — see
/// `elohim-storage/CLAUDE.md` Identity & Transport Coherence); the canonical join is
/// `humans.agent_pub_key == shard_locations.peer_id`. `collectives` is LEFT-joined so a
/// holder without a collective folds to the `unknown` region bucket. A query error
/// degrades to an empty `Vec` (warn-and-continue), exactly like the placement-gap loader.
///
/// ## Dormancy (correct-but-empty)
/// `humans.agent_pub_key` is a substrate-only-written column (no HTTP create surface
/// populates it today — `project_resilience_snapshot_humans_junction`), so this join
/// reads EMPTY on a commitments-only-seeded node. That is the honest correct-but-dormant
/// projection: the lens is SELECTED (so `region_occupancy` is `Some(empty)`, never absent),
/// and it lights up the moment a real `agent_cid`-bearing `humans` row + shard placement land.
fn load_all_holder_relation(conn: &mut SqliteConnection) -> Vec<HolderRow> {
    use crate::db::diesel_schema::{collectives, humans, shard_locations};

    let rows: Vec<(Option<String>, String, Option<String>)> = match shard_locations::table
        .inner_join(
            humans::table.on(humans::agent_pub_key
                .nullable()
                .eq(shard_locations::peer_id.nullable())),
        )
        .left_join(collectives::table.on(collectives::id.nullable().eq(humans::household_id)))
        .select((
            humans::household_id,
            humans::id,
            collectives::region.nullable(),
        ))
        .load(conn)
    {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!("load_all_holder_relation: query failed: {e}");
            return Vec::new();
        }
    };

    rows.into_iter()
        .map(|(hub_id, agent_id, region)| HolderRow {
            hub_id,
            agent_id,
            region,
        })
        .collect()
}

/// Build the full [`WeaveView`] from the current DB state.
///
/// The caller stamps `measured_at` (passed as an ISO-8601 string); this
/// function NEVER calls a clock.  Both Prometheus gauges are also set here
/// as a side-effect so the two wire shapes stay in sync with one load.
///
/// `tier_occupancy` stays `None` by deliberate scope decision: the holder
/// relation carries no risk-tier dimension and `RiskTierDistribution` is a
/// spatial-vulnerability type — fabricating a tier would violate the
/// not-selected-field contract. Designing a real risk-tier source is a separate
/// follow-on (see the fold's doc-comment in `operational_weave.rs`).
pub fn build_weave_view(conn: &mut SqliteConnection, measured_at: String) -> WeaveView {
    let gaps = load_all_placement_gaps(conn);
    let custodians = load_custodian_relation(conn);
    let holders = load_all_holder_relation(conn);

    // --- folds (pure) ----------------------------------------------------------
    let coverage = rs_coverage(&gaps);
    let cluster_capacity = aggregate_capacity(&custodians);
    let regions = region_occupancy(&holders);

    // --- gauges (adapter layer — fold is pure; NEVER set() inside a fold) -------
    emit_placement_gap_gauge(&gaps);
    crate::metrics::ELOHIM_RS_COVERAGE_MILLI.set((coverage * 1000.0) as i64);
    emit_capacity_gauges(&cluster_capacity);

    // --- view ------------------------------------------------------------------
    WeaveView {
        placement_gap_count: placement_gap_count(&gaps) as u32,
        rs_coverage: Some(coverage),
        cluster_capacity: Some(cluster_capacity),
        tier_occupancy: None,
        region_occupancy: Some(regions),
        measured_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gap() -> PlacementGapView {
        PlacementGapView {
            id: "g".into(),
            content_id: "c".into(),
            shard_hash: "s".into(),
            requested_steward_count: 3,
            achieved_steward_count: 1,
            contract_coverage: 0.33,
            gap_kind: "under_replicated".into(),
            first_seen_at: "t0".into(),
            last_seen_at: "t1".into(),
        }
    }

    #[test]
    fn emit_sets_the_gauge_to_the_fold_count() {
        crate::metrics::register_all(); // idempotent (Once-guarded)
        emit_placement_gap_gauge(&[gap(), gap()]);
        assert_eq!(crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.get(), 2);
        emit_placement_gap_gauge(&[]);
        assert_eq!(crate::metrics::ELOHIM_PLACEMENT_GAP_COUNT.get(), 0);
    }

    #[test]
    fn emit_capacity_gauges_mirror_triptych_none_is_zero() {
        crate::metrics::register_all(); // idempotent (Once-guarded)
        emit_capacity_gauges(&elohim_views::ComputeTriptych {
            free: Some(30),
            used: Some(5),
            stewarded: None, // unreported field → 0, never a panic
        });
        assert_eq!(crate::metrics::ELOHIM_CUSTODIAN_FREE_BYTES.get(), 30);
        assert_eq!(crate::metrics::ELOHIM_CUSTODIAN_USED_BYTES.get(), 5);
        assert_eq!(crate::metrics::ELOHIM_CUSTODIAN_STEWARDED_BYTES.get(), 0);
    }
}
