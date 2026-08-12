"""Canonical drift-score formula for CLAUDE.md review.

The score combines age, edit pressure, and structural change pressure into
a single number compared against a threshold (default 3.0). Hooks update
counters; audit/ceremony recomputes the score from those counters at audit
time (counters are source of truth; stored drift_score is a cache).

Weights are re-tunable here. Changing them does not change the protocol —
just shifts when the ceremony fires. Document changes in the SKILL.md.
"""
from __future__ import annotations

import math
from datetime import date


# Re-tunable. Heavier weights signal "this counter type matters more."
SCORE_WEIGHTS = {
    "age": 1.0,              # days_since_audit / 90  — slow ramp toward threshold
    "scope_edits": 0.8,      # log1p(scope_edits)     — files in scope changed
    "direct_edits": 2.0,     # log1p(direct_edits)    — CLAUDE.md itself edited
    "structural_edits": 5.0,  # log1p(structural_edits) — mv/cp -r/rm -rf in scope; high-impact
}


def days_since(iso_date: str | None) -> float:
    """Return days since `iso_date` (YYYY-MM-DD); 90.0 if never audited."""
    if not iso_date:
        return 90.0
    try:
        d = date.fromisoformat(iso_date)
        return max(0.0, (date.today() - d).days)
    except (ValueError, TypeError):
        return 90.0


def compute_score(entry: dict, weights: dict | None = None) -> float:
    """Compute drift_score from a file's accumulated counters.

    `entry` is the per-file dict from claude-md-drift.json's `files`. Missing
    counters default to 0 / None. Returns a float rounded to 3 decimals.
    """
    w = weights or SCORE_WEIGHTS
    age_term = w["age"] * (days_since(entry.get("last_audited")) / 90.0)
    scope_term = w["scope_edits"] * math.log1p(entry.get("scope_edits", 0))
    direct_term = w["direct_edits"] * math.log1p(entry.get("direct_edits", 0))
    structural_term = w["structural_edits"] * math.log1p(entry.get("structural_edits", 0))
    return round(age_term + scope_term + direct_term + structural_term, 3)


def measure(entry: dict, weights: dict | None = None) -> dict:
    """`compute_score` as a DECLARED measure — and the declaration is mostly a confession.

    Every term above is dimensionless: `days/90` is a ratio of durations, `log1p(count)` is a
    bare number. So this passes any dimensional check, including the `MeasureKind::combine_additive`
    algebra in `elohim/epr/src/measure.rs` — `Ratio + Ratio` is legal. **That is the algebra's
    limit, not this score's absolution** (sealed spec Q16): dimension and *commensurability* are
    different questions, and adding normalized time to a log of edit counts is incoherent on the
    second while passing the first. The weighted sum is a preference ordering wearing a
    quantity's clothes, and the threshold `3.0` has no interpretation a reader could argue with.

    Meadows made exactly this objection to GNP — an index whose value theory is buried can only
    be accepted or rejected, never argued with, and both are failures of deliberation. Our own
    comparative-political-economy trap library reached the same rule from the other direction:
    prefer observable mechanisms to imputed aggregates.

    So the claim is `modelled`, not `witnessed` or `estimated`, and the interval is **unknown**.
    The counters underneath ARE witnessed and could each carry an exact interval; the composite
    cannot, because there is no observation it is an estimate OF. Reporting a tight band around
    a number whose weights a human re-tunes at will would be the false precision this whole
    ontology exists to prevent — the score moves when someone changes their mind, not when the
    world does.

    Kept as a SEPARATE entry point rather than changing `compute_score`'s return: the raw float
    is what the hooks cache and the audit compares against a threshold, and rewiring that is a
    behaviour change this declaration does not need to make.
    """
    from . import signal_measure as sm

    w = weights or SCORE_WEIGHTS
    terms = ", ".join(f"{k}x{v}" for k, v in w.items())
    return sm.measure(
        compute_score(entry, weights), "ratio", claim="modelled",
        interval=sm.unknown_interval(),
        # Spec Q17, and this is the call site the whole enum was minted for. The paragraph below
        # already confessed the incommensurability in prose; `unknown_reason` makes it MACHINE
        # readable, so an uncertainty work-queue can exclude this score structurally instead of
        # ranking "go measure the composite harder" — the exact false-precision reflex the
        # confession exists to refuse. `tightenable("incommensurable") == "never"`.
        unknown_reason="incommensurable",
        basis=(f"weighted composite of re-tunable terms ({terms}) over witnessed counters "
               f"(scope_edits={entry.get('scope_edits', 0)}, "
               f"direct_edits={entry.get('direct_edits', 0)}, "
               f"structural_edits={entry.get('structural_edits', 0)}, "
               f"days_since_audit={days_since(entry.get('last_audited')):.0f}). "
               f"NOT an estimate of any observable: the terms are dimensionless but "
               f"INCOMMENSURABLE (normalized time added to log-counts), so the interval is "
               f"unknown by construction and the threshold is a tuning choice, not a limit "
               f"(spec Q16)"))
