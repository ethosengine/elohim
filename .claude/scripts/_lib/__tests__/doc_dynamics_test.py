"""Generation/absorption ratio (slice-1 Task 5) — the first real consumer of the
measure-dynamics + confidence ontology (Tasks 1/4) applied to our own doc corpus.

Hermetic by construction: `generation_absorption_ratio` takes an injectable
`count_fn(window_days, diff_filter) -> int` (default `_git_count`, the real
git-backed implementation). The four behavioral tests below drive it from
FIXTURE counts so they are deterministic — never dependent on what the last
N days of this repo's actual history happen to contain (the brief's original
tests called the real git-backed implementation directly, which made
`test_zero_absorption_...` fail outright whenever zero doc deletions occurred
in the window; that's a hermeticity bug, not a real assertion about the repo).
One LIVE test (`test_live_ratio_has_well_formed_shape`) calls the real thing
and asserts only shape — kind/claim/basis presence and interval well-
formedness — never specific values.

Run: python3 -m pytest .claude/scripts/_lib/__tests__/doc_dynamics_test.py -v
(targeting this file directly, not the whole __tests__ directory, is
intentional — most sibling files are self-running assert-at-import scripts,
and two unrelated live-artifact fixtures currently abort whole-directory
pytest collection; see CLAUDE.md / this task's report for detail.)
"""
from __future__ import annotations

import sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib.doc_dynamics import generation_absorption_ratio  # noqa: E402


def _fixture_counts(generated: int, absorbed: int):
    """A deterministic stand-in for the real git-backed count_fn: returns
    `generated` for diff_filter "A" and `absorbed` for diff_filter "DR",
    regardless of window_days."""
    def count_fn(window_days: int, diff_filter: str) -> int:
        assert diff_filter in ("A", "DR"), f"unexpected diff_filter {diff_filter!r}"
        return generated if diff_filter == "A" else absorbed
    return count_fn


def test_ratio_is_declared_as_a_ratio_kind():
    r = generation_absorption_ratio(window_days=28, count_fn=_fixture_counts(5, 2))
    assert r["kind"] == "ratio"


def test_absorption_is_honestly_an_estimate_not_a_witness():
    # No ledger counts all four absorption paths, so claiming witnessed here
    # would be exactly the false precision this ontology exists to prevent.
    r = generation_absorption_ratio(window_days=28, count_fn=_fixture_counts(5, 2))
    assert r["confidence"]["claim"] == "estimated"
    assert r["confidence"]["basis"], "an estimate without a basis is uninterpretable"


def test_interval_brackets_the_point_value():
    r = generation_absorption_ratio(window_days=28, count_fn=_fixture_counts(9, 3))
    lo, hi = r["confidence"]["interval"]["lo"], r["confidence"]["interval"]["hi"]
    assert lo <= r["value"] <= hi
    assert hi > lo, "a nonzero-width interval — this quantity is not exactly known"


def test_zero_absorption_yields_an_unbounded_upper_interval():
    # Renamed from the brief's test_zero_absorption_yields_unknown_not_infinity,
    # which asserted interval["hi"] == float("inf") under a name that stated the
    # opposite of what it checks.
    r = generation_absorption_ratio(window_days=28, count_fn=_fixture_counts(5, 0))
    assert r["confidence"]["interval"]["hi"] == float("inf")


def test_live_ratio_has_well_formed_shape():
    """The one live test: calls the real git-backed implementation. Asserts
    only SHAPE (kind/claim/basis present, interval well-formed) — never a
    specific ratio or interval value, since those depend on the live repo's
    actual doc-corpus history in the trailing window."""
    r = generation_absorption_ratio(window_days=28)
    assert r["kind"] == "ratio"
    assert isinstance(r["value"], float)
    confidence = r["confidence"]
    assert confidence["claim"] == "estimated"
    assert isinstance(confidence["basis"], str) and confidence["basis"]
    interval = confidence["interval"]
    lo, hi = interval["lo"], interval["hi"]
    assert isinstance(lo, float) and isinstance(hi, float)
    assert lo <= hi, "interval lower bound must not exceed the upper bound"
    assert lo <= r["value"] <= hi, "the point value must sit inside its own interval"
