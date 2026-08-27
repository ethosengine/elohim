"""The Python half of Q17 — an unknown that says WHY, and the mirror that keeps it honest.

`UnknownReason` lives in `elohim/epr/src/measure.rs`; this module re-declares it as a wire
vocabulary because the memory kit emits measures without touching Rust. Two declarations of one
vocabulary is exactly the shape that has drifted here before (the `HASH_EXCLUDE_KEYS` pair in
`eprfs-meta::canonical` / `epr_meta._HASH_EXCLUDE_KEYS` carries a written warning about it), so
the first test below reads the Rust enum off disk rather than trusting a copy.

The stakes are not tidiness. Row 18 of the measure-family backlog wants an aggregate's
uncertainty decomposed into "which edge, if measured better, most tightens this?" — and
`drift_score.measure` is this repo's live example of an unknown that NOTHING will ever tighten.
If the two vocabularies drift, that exclusion silently stops working and the queue starts
recommending that someone go measure a preference ordering more carefully.

Run: python3 .claude/scripts/_lib/__tests__/signal_measure_unknown_reason_test.py

PLAIN SCRIPT, NOT pytest — deliberately, and the reason is this file's own subject. It was
authored against pytest, which this repo declares NOWHERE and which is not installed in the
devcontainer, so it had never executed here: a drift guard for a vocabulary whose docstring
says it 'has drifted here before', itself silently not running. That is the `#[ignore]`-is-a-
CI-no-op trap one layer up. Its 45 sibling harnesses are plain scripts; conforming costs a
12-line shim and buys execution everywhere, with no dependency to install.
"""
from __future__ import annotations

import re
import sys
import traceback
from contextlib import contextmanager
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
assert REPO is not None

from _lib import drift_score as ds  # noqa: E402
from _lib import signal_measure as sm  # noqa: E402

MEASURE_RS = REPO / "elohim" / "epr" / "src" / "measure.rs"


@contextmanager
def _raises(exc, match: str | None = None):
    """_raises, minus pytest. Same two failure modes: nothing raised, or the message does
    not match — both AssertionError so the runner reports them like any other."""
    try:
        yield
    except exc as e:  # noqa: BLE001 — the caller names the type it wants
        if match is not None and not re.search(match, str(e)):
            raise AssertionError(f"{exc.__name__} raised but {match!r} not in {str(e)!r}") from None
        return
    raise AssertionError(f"expected {exc.__name__}, nothing raised")


def needs_rust(fn):
    """Marks a test that reads `measure.rs`. The runner SKIPS it LOUDLY when the Rust source is
    absent — a skip that prints nothing is the failure this file exists to catch."""
    fn._needs_rust = True
    return fn



def _rust_unknown_reasons() -> list[str]:
    """Variant names from the `UnknownReason` enum body, in kebab-case (its serde rename)."""
    src = MEASURE_RS.read_text()
    body = src.split("pub enum UnknownReason {", 1)[1].split("\n}", 1)[0]
    variants = re.findall(r"^\s{4}([A-Z][A-Za-z0-9]*),\s*$", body, re.M)
    return [re.sub(r"(?<!^)(?=[A-Z])", "-", v).lower() for v in variants]


def _rust_fn_body(name: str) -> str:
    """The source text of one `pub fn <name>` up to the next top-level `pub fn`."""
    src = MEASURE_RS.read_text()
    after = src.split(f"pub fn {name}", 1)[1]
    return after.split("\n    pub fn ", 1)[0]


@needs_rust
def test_the_python_vocabulary_is_the_rust_enum_and_not_a_copy_of_it():
    # Read from the source of truth, not asserted against a second literal — a hand-maintained
    # duplicate would pass this test on the day it drifted.
    #
    # ORDER-sensitive, not just set-equal. A list position here is the tie-break `reduce_reasons`
    # uses, and `UnknownReason::stable_index` is the same tie-break in Rust: if the two sequences
    # diverge, the two sides reduce an equal-rank multiset to DIFFERENT reasons while every
    # set-equality check stays green.
    assert list(sm.UNKNOWN_REASONS) == _rust_unknown_reasons()
    # And every one of them classifies, so a new Rust variant cannot arrive unclassified.
    assert set(sm.TIGHTENABLE) == set(sm.UNKNOWN_REASONS)


@needs_rust
def test_the_tie_break_index_matches_the_rust_one_position_for_position():
    # `stable_index` is written out in Rust rather than derived precisely so it can be read back
    # here. Its numbers ARE the declaration order; this asserts Python's list agrees.
    body = _rust_fn_body("stable_index")
    pairs = re.findall(r"UnknownReason::([A-Za-z0-9]+) => (\d+),", body)
    rust_index = {re.sub(r"(?<!^)(?=[A-Z])", "-", v).lower(): int(i) for v, i in pairs}
    assert rust_index == {r: i for i, r in enumerate(sm.UNKNOWN_REASONS)}


