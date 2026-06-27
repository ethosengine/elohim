//! The lens-market AFFINITY fold — context-relative, earned-by-use lens weight.
//! Pure (DB-free); the diesel `lens_selection` rows live in elohim-storage and are
//! mirrored onto `LensSelectionRow` before reaching these folds (the §11 add-a-lens
//! recipe, elohim/elohim-facings/CLAUDE.md).
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
//! (§3 affinity = earned, not decreed). Plan I2 / gap #3:
//! genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md
//!
//! Affinity is **gaming-resistant**: it counts DISTINCT selectors (attested), never
//! raw exercise counts — a Sybil spamming selections under one identity cannot move
//! it (mirrors the `rea::commitment_backed` distinct-household discipline). It is a
//! C-class fold (Operational): the stored `lens_affinity` projection is reconstructable
//! by re-running this over the notarized selection tallies (the integrity-recompute
//! contract, plan I5).

use std::collections::BTreeMap;

/// DB-free mirror of one lens-selection row. The impure loader maps the
/// `lens_selection` / `ballot_tallies` diesel rows onto this so the fold stays DB-free.
///
/// - `lens_cid` — the selected lens `entry_hash` (NOT action_hash; plan A5 gospel).
/// - `selector_agent` — `agent_cid` (`uhCAk…`) of who exercised the lens; never a
///   transport id (cross-namespace guard, plan A5). Distinct-counted for gaming-resistance.
/// - `epr_scope` — the EPR **slug-id** the selection is in (`epr:lamad-spa`), the
///   context key affinity is relative to (plan A3 — slug-id, not the dag-cbor CID).
/// - `selected_at` — RFC3339 timestamp; carried for the later volatility/decay folds.
#[derive(Debug, Clone)]
pub struct LensSelectionRow {
    pub lens_cid: String,
    pub selector_agent: String,
    pub epr_scope: String,
    pub selected_at: String,
}

/// **Intent.** For one context (`epr_scope`), the affinity of each governing lens =
/// distinct selectors who exercised it. `bucket_by` drops out-of-scope rows via the
/// `None` key, then `distinct_count_by` collapses repeat selectors — deterministic
/// `BTreeMap` order (no wire-order leak; plan I2).
pub fn affinity_in_scope(rows: &[LensSelectionRow], epr_scope: &str) -> BTreeMap<String, usize> {
    crate::fold::bucket_by(rows, |r| {
        (r.epr_scope == epr_scope).then(|| r.lens_cid.clone())
    })
    .into_iter()
    .map(|(lens, bucket)| {
        (
            lens,
            crate::fold::distinct_count_by(&bucket, |r| r.selector_agent.clone()),
        )
    })
    .collect()
}

/// Affinity of a single lens in a single context (0 if it has never been selected
/// there — a warm-but-low lens, plan §3.1 warm standby).
pub fn affinity_for(rows: &[LensSelectionRow], lens_cid: &str, epr_scope: &str) -> usize {
    affinity_in_scope(rows, epr_scope)
        .get(lens_cid)
        .copied()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(lens: &str, agent: &str, scope: &str) -> LensSelectionRow {
        LensSelectionRow {
            lens_cid: lens.to_string(),
            selector_agent: agent.to_string(),
            epr_scope: scope.to_string(),
            selected_at: "2026-06-27T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn affinity_is_distinct_selectors_per_lens_in_scope() {
        let rows = vec![
            sel("georgist", "alice", "epr:land"),
            sel("georgist", "alice", "epr:land"), // repeat selector — collapses (gaming-resistance)
            sel("georgist", "bob", "epr:land"),
            sel("beerian", "carol", "epr:land"),
        ];
        let aff = affinity_in_scope(&rows, "epr:land");
        assert_eq!(aff.get("georgist").copied(), Some(2), "distinct selectors, not raw count");
        assert_eq!(aff.get("beerian").copied(), Some(1));
    }

    #[test]
    fn affinity_is_context_relative() {
        // Land-value-tax: high affinity in a developing-economy context, warm-but-low
        // in the platform context (spec §3.1).
        let rows = vec![
            sel("land-tax", "a", "epr:developing"),
            sel("land-tax", "b", "epr:developing"),
            sel("antitrust", "c", "epr:platform"),
        ];
        assert_eq!(affinity_for(&rows, "land-tax", "epr:developing"), 2);
        assert_eq!(affinity_for(&rows, "land-tax", "epr:platform"), 0, "warm, not active, in this context");
    }
}
