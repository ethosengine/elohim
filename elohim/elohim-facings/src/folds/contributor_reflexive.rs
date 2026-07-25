//! The contributor-reflexive facing's folds — "how the network sees a contributor":
//! the recognition the network has routed TO a `ContributorPresence` (folded from its
//! `economic_events`) and the content it stewards (folded from its active
//! `stewardship_allocations`).
//!
//! Pure (DB-free) by construction. The storage rows (`EconomicEvent`,
//! `StewardshipAllocation`) are diesel-typed and CANNOT enter this crate; the storage
//! adapter (`elohim-storage::services::contributor_reflexive_facing`) maps them to the
//! DB-free mirror rows below, then calls these folds — the thin-adapter pattern every
//! lens uses.
//!
//! Child of the §11 lens framework
//! (`genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md`);
//! SIBLING to the rea-economic (commitment-ledger) lens, which is a DIFFERENT concern —
//! recognition-share / `StewardshipAllocation` is explicitly orthogonal to commitments
//! per that charter's Non-goals. Spec:
//! `genesis/docs/superpowers/specs/2026-06-21-contributor-presence-bootstrap-whoswho-design.md` (Wave 2).
//!
//! Scope (v1): a SINGLE-presence rollup. Two deliberate boundaries, grounded in the data:
//!   * Steward recursion ("fractal stewards") has NO data substrate today — every steward
//!     edge is single-hop (`steward_presence_id → content`, `presence.steward_id → a human`,
//!     `human → household`); there is no steward→steward edge to descend. A future fractal
//!     rollup reuses `elohim-storage::recursion::CoverageRollup`, not new machinery.
//!   * Commons settlements carry `contributor_presence_id = None`, so the presence-joined
//!     relation only PARTIALLY sees the commons (see [`commons_flow_value`]); full commons
//!     accounting is a separate `receiver == "commons-pool"` query (deferred).

use std::collections::{BTreeMap, BTreeSet};

use crate::fold::bucket_by;

/// The literal Collective CID that receives the commons tribute in REA settlements
/// (`elohim-storage::services::share_routing`). Recognition/value routed here is "to the commons".
pub const COMMONS_POOL: &str = "commons-pool";

/// DB-free mirror of one `economic_events` row joined to a contributor presence
/// (`contributor_presence_id ==` the presence). The storage adapter maps the diesel
/// `EconomicEvent` onto this so the folds stay DB-free. Only the fields these folds
/// consume are mirrored (not the full event row).
#[derive(Debug, Clone)]
pub struct RecognitionEventRow {
    /// REA verb — `produce` (recognition distribution), `transfer` (settlement),
    /// `cite`, `use`, … .
    pub action: String,
    /// Beneficiary of the value. The literal `"commons-pool"` for a commons tribute.
    pub receiver: String,
    /// The value magnitude to sum (`resource_quantity_value`); `None` ⇒ a
    /// non-quantified event (skipped from sums).
    pub resource_quantity_value: Option<f64>,
    /// The content node that drove the recognition, when present.
    pub content_id: Option<String>,
}

/// DB-free mirror of one `stewardship_allocations` row where this presence is the steward.
/// The storage adapter maps the diesel `StewardshipAllocation` onto this.
#[derive(Debug, Clone)]
pub struct AllocationRow {
    pub content_id: String,
    pub allocation_ratio: f64,
    /// `active` / `disputed` / `superseded` — only `active` counts (the pipeline invariant).
    pub governance_state: String,
    /// Running total recognition routed to this steward via this allocation
    /// (maintained by `accumulate_recognition`, not re-derived here).
    pub recognition_accumulated: f64,
}

/// Total recognition value the network has routed to this contributor — the sum of
/// quantified event magnitudes. Non-quantified events (`None` value) are skipped.
pub fn total_recognition_value(events: &[RecognitionEventRow]) -> f64 {
    events
        .iter()
        .filter_map(|e| e.resource_quantity_value)
        .sum()
}

/// Recognition value bucketed by REA action, summed per action. Reuses
/// [`bucket_by`] (deterministic `BTreeMap` order → no hash-iteration leak onto the wire).
pub fn recognition_value_by_action(events: &[RecognitionEventRow]) -> BTreeMap<String, f64> {
    bucket_by(events, |e| Some(e.action.clone()))
        .into_iter()
        .map(|(action, rows)| {
            let sum: f64 = rows.iter().filter_map(|e| e.resource_quantity_value).sum();
            (action, sum)
        })
        .collect()
}

/// The count of DISTINCT content nodes that routed recognition to this contributor
/// (an event with no `content_id` is not counted; the same content via multiple events
/// counts once). `BTreeSet` for deterministic distinctness.
pub fn distinct_content_recognized(events: &[RecognitionEventRow]) -> i32 {
    events
        .iter()
        .filter_map(|e| e.content_id.as_deref())
        .collect::<BTreeSet<_>>()
        .len() as i32
}