def test_the_three_responses_stay_distinguishable():
    # The distinction row 18 is built on: measurable ignorance, a structure problem that more
    # measurement cannot touch, and the kind nothing will ever fix.
    assert sm.tightenable("no-observations") == "by-measurement"
    assert sm.tightenable("not-yet-instrumented") == "by-measurement"
    assert sm.tightenable("undefined-division") == "by-structure"
    assert sm.tightenable("incommensurable") == "never"
    with _raises(ValueError):
        sm.tightenable("made-up")


def test_a_measure_without_a_reason_emits_exactly_what_it_always_did():
    # The non-breaking half. The key is absent, not null — same shape as the Rust field's
    # `skip_serializing_if`, so nothing downstream that reads `confidence` sees a new key.
    m = sm.measure(1.0, "ratio", basis="fixture")
    assert set(m["confidence"]) == {"claim", "interval", "basis"}
    unknown_no_reason = sm.measure(None, "ratio", basis="fixture")
    assert sm.is_unknown(unknown_no_reason["confidence"]["interval"])
    assert "unknownReason" not in unknown_no_reason["confidence"]


def test_a_reason_is_refused_on_a_bounded_interval_and_on_a_word_we_do_not_know():
    with _raises(ValueError, match="ABSENCE"):
        sm.measure(1.0, "ratio", basis="fixture", unknown_reason="no-observations")
    with _raises(ValueError, match="not in"):
        sm.measure(None, "ratio", basis="fixture", unknown_reason="probably-fine")


def test_the_live_incommensurable_composite_is_machine_excludable():
    # `drift_score` is the repo's own confessed incommensurable (spec Q16): a weighted sum of
    # normalized time and log-counts, whose weights move when a human changes their mind. Before
    # Q17 the confession lived only in its `basis` prose, where no queue could read it.
    m = ds.measure({"scope_edits": 2, "direct_edits": 1, "structural_edits": 0})
    assert sm.is_unknown(m["confidence"]["interval"])
    assert m["confidence"]["unknownReason"] == "incommensurable"
    assert sm.tightenable(m["confidence"]["unknownReason"]) == "never", (
        "this score must never appear in a 'go measure it better' queue"
    )
    # The prose confession stays too — the enum is for machines, the basis is for readers.
    assert "INCOMMENSURABLE" in m["confidence"]["basis"]


# --------------------------------------------------------------- the reduction, mirrored

def test_the_reduction_is_least_tightenable_wins_and_order_independent():
    # Mirrors `UnknownReason::reduce`. The ordering half is not cosmetic: `tightenable()` maps
    # seven reasons onto three responses, so distinct reasons TIE, and a reduction keyed on the
    # response alone resolves ties by argument order. In Rust that shipped as an order-dependent
    # fold; here it would mean two callers emitting different measures from the same inputs.
    assert sm.reduce_reasons(["no-observations", "incommensurable"]) == "incommensurable"
    assert sm.reduce_reasons(["no-observations", "zero-base"]) == "zero-base"
    assert sm.reduce_reasons([]) is None
    assert sm.reduce_reasons([None, None]) is None, "no contributor, no claim"
    assert sm.reduce_reasons(["zero-base"]) == "zero-base"

    # Every unordered pair, both directions — a tie-break that works for one pair is not a total
    # order.
    for i, a in enumerate(sm.UNKNOWN_REASONS):
        for b in sm.UNKNOWN_REASONS[i:]:
            assert sm.reduce_reasons([a, b]) == sm.reduce_reasons([b, a]), f"{a} vs {b}"

    with _raises(ValueError):
        sm.reduce_reasons(["made-up"])


# --------------------------------------------------------------- MINT-SITE parity

