//! The lens-market CONTENTION fold — the controversy-spread surface (Beer's
//! viability/dissent hand). Pure (DB-free); the diesel verdict/ballot-tally rows
//! live in elohim-storage and are mirrored onto `LensVerdictRow` first.
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
//! (§8 contention = controversy SPREAD, not net score). Plan I2 / gap #4:
//! genesis/docs/superpowers/plans/2026-06-27-plural-mishpat-lenses-wave1-plan.md
//!
//! Controversy, not popularity: 500↑/500↓ is maximally contended (1.0); 1000↑/0↓ is
//! not (0.0). C-class fold — `epr_contention` projection is reconstructable from the
//! notarized tallies (the integrity-recompute contract). The threshold-crossing of
//! this index, JOINED with affinity-decay (lens_selector::classify_regime), is what
//! fires `ContentionBreach` — the cross-surface fusion (plan I3, the net-new primitive).

use std::collections::BTreeMap;

/// DB-free mirror of one verdict row (an attested ballot tally entry, B2 — the raw
/// ballot stays private; the fold sees only the notarized tally; plan A5).
///
/// - `epr_scope` — the EPR **slug-id** the verdict is over (plan A3).
/// - `lens_cid` — the lens the verdict is about (`entry_hash`).
/// - `verdict` — the stance token: `agree`/`disagree` (binary), or a ranked/intensity
///   token a later method-lens enriches (quadratic/RCV; spec §8). Wave-1 reads the
///   binary agree/disagree spread.
/// - `agent` — `agent_cid` of the voter (never a transport id).
#[derive(Debug, Clone)]
pub struct LensVerdictRow {
    pub epr_scope: String,
    pub lens_cid: String,
    pub verdict: String,
    pub agent: String,
}

/// **Intent.** The controversy index over one EPR context, in `[0.0, 1.0]`:
/// `1.0 - |agree - disagree| / (agree + disagree)`. `1.0` = perfectly split (maximum
/// contention), `0.0` = unanimous (or no verdicts). Volume-weighting (high-volume AND
/// high-balance = the spec's "contested regime") is a later refinement; Wave-1 measures
/// the balance, the loader supplies the volume alongside.
pub fn contention_index(rows: &[LensVerdictRow], epr_scope: &str) -> f64 {
    let mut agree = 0usize;
    let mut disagree = 0usize;
    for r in rows.iter().filter(|r| r.epr_scope == epr_scope) {
        match r.verdict.as_str() {
            "agree" | "up" | "for" => agree += 1,
            "disagree" | "down" | "against" => disagree += 1,
            _ => {} // abstentions / unranked tokens do not move the binary spread
        }
    }
    let total = agree + disagree;
    if total == 0 {
        return 0.0;
    }
    1.0 - ((agree as f64 - disagree as f64).abs() / total as f64)
}

/// Per-lens verdict histogram (the raw distribution the contention index summarizes;
/// feeds the ControversyBadge view). Deterministic `BTreeMap` order.
pub fn verdicts_by_lens(rows: &[LensVerdictRow]) -> BTreeMap<String, BTreeMap<String, usize>> {
    crate::fold::bucket_by(rows, |r| Some(r.lens_cid.clone()))
        .into_iter()
        .map(|(lens, bucket)| {
            let mut counts: BTreeMap<String, usize> = BTreeMap::new();
            for r in bucket {
                *counts.entry(r.verdict.clone()).or_default() += 1;
            }
            (lens, counts)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(scope: &str, lens: &str, verdict: &str, agent: &str) -> LensVerdictRow {
        LensVerdictRow {
            epr_scope: scope.to_string(),
            lens_cid: lens.to_string(),
            verdict: verdict.to_string(),
            agent: agent.to_string(),
        }
    }

    #[test]
    fn split_is_maximally_contended_unanimous_is_not() {
        let split = vec![
            v("e", "l", "agree", "a"),
            v("e", "l", "disagree", "b"),
        ];
        assert_eq!(contention_index(&split, "e"), 1.0, "1/1 split = max contention");

        let unanimous = vec![
            v("e", "l", "agree", "a"),
            v("e", "l", "agree", "b"),
            v("e", "l", "agree", "c"),
        ];
        assert_eq!(contention_index(&unanimous, "e"), 0.0, "3/0 = no contention");
    }

    #[test]
    fn empty_context_has_zero_contention() {
        assert_eq!(contention_index(&[], "e"), 0.0);
    }

    #[test]
    fn verdict_histogram_is_per_lens() {
        let rows = vec![
            v("e", "georgist", "agree", "a"),
            v("e", "georgist", "disagree", "b"),
            v("e", "beerian", "agree", "c"),
        ];
        let h = verdicts_by_lens(&rows);
        assert_eq!(h.get("georgist").unwrap().get("agree").copied(), Some(1));
        assert_eq!(h.get("georgist").unwrap().get("disagree").copied(), Some(1));
        assert_eq!(h.get("beerian").unwrap().get("agree").copied(), Some(1));
    }
}
