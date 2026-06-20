//! Operational-weave facing — impure loaders + the two adapters that project the
//! pure folds (elohim_facings::folds::operational_weave) as a typed WeaveView AND
//! as Prometheus gauges. The fold returns numbers; the adapter emits both wire
//! shapes. Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Double, Nullable};
use elohim_facings::folds::operational_weave::{placement_gap_count, CustodianRow};
use elohim_views::{CustodianStorageMetricsView, PlacementGapView};

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

    let stewarded_rows: Vec<(String, Option<f64>)> = match rc::rea_commitments
        .filter(rc::action.eq("custody-blob"))
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
            let (free, used) = match serde_json::from_str::<CustodianStorageMetricsView>(
                &m.storage_json,
            ) {
                Ok(s) => (
                    Some(s.free_bytes.max(0) as u64),
                    Some(s.used_bytes.max(0) as u64),
                ),
                Err(_) => (None, None), // unsampled node — skipped in aggregate
            };
            let stewarded = stewarded_map.get(&m.custodian_id).copied();
            CustodianRow { agent_cid: m.custodian_id, free, used, stewarded }
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
}
