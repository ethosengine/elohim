"""The ceremony's own firing gate, denominated.

`THRESHOLD = 120` is documented as *"calibrated to roughly a WEEK OF HEAVY DEVELOPMENT"* — a
per-week quantity. `pressure` was a bare cumulative count since an unrecorded moment, so 210
items in five days and 210 in five weeks produced an identical reading, and only one of those
says the system is losing. The reset stamp is what makes the difference expressible.

This matters more than a tidier number: this gate decides WHEN the memory ceremony fires, so a
mis-denominated reading is Meadows' third cause of overshoot — a delay or error in perceiving
the limit — sitting directly on the loop that exists to prevent overshoot.

Run: python3 -m pytest .claude/scripts/_lib/__tests__/cleanup_pressure_rate_test.py -v
"""
from __future__ import annotations

import importlib.util
import json
import os
import sys
from datetime import datetime, timedelta, timezone
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

# cleanup-pressure.py is hyphenated, so it is loaded by path rather than imported by name.
_spec = importlib.util.spec_from_file_location(
    "cleanup_pressure", REPO / ".claude" / "scripts" / "memory-kit" / "cleanup-pressure.py")
cp = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(cp)


def _kit(tmp_path, drifted: int, cycles=None):
    """A synthetic memory-kit: one accumulator holding `drifted` items, plus optional history."""
    kit = tmp_path / ".claude" / "memory-kit"
    kit.mkdir(parents=True)
    (kit / "placement-drift.json").write_text(json.dumps(
        {"schema_version": 1, "due": {f"doc-{i}.md": 1 for i in range(drifted)}}))
    if cycles is not None:
        (kit / cp.CYCLE_FILE).write_text(json.dumps({"schema_version": 1, "cycles": cycles}))
    return tmp_path


def _ago(**kw):
    return (datetime.now(timezone.utc) - timedelta(**kw)).isoformat(timespec="seconds")


def test_without_a_stamped_reset_the_rate_is_unknown_not_guessed():
    """The whole defect, asserted. An unrecorded start date means the elapsed time is unknown,
    which means the rate is unknown — and an unknown rate must NOT become a zero or an infinity,
    because both of those compare against a threshold and an absence must not."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 210)
        dyn = cp.dynamics(root)
        assert dyn["growth"]["value"] is None
        assert dyn["growth"]["kind"] == "rate" and dyn["growth"]["per"] == "week"
        assert dyn["growth"]["confidence"]["interval"] == {"lo": float("-inf"),
                                                           "hi": float("inf")}
        assert "not computable" in dyn["growth"]["confidence"]["basis"]


def test_the_same_count_reads_differently_at_different_elapsed_times():
    """THE POINT. 210 items is over a 120/week threshold when it arrived in five days and well
    under it when it took five weeks. Before the stamp, those were the same reading."""
    import tempfile
    readings = {}
    for label, days in (("fast", 5), ("slow", 35)):
        with tempfile.TemporaryDirectory() as d:
            root = _kit(Path(d), 210, cycles=[{"reset_at": _ago(days=days),
                                               "pressure_at_reset": 100}])
            readings[label] = cp.dynamics(root)["growth"]["value"]
    assert readings["fast"] > cp.THRESHOLD, "210 in five days is a losing week"
    assert readings["slow"] < cp.THRESHOLD, "210 over five weeks is not"
    assert readings["fast"] > readings["slow"] * 5


def test_the_level_gate_is_unchanged_and_the_rate_gates_nothing():
    """Migrating `due` to the rate would change WHEN the ceremony fires. That is a governance
    trigger, and changing one is the operator's call — so the rate is reported as evidence for
    that decision and deliberately gates nothing."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 210, cycles=[{"reset_at": _ago(days=35), "pressure_at_reset": 100}])
        st = cp.status(root, as_json=True)
        assert st["due"] is True, "the level gate still fires, unchanged"
        assert st["rate_due"] is False, "and the rate disagrees with it — that is the finding"
        assert st["threshold_per"] == "week"


