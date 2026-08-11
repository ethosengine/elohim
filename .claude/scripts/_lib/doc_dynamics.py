"""Generation / absorption for the doc corpus — the harvest/regeneration index
applied to our own development discipline.

Meadows' rule: overshoot is indicated when the ratio crosses 1.0, not when the
stock is visibly gone. The numerator is witnessed (git log counts adds); the
denominator is ESTIMATED, because absorption happens through at least four
paths (deletion, moves to held/, archive sweeps, in-place compaction) and no
ledger counts all four. The interval is wide on purpose — narrowing it to look
better would be exactly the false precision this ontology exists to prevent.

Wire vocabulary mirrors elohim/epr/src/measure.rs (`kind: "ratio"`,
`confidence.claim: "estimated"`) — see epr_meta_measure_ontology_test.py for
the Rust/YAML parity gate this module does not re-derive but must not drift
from.
"""
from __future__ import annotations

import subprocess

DOC_GLOBS = ["genesis/docs/superpowers/specs", "genesis/docs/superpowers/plans"]

# Absorption paths we can count vs. cannot. The gap between them IS the
# interval width.
#   counted:   files deleted or renamed/moved (git log --diff-filter=DR) —
#              a rename INCLUDES a move to held/ (true absorption) but also
#              an in-place rename within the same directory (not absorption
#              at all); this measure cannot distinguish the two, which is why
#              the multiplier below stays wide rather than tight.
#   uncounted: in-place compaction, decompose-to-zero-residue, archive
#              sweeps — none of these touch git's file-identity tracking in a
#              way `--diff-filter` can see.
ABSORPTION_UNCOUNTED_MULTIPLIER = (1.0, 3.0)  # lo, hi — see basis string

# A hung `git log` must never stall session start — a few seconds is plenty
# for a --since-windowed, path-scoped query even on a large repo.
GIT_TIMEOUT_SECONDS = 5


def _git_count(window_days: int, diff_filter: str) -> int:
    """Real implementation: count distinct doc paths touched by `diff_filter`
    events in the last `window_days` days. Raises on subprocess failure
    (missing git, timeout, etc.) — callers that must never crash (e.g.
    session start) are responsible for catching around the call, per the
    same discipline habits-status.py already applies to its other ledgers."""
    if window_days <= 0:
        return 0
    out = subprocess.run(
        ["git", "log", f"--since={window_days} days ago", "--diff-filter=" + diff_filter,
         "--name-only", "--pretty=format:", "--"] + DOC_GLOBS,
        capture_output=True, text=True, check=False, timeout=GIT_TIMEOUT_SECONDS,
    ).stdout
    return len({line for line in out.splitlines() if line.strip()})


def generation_absorption_ratio(window_days: int = 28, *, count_fn=_git_count) -> dict:
    """generated / absorbed over the last `window_days` days, as a `kind:
    "ratio"` measure with an `estimated` confidence.

    `count_fn(window_days, diff_filter) -> int` is injectable for tests —
    defaults to the real git-backed `_git_count` so production callers get
    live counts with no extra wiring. Tests should pass a fixture callable
    that returns fixed counts per diff_filter, so results are deterministic
    and independent of what the last N days of this repo's history happen to
    contain (see doc_dynamics_test.py).
    """
    generated = count_fn(window_days, "A")
    absorbed_counted = count_fn(window_days, "DR")
    lo_mult, hi_mult = ABSORPTION_UNCOUNTED_MULTIPLIER
    lo_absorb = absorbed_counted * lo_mult
    hi_absorb = absorbed_counted * hi_mult

    def ratio(num, den):
        return float("inf") if den <= 0 else num / den

    # A LARGER absorption denominator gives a SMALLER ratio, so the bounds swap.
    value = ratio(generated, (lo_absorb + hi_absorb) / 2 if absorbed_counted else 0)
    return {
        "value": value,
        "kind": "ratio",
        "confidence": {
            "claim": "estimated",
            "interval": {"lo": ratio(generated, hi_absorb), "hi": ratio(generated, lo_absorb)},
            "basis": (
                f"generation witnessed from git log over {window_days}d "
                f"({generated} added); absorption estimated from {absorbed_counted} "
                f"counted delete/rename events (note: rename also counts a plain "
                f"in-place rename, not only a move to held/) × [{lo_mult},{hi_mult}] "
                f"to allow for in-place compaction, decompose-to-zero-residue, and "
                f"archive sweeps that no ledger counts"
            ),
        },
    }
