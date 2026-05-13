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