def test_the_one_arithmetic_mint_site_classifies_every_absence_it_produces():
    # Vocabulary parity is not mint-site parity. `ratio_of_rates` is documented as mirroring
    # `stock::respite_response`, which mints `UndefinedDivision` through `div_because` — and on
    # the day the vocabulary landed, this function still returned a bare unknown at both of its
    # absence branches. An unclassified unknown is exactly what row 18's queue cannot rank, so a
    # mirror that carries the words but not the mints is decoration.
    def rate(v, per="week", reason=None):
        return sm.measure(v, "rate", per=per, basis="fixture", claim="witnessed",
                          unknown_reason=reason)

    # A witnessed zero response rate: the band admits zero, same guard `div_because` fires.
    out = sm.ratio_of_rates(rate(4.0), rate(0.0), basis="fixture")
    assert out["value"] is None
    assert out["confidence"]["unknownReason"] == "undefined-division"
    assert sm.tightenable(out["confidence"]["unknownReason"]) == "by-structure"

    # An absent response rate is the same shape — an unknown band admits zero too.
    assert sm.ratio_of_rates(
        rate(4.0), rate(None), basis="fixture")["confidence"]["unknownReason"] == (
        "undefined-division")

    # An absent NUMERATOR over a good denominator is the measurable class, and must not be
    # laundered into the structural one — "go build the counter" is real, rankable work.
    absent_num = sm.ratio_of_rates(rate(None), rate(2.0), basis="fixture")
    assert absent_num["confidence"]["unknownReason"] == "no-observations"
    assert sm.tightenable(absent_num["confidence"]["unknownReason"]) == "by-measurement"

    # Bounded both sides: no absence, and therefore no reason key at all.
    fine = sm.ratio_of_rates(rate(4.0), rate(2.0), basis="fixture")
    assert fine["value"] == 2.0
    assert "unknownReason" not in fine["confidence"]


def test_an_operands_declared_reason_is_threaded_and_reduced_against_the_guard():
    # The other half of the Rust fix, mirrored: the guards see only values, so a numerator that
    # already declared WHY it is unknown has to be threaded in — and reduced, not overridden.
    def rate(v, per="week", reason=None):
        return sm.measure(v, "rate", per=per, basis="fixture", claim="modelled",
                          unknown_reason=reason)

    # `incommensurable` (never) beats the zero-denominator guard (by-structure): fixing the model
    # cannot help when the numerator is a preference ordering.
    both = sm.ratio_of_rates(rate(None, reason="incommensurable"), rate(0.0), basis="fixture")
    assert both["confidence"]["unknownReason"] == "incommensurable"
    assert sm.tightenable(both["confidence"]["unknownReason"]) == "never"

    # And the guard wins over a merely-measurable operand reason.
    structural = sm.ratio_of_rates(rate(None, reason="no-observations"), rate(0.0),
                                   basis="fixture")
    assert structural["confidence"]["unknownReason"] == "undefined-division"

    # A declared reason on the numerator refines the `no-observations` default when the
    # denominator is fine — an uninstrumented signal is a different queue entry from an empty one.
    refined = sm.ratio_of_rates(rate(None, reason="not-yet-instrumented"), rate(2.0),
                                basis="fixture")
    assert refined["confidence"]["unknownReason"] == "not-yet-instrumented"


@needs_rust
def test_the_zero_denominator_mint_names_the_same_variant_as_the_rust_guard():
    # Cheapest possible cross-boundary pin: read the Rust guard's own variant rather than
    # trusting the docstring above `ratio_of_rates`. A rename or a reclassification on the Rust
    # side now fails HERE instead of silently leaving the Python mint behind, which is the drift
    # that produced this defect in the first place.
    body = _rust_fn_body("div_because")
    assert "other.lo <= 0.0 && other.hi >= 0.0" in body, "the zero-crossing guard moved"
    minted = re.findall(r"UnknownReason::([A-Za-z0-9]+)", body)
    assert "UndefinedDivision" in minted
    zero_den = sm.ratio_of_rates(
        sm.measure(4.0, "rate", per="week", basis="f"),
        sm.measure(0.0, "rate", per="week", basis="f"),
        basis="f",
    )
    assert zero_den["confidence"]["unknownReason"] == "undefined-division"
    # Every reason the Rust arithmetic can mint must exist in the Python vocabulary.
    for v in set(minted):
        assert re.sub(r"(?<!^)(?=[A-Z])", "-", v).lower() in sm.UNKNOWN_REASONS


# ───────────────────────────── runner ─────────────────────────────

if __name__ == "__main__":
    _tests = [(n, f) for n, f in sorted(globals().items())
              if n.startswith("test_") and callable(f)]
    _passed = _skipped = 0
    _failed: list[str] = []
    for _name, _fn in _tests:
        if getattr(_fn, "_needs_rust", False) and not MEASURE_RS.is_file():
            print(f"  SKIP {_name} — {MEASURE_RS.relative_to(REPO)} absent from this checkout")
            _skipped += 1
            continue
        try:
            _fn()
        except Exception:  # noqa: BLE001 — a harness reports, it does not propagate
            print(f"  FAIL {_name}")
            print("".join("       " + l for l in traceback.format_exc().splitlines(True)))
            _failed.append(_name)
        else:
            print(f"  ok   {_name}")
            _passed += 1
    _tail = f", {_skipped} skipped" if _skipped else ""
    print(f"\n{_passed}/{len(_tests) - _skipped} passed{_tail}")
    sys.exit(1 if _failed else 0)