def test_reset_stamps_the_cycle_and_records_what_it_drained():
    """`pressure_at_reset` is captured BEFORE zeroing because it is the numerator of the RESPONSE
    rate the next reading needs. Zeroing first would discard the only evidence of how much the
    cycle actually absorbed."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 210)
        cp.reset(root)
        state = json.loads((root / ".claude" / "memory-kit" / cp.CYCLE_FILE).read_text())
        assert len(state["cycles"]) == 1
        assert state["cycles"][0]["pressure_at_reset"] == 210
        assert state["cycles"][0]["reset_at"]
        # ...and the accumulators are actually drained.
        assert cp.activity(str(root / ".claude" / "memory-kit")) == 0


def test_two_cycles_make_respite_response_computable():
    """Meadows' controllability index applied to the ceremony itself. One cycle gives a growth
    rate; the SECOND gives a response rate, and only then can the ratio say whether firing the
    loop more often could ever close the gap."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 200, cycles=[
            {"reset_at": _ago(days=28), "pressure_at_reset": 150},
            {"reset_at": _ago(days=14), "pressure_at_reset": 70},
        ])
        dyn = cp.dynamics(root)
        # 200 items over 2 weeks = 100/wk arriving; the closed cycle drained 70 over 2 weeks = 35/wk.
        assert dyn["growth"]["value"] > 0
        assert dyn["response"]["value"] > 0
        ratio = dyn["controllability"]["value"]
        assert ratio is not None and ratio > 1.0, (
            "drift arriving faster than the loop absorbs it — the reading that says 'try harder' "
            "is not on the menu")
        assert dyn["controllability"]["kind"] == "ratio"


def test_a_single_cycle_leaves_the_ratio_honestly_unknown():
    """One stamped cycle is not enough for a response rate, and a response rate invented from one
    data point would be exactly the false precision this ontology exists to prevent."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 200, cycles=[{"reset_at": _ago(days=14), "pressure_at_reset": 150}])
        dyn = cp.dynamics(root)
        assert dyn["growth"]["value"] is not None, "growth IS computable from one stamp"
        assert dyn["response"]["value"] is None
        assert dyn["controllability"]["value"] is None


def test_the_cycle_history_is_bounded_so_it_never_becomes_its_own_dump():
    """An unbounded append-only file inside the memory kit would be the kit generating exactly
    the debt it exists to drain."""
    import tempfile
    with tempfile.TemporaryDirectory() as d:
        root = _kit(Path(d), 5, cycles=[{"reset_at": _ago(days=i + 1), "pressure_at_reset": i}
                                        for i in range(cp.MAX_CYCLES + 10)])
        cp.reset(root)
        state = json.loads((root / ".claude" / "memory-kit" / cp.CYCLE_FILE).read_text())
        assert len(state["cycles"]) == cp.MAX_CYCLES


def test_a_rate_cannot_be_divided_by_a_level():
    """The refusal shared with `stock::respite_response`: the controllability index divides two
    RATES, and a level in either position is the unit error the whole vocabulary exists to
    refuse."""
    from _lib import signal_measure as sm
    rate = sm.measure(10.0, "rate", per="week", basis="fixture")
    level = sm.measure(10.0, "level", basis="fixture")
    for bad in ((rate, level), (level, rate)):
        try:
            sm.ratio_of_rates(*bad, basis="fixture")
            raise AssertionError("expected a refusal")
        except ValueError as e:
            assert "RATE" in str(e) or "level" in str(e)


def test_rates_of_different_tempo_refuse_to_divide():
    from _lib import signal_measure as sm
    per_week = sm.measure(10.0, "rate", per="week", basis="fixture")
    per_day = sm.measure(10.0, "rate", per="day", basis="fixture")
    try:
        sm.ratio_of_rates(per_week, per_day, basis="fixture")
        raise AssertionError("expected a refusal")
    except ValueError as e:
        assert "different periods" in str(e)


def test_a_rate_cannot_be_declared_without_its_period():
    from _lib import signal_measure as sm
    try:
        sm.measure(1.0, "rate", basis="fixture")
        raise AssertionError("expected a refusal")
    except ValueError as e:
        assert "forget its denominator" in str(e)


def test_the_live_gate_still_answers():
    """Shape only — the live repo's drift state is whatever it is. What must hold is that the
    instrument answers at all and keeps its documented keys."""
    st = cp.status(str(REPO), as_json=True)
    for key in ("pressure", "threshold", "due", "threshold_per", "rate_per_week", "rate_due"):
        assert key in st, key
    assert isinstance(st["due"], bool)