/// Recognition value visibly routed VIA the commons (`receiver == "commons-pool"`).
///
/// PARTIAL by construction: most commons settlements carry `contributor_presence_id = None`
/// and are therefore absent from this presence-joined relation. This captures only the
/// commons flows attributed to THIS presence's events; full commons accounting is a
/// separate `receiver`-keyed query (deferred — see module docs).
pub fn commons_flow_value(events: &[RecognitionEventRow]) -> f64 {
    events
        .iter()
        .filter(|e| e.receiver == COMMONS_POOL)
        .filter_map(|e| e.resource_quantity_value)
        .sum()
}

/// Active steward allocations: `(count, summed recognition_accumulated)`. Filters
/// `governance_state == "active"` (disputed/superseded allocations do not count).
pub fn steward_allocation_summary(allocs: &[AllocationRow]) -> (i32, f64) {
    let active: Vec<&AllocationRow> = allocs
        .iter()
        .filter(|a| a.governance_state == "active")
        .collect();
    let total: f64 = active.iter().map(|a| a.recognition_accumulated).sum();
    (active.len() as i32, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(
        action: &str,
        receiver: &str,
        value: Option<f64>,
        content: Option<&str>,
    ) -> RecognitionEventRow {
        RecognitionEventRow {
            action: action.to_string(),
            receiver: receiver.to_string(),
            resource_quantity_value: value,
            content_id: content.map(str::to_string),
        }
    }

    fn alloc(content: &str, ratio: f64, state: &str, accrued: f64) -> AllocationRow {
        AllocationRow {
            content_id: content.to_string(),
            allocation_ratio: ratio,
            governance_state: state.to_string(),
            recognition_accumulated: accrued,
        }
    }

    #[test]
    fn total_recognition_value_sums_and_skips_none() {
        let events = vec![
            ev("produce", "presence-x", Some(3.0), Some("c1")),
            ev("produce", "presence-x", Some(1.5), Some("c2")),
            ev("cite", "presence-x", None, Some("c3")), // unquantified → skipped
        ];
        assert_eq!(total_recognition_value(&events), 4.5);
        assert_eq!(total_recognition_value(&[]), 0.0);
    }

    #[test]
    fn recognition_value_by_action_buckets_and_sums() {
        let events = vec![
            ev("produce", "presence-x", Some(2.0), Some("c1")),
            ev("produce", "presence-x", Some(3.0), Some("c2")),
            ev("transfer", "commons-pool", Some(1.0), None),
            ev("cite", "presence-x", None, Some("c3")), // contributes 0 to its bucket
        ];
        let by = recognition_value_by_action(&events);
        assert_eq!(by.get("produce").copied(), Some(5.0));
        assert_eq!(by.get("transfer").copied(), Some(1.0));
        assert_eq!(by.get("cite").copied(), Some(0.0));
        // BTreeMap → deterministic key order (no hash-iteration leak).
        let keys: Vec<&String> = by.keys().collect();
        assert_eq!(keys, vec!["cite", "produce", "transfer"]);
    }

    #[test]
    fn distinct_content_recognized_collapses_and_ignores_none() {
        let events = vec![
            ev("produce", "p", Some(1.0), Some("c1")),
            ev("produce", "p", Some(1.0), Some("c1")), // dup content → once
            ev("produce", "p", Some(1.0), Some("c2")),
            ev("transfer", "p", Some(1.0), None), // no content → not counted
        ];
        assert_eq!(distinct_content_recognized(&events), 2);
        assert_eq!(distinct_content_recognized(&[]), 0);
    }

    #[test]
    fn commons_flow_value_only_counts_commons_receiver() {
        let events = vec![
            ev("transfer", "commons-pool", Some(5.0), None),
            ev("transfer", "collective-abc", Some(9.0), None), // not commons
            ev("produce", "presence-x", Some(3.0), Some("c1")), // not commons
            ev("transfer", "commons-pool", Some(2.0), None),
        ];
        assert_eq!(commons_flow_value(&events), 7.0);
        assert_eq!(commons_flow_value(&[]), 0.0);
    }

    #[test]
    fn steward_allocation_summary_filters_active() {
        let allocs = vec![
            alloc("c1", 1.0, "active", 10.0),
            alloc("c2", 0.5, "active", 4.0),
            alloc("c3", 1.0, "disputed", 99.0),   // excluded
            alloc("c4", 1.0, "superseded", 99.0), // excluded
        ];
        let (count, total) = steward_allocation_summary(&allocs);
        assert_eq!(count, 2);
        assert_eq!(total, 14.0);
        assert_eq!(steward_allocation_summary(&[]), (0, 0.0));
    }
}
