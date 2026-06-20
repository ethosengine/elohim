//! Operational-weave facing — impure loaders + the two adapters that project the
//! pure folds (elohim_facings::folds::operational_weave) as a typed WeaveView AND
//! as Prometheus gauges. The fold returns numbers; the adapter emits both wire
//! shapes. Charter: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md

use elohim_facings::folds::operational_weave::placement_gap_count;
use elohim_views::PlacementGapView;

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
