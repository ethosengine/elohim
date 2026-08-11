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

from _lib.doc_dynamics import (  # noqa: E402
    corpus_dynamics,
    doc_corpus_stock,
    generation_absorption_ratio,
)


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


def test_zero_absorption_interval_has_positive_width():
    # Regression guard, fix round 1 (2026-08-11): lo_absorb == hi_absorb == 0
    # when absorbed_counted == 0 used to collapse BOTH ratio() bounds onto the
    # same den<=0 branch, producing a ZERO-WIDTH {inf, inf} interval — a claim
    # of certainty at the exact moment data is most absent, and ill-formed
    # under Interval::width() (inf - inf = NaN). The live shape test's
    # `lo <= hi` alone would NOT have caught this (satisfied by equality), so
    # this test asserts positive width specifically for the zero-absorption
    # case. Sealed law L3 (elohim/epr/src/measure.rs Interval::unknown())
    # requires this epistemic state to render as the interval that admits
    # everything: lo=-inf, hi=+inf.
    r = generation_absorption_ratio(window_days=28, count_fn=_fixture_counts(62, 0))
    iv = r["confidence"]["interval"]
    assert iv["lo"] == float("-inf")
    assert iv["hi"] == float("inf")
    assert iv["hi"] > iv["lo"], (
        "zero-absorption interval must have positive (unbounded) width, "
        "never collapse to a zero-width point"
    )


# ------------------------------------------------------------------ slice 2: the stock shape

def _stock_fns(generated: int, absorbed: int, in_place: int = 0, level: int = 300):
    """Deterministic stand-in for the (added, absorbed, level) triple."""
    return (lambda w: generated, lambda w: (absorbed, in_place), lambda: level)


def test_the_corpus_is_modelled_as_a_stock_not_just_a_ratio():
    """A ratio with no level behind it cannot answer the turnover question — the one that
    actually separates dynamic equilibrium from silting. Slice 1 emitted half a model."""
    s = doc_corpus_stock(28, count_fns=_stock_fns(28, 14, level=300))
    assert s["level"]["kind"] == "level"
    assert s["inflow"]["kind"] == "rate" and s["inflow"]["per"] == "week"
    assert s["outflow"]["kind"] == "rate" and s["outflow"]["per"] == "week"
    assert s["emission_absorption"]["kind"] == "ratio"
    # 28 authored over 4 weeks = 7/wk; 14 absorbed = 3.5/wk; index 2.0.
    assert s["inflow"]["value"] == 7.0
    assert s["outflow"]["value"] == 3.5
    assert s["emission_absorption"]["value"] == 2.0


def test_the_window_is_named_in_every_basis_not_left_as_an_argument_default():
    """Slice 2 item C. In the first live run the 28d default ALONE decided whether the headline
    read `unknown` or `3.2` — a measure whose conclusion flips on an undeclared parameter is not
    yet honest, so the window rides in the basis where a reader can disagree with it."""
    s = doc_corpus_stock(90, count_fns=_stock_fns(90, 30))
    for leg in ("inflow", "outflow", "emission_absorption", "turnover_time"):
        assert "90d window" in s[leg]["confidence"]["basis"], leg


def test_several_windows_are_reported_because_absorption_is_bursty():
    spans = [s["window_days"] for s in corpus_dynamics(count_fns=_stock_fns(10, 5))]
    assert len(spans) > 1 and spans == sorted(spans)


def test_an_in_place_rename_is_not_absorption():
    """THE DISCRIMINATION SLICE 1 COULD NOT MAKE, and the same judgment the Rust projection
    makes against the recipe's stage globs: a move to `held/` left the plate; a rename inside
    the corpus did not. Slice 1's `--diff-filter=DR` counted both, which is why its estimate had
    to stay wide."""
    s = doc_corpus_stock(28, count_fns=_stock_fns(28, absorbed=14, in_place=6))
    assert s["counts"]["absorbed"] == 14
    assert s["counts"]["in_place_renames"] == 6
    assert "6 in-place renames excluded" in s["outflow"]["confidence"]["basis"], (
        "what was NOT counted belongs in the basis — a number that quietly improved is worse "
        "than one that says why")


def test_zero_absorption_gives_unknown_and_a_value_that_refuses_to_compare():
    """Spec Q12 + Q13 mirrored from the Rust. The interval must be honest absence AND the value
    must not silently satisfy `> 1.0` — either leg alone still lets a consumer read 'confirmed
    overshoot' off an absence, which is the exact misreading this ontology exists to prevent."""
    s = doc_corpus_stock(28, count_fns=_stock_fns(64, 0))
    idx = s["emission_absorption"]
    assert idx["confidence"]["interval"] == {"lo": float("-inf"), "hi": float("inf")}
    assert idx["value"] is None, "unknowable must not be a number that compares"
    assert s["turnover_time"]["value"] is None


def test_an_entirely_above_one_interval_is_confirmed_overshoot_not_a_central_estimate():
    """The distinction the ⚠/~ flag turns on: `lo > 1.0` means overshoot at EVERY plausible
    absorption rate, including the most generous 3x widening — a far stronger claim than a
    central value above 1.0 with a band straddling it."""
    s = doc_corpus_stock(90, count_fns=_stock_fns(320, 98))
    iv = s["emission_absorption"]["confidence"]["interval"]
    assert iv["lo"] > 1.0, "even 3x the counted absorption does not bring this under 1.0"
    assert iv["lo"] < iv["hi"]


def test_turnover_is_a_duration_and_scales_with_the_level():
    """Turnover distinguishes a corpus that is silting from one turning over. Two corpora with
    identical flows and different levels have different turnovers — which is precisely why the
    level had to be modelled rather than inferred from flow arithmetic."""
    small = doc_corpus_stock(28, count_fns=_stock_fns(28, 14, level=100))
    large = doc_corpus_stock(28, count_fns=_stock_fns(28, 14, level=400))
    assert small["turnover_time"]["kind"] == "level", "a duration, not a ratio"
    assert large["turnover_time"]["value"] == 4 * small["turnover_time"]["value"]


def test_the_live_stock_has_a_well_formed_shape():
    """The one live leg: real git, shape only — never a specific value, which depends on what
    the trailing window of this repo's history happens to contain."""
    for s in corpus_dynamics():
        assert s["level"]["value"] >= 0
        for leg in ("level", "inflow", "outflow", "emission_absorption", "turnover_time"):
            iv = s[leg]["confidence"]["interval"]
            assert iv["lo"] <= iv["hi"], f"{leg} interval is malformed"
            assert s[leg]["confidence"]["basis"], f"{leg} has no basis"
            v = s[leg]["value"]
            if v is not None:
                assert iv["lo"] <= v <= iv["hi"], f"{leg} value sits outside its own interval"


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
