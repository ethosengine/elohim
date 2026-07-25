//! The lens-market SELECTOR + REGIME-DRIFT folds. Pure (DB-free).
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
//! (§3.1 B+C selection, §7 floor, §8 the regime-drift fusion). Plan I2/I3, gaps #8/#11:
//! genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md
//!
//! `select_lenses` is telos-fitness selection over the rows a SQL scope filter already
//! returned (the impure load + per-row degrade-skip of unparseable lenses lives storage-side,
//! mirroring `find_active_projections`). `classify_regime` is the GENUINELY NEW primitive:
//! the cross-surface fusion that no existing trigger carries (DelayBreach is viability-only;
//! the limitarian `valid_until` is a clock-only) — Breached iff affinity decays AND contention
//! rises together.

/// DB-free mirror of one governing lens, pre-joined with its current C-class scores.
/// `valid` is the per-lens fail-closed flag: a poisoned/unparseable lens row is loaded
/// as `valid: false` (NOT dropped from the set) so it degrades its own row only, never
/// empties the market (the EprRouter-empties lesson; plan A5).
#[derive(Debug, Clone)]
pub struct LensRow {
    pub lens_cid: String,
    pub epr_scope: String,
    pub affinity: usize,
    pub contention: f64,
    pub valid: bool,
}

/// Telos-fitness selection: keep the valid lenses, rank by earned affinity (desc),
/// deterministic `lens_cid` tie-break. Returns ALL valid lenses (plurality is preserved —
/// this is a ranked *set* of co-valid readings for the table, NOT a single winner;
/// design-spec "infrastructure for the table, not the tower"). The caller renders the
/// high-affinity lens as the de-facto active reading in this context while the rest stay
/// warm (spec §3.1).
pub fn select_lenses(rows: &[LensRow]) -> Vec<&LensRow> {
    let mut selected: Vec<&LensRow> = rows.iter().filter(|r| r.valid).collect();
    selected.sort_by(|a, b| {
        b.affinity
            .cmp(&a.affinity)
            .then(a.lens_cid.cmp(&b.lens_cid))
    });
    selected
}

/// The regime status of an incumbent lens between two observations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegimeStatus {
    /// Holding its regime — affinity not decaying, contention not rising.
    Stable,
    /// One surface moved adversely (a warning, not yet a breach).
    Drifting,
    /// BOTH surfaces moved adversely together — the joint fusion. This is the
    /// "this lens has exited its valid regime" detector that fires `ContentionBreach`.
    Breached,
}

/// **The net-new primitive (spec §8).** Breached iff the justice surface decays
/// (`affinity_now < affinity_prev`) AND the viability/dissent surface rises
/// (`contention_now > contention_prev`) together. Either alone is a false positive —
/// a quiet backwater has low contention AND low fitness; a healthy live debate has high
/// contention BUT a stable affinity leader — so either alone is only `Drifting`.
pub fn classify_regime(
    affinity_now: usize,
    affinity_prev: usize,
    contention_now: f64,
    contention_prev: f64,
) -> RegimeStatus {
    let affinity_decay = affinity_now < affinity_prev;
    let contention_rise = contention_now > contention_prev;
    match (affinity_decay, contention_rise) {
        (true, true) => RegimeStatus::Breached,
        (true, false) | (false, true) => RegimeStatus::Drifting,
        (false, false) => RegimeStatus::Stable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lens(cid: &str, affinity: usize, contention: f64, valid: bool) -> LensRow {
        LensRow {
            lens_cid: cid.to_string(),
            epr_scope: "epr:x".to_string(),
            affinity,
            contention,
            valid,
        }
    }

    #[test]
    fn select_keeps_plurality_ranks_by_affinity_skips_invalid() {
        let rows = vec![
            lens("beerian", 5, 0.2, true),
            lens("poisoned", 99, 0.9, false), // invalid → degrades its row only
            lens("georgist", 8, 0.1, true),
        ];
        let sel = select_lenses(&rows);
        assert_eq!(
            sel.len(),
            2,
            "both valid lenses kept — plurality preserved, poisoned skipped"
        );
        assert_eq!(sel[0].lens_cid, "georgist", "higher affinity ranks first");
        assert_eq!(sel[1].lens_cid, "beerian");
    }

    #[test]
    fn regime_breached_only_on_joint_decay_and_rise() {
        assert_eq!(
            classify_regime(3, 5, 0.8, 0.4),
            RegimeStatus::Breached,
            "affinity down AND contention up"
        );
        assert_eq!(
            classify_regime(3, 5, 0.4, 0.4),
            RegimeStatus::Drifting,
            "affinity down alone"
        );
        assert_eq!(
            classify_regime(5, 5, 0.8, 0.4),
            RegimeStatus::Drifting,
            "contention up alone"
        );
        assert_eq!(
            classify_regime(6, 5, 0.2, 0.4),
            RegimeStatus::Stable,
            "both healthy"
        );
    }
}
