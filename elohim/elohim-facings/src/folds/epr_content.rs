//! The EPR content-perspective facing's folds — an EPR content atom's first-person
//! view of its leg-neighborhood (knowledge · value · governance couplings), folded
//! from the `epr_coupling` relation.
//!
//! Pure (DB-free) by construction. The storage row (`EprCouplingRow`) is diesel-typed
//! and CANNOT enter this crate; the storage adapter maps it to the DB-free
//! [`CouplingRow`] below, then calls these folds — the same thin-adapter pattern the
//! resiliency facing uses, and the pattern EVERY lens uses (its DB rows are
//! diesel-typed; the pure fold takes a DB-free mirror row).
//!
//! Charter: `genesis/docs/superpowers/specs/2026-06-19-epr-content-perspective-facing-lens-design.md`.

use std::collections::BTreeSet;

use elohim_views::EprCouplingView;

/// DB-free mirror of one `epr_coupling` row: the source atom (`epr_cid`) couples to
/// `target_cid` via `leg` (one of "knowledge" / "value" / "governance"). The storage
/// adapter maps the diesel `EprCouplingRow` onto this so the folds stay DB-free.
#[derive(Debug, Clone)]
pub struct CouplingRow {
    pub epr_cid: String,
    pub leg: String,
    pub target_cid: String,
}

/// Pivot an atom's FORWARD couplings into the three named legs. Lifted verbatim from
/// the `/envelope` handler's inline kernel (last-write-wins per leg — preserved so the
/// `/envelope` JSON stays byte-identical). DB-free, so the leg semantics are
/// exhaustively unit-testable.
pub fn fold_coupling_legs(rows: &[CouplingRow]) -> EprCouplingView {
    let mut coupling = EprCouplingView::default();
    for row in rows {
        match row.leg.as_str() {
            "knowledge" => coupling.knowledge = Some(row.target_cid.clone()),
            "value" => coupling.value = Some(row.target_cid.clone()),
            "governance" => coupling.governance = Some(row.target_cid.clone()),
            _ => {}
        }
    }
    coupling
}

/// The atom's forward neighborhood degree — the count of DISTINCT target CIDs it
/// couples to across all legs (a multi-leg coupling to the same target counts once).
pub fn neighborhood_degree(rows: &[CouplingRow]) -> i32 {
    rows.iter()
        .map(|r| r.target_cid.as_str())
        .collect::<BTreeSet<_>>()
        .len() as i32
}

/// The reverse "part-of" set — the DISTINCT source atoms that couple TO this atom
/// (fed the REVERSE couplings, where `target_cid == this atom`). Sorted (`BTreeSet`)
/// for deterministic wire order — no hash-iteration leak.
pub fn reverse_part_of(reverse_rows: &[CouplingRow]) -> Vec<String> {
    reverse_rows
        .iter()
        .map(|r| r.epr_cid.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(epr_cid: &str, leg: &str, target_cid: &str) -> CouplingRow {
        CouplingRow {
            epr_cid: epr_cid.to_string(),
            leg: leg.to_string(),
            target_cid: target_cid.to_string(),
        }
    }

    #[test]
    fn fold_coupling_legs_pivots_the_three_legs() {
        let rows = vec![
            row("self", "knowledge", "k-cid"),
            row("self", "value", "v-cid"),
            row("self", "governance", "g-cid"),
        ];
        let c = fold_coupling_legs(&rows);
        assert_eq!(c.knowledge.as_deref(), Some("k-cid"));
        assert_eq!(c.value.as_deref(), Some("v-cid"));
        assert_eq!(c.governance.as_deref(), Some("g-cid"));
    }

    #[test]
    fn fold_coupling_legs_ignores_unknown_legs_and_is_last_write_wins() {
        // Preserves the /envelope handler's exact behavior: unknown legs dropped,
        // duplicate leg → last row wins (byte-identical-gate invariant).
        let rows = vec![
            row("self", "knowledge", "first"),
            row("self", "process", "ignored"), // 4th leg not yet modeled in the view
            row("self", "knowledge", "second"),
        ];
        let c = fold_coupling_legs(&rows);
        assert_eq!(c.knowledge.as_deref(), Some("second"), "last write wins");
        assert!(c.value.is_none());
        assert!(c.governance.is_none());
    }

    #[test]
    fn fold_coupling_legs_empty_is_default() {
        let c = fold_coupling_legs(&[]);
        assert!(c.knowledge.is_none() && c.value.is_none() && c.governance.is_none());
    }

    #[test]
    fn neighborhood_degree_counts_distinct_targets() {
        let rows = vec![
            row("self", "knowledge", "a"),
            row("self", "value", "a"), // same target via another leg → counts once
            row("self", "governance", "b"),
        ];
        assert_eq!(neighborhood_degree(&rows), 2);
        assert_eq!(neighborhood_degree(&[]), 0);
    }

    #[test]
    fn reverse_part_of_is_distinct_and_sorted() {
        // fetch_reverse_coupling returns rows whose target_cid == this atom; the
        // source atoms (epr_cid) are the "part-of" set.
        let reverse = vec![
            row("z-atom", "knowledge", "self"),
            row("a-atom", "value", "self"),
            row("z-atom", "governance", "self"), // dup source → once
        ];
        assert_eq!(reverse_part_of(&reverse), vec!["a-atom", "z-atom"]);
        assert!(reverse_part_of(&[]).is_empty());
    }
}
