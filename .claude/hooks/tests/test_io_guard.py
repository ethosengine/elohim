#!/usr/bin/env python3
"""Tests for the write budget (genesis/agentic/bin/io-guard) and the workspace berth (genesis/agentic/bin/berth).

Evidence basis (2026-09-03 10:20Z): the household mesh + prologue seed wrote 130–270 MB/s to the NVMe the
cluster datastore shares; dqlite lease writes hit 3–9.7 s, the devworkspace controller lost leader
election and re-rendered the pod without its secrets. These tests pin the pure decisions: the level
math (rate OR host PSI, PSI-hard wins), the action ladder (pause at high, kill+pause at hard, resume
only when quiet AND something is paused, nothing without dwell), and the berth's honest refusal
(a live holder is named, never queued; a stale holder is taken over on the record).
"""
import importlib.machinery
import importlib.util
import os
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
IO_GUARD = ROOT / "genesis" / "agentic" / "bin" / "io-guard"
BERTH = ROOT / "genesis" / "agentic" / "bin" / "berth"


def load(path, name):
    loader = importlib.machinery.SourceFileLoader(name, str(path))
    spec = importlib.util.spec_from_file_location(name, path, loader=loader)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


io = load(IO_GUARD, "io_guard_daemon")
berth = load(BERTH, "berth_cli")

POL = dict(io.IO_DEFAULTS)


def test_level_rate_ladder():
    assert io.level_for(10, 0, POL) == "ok"
    assert io.level_for(60, 0, POL) == "soft"
    assert io.level_for(100, 0, POL) == "high"
    assert io.level_for(160, 0, POL) == "hard"


def test_level_psi_raises_and_hard_wins_regardless_of_rate():
    assert io.level_for(0, 10, POL) == "high"
    assert io.level_for(0, 20, POL) == "hard"
    assert io.level_for(5, 25, POL) == "hard"


def test_actions_need_dwell():
    assert io.actions_for("high", False, False) == []
    assert io.actions_for("hard", False, False) == []
    assert io.actions_for("high", True, False) == [("pause", "tier1")]
    assert io.actions_for("hard", True, False) == [("kill", "tier1"), ("pause", "tier2")]


def test_resume_only_when_quiet_and_paused():
    assert io.actions_for("ok", True, False) == []
    assert io.actions_for("ok", True, True) == [("resume", None)]
    assert io.actions_for("soft", True, True) == []


def test_write_sensor_rate_over_window():
    s = io.WriteSensor(None, window_s=30)
    s.samples.append((100.0, 0))
    s.samples.append((110.0, 10 * 1048576))
    assert abs(s.rate(110.0) - 1.0) < 1e-6
    s.samples.append((130.0, 70 * 1048576))
    assert abs(s.rate(130.0) - (70 / 30)) < 1e-6


def test_psi_parser(tmp_path=None):
    d = tmp_path or Path(tempfile.mkdtemp())
    p = d / "io"
    p.write_text("some avg10=1.00 avg60=44.50 avg300=3.00 total=1\nfull avg10=21.25 avg60=2.00 avg300=1.00 total=1\n")
    psi = io.WriteSensor.read_psi(str(p))
    assert psi["full_avg10"] == 21.25 and psi["some_avg60"] == 44.5


class Clock:
    def __init__(self, t=1000.0):
        self.t = t

    def __call__(self):
        return self.t


def _berth(tmp):
    os.environ["CLAUDE_PROJECT_DIR"] = "/nonexistent"  # no policy file → default capacities
    clock = Clock()
    b = berth.Berth(d=str(tmp), now=clock)
    return b, clock


def test_claim_refuses_live_holder_and_names_them():
    with tempfile.TemporaryDirectory() as tmp:
        b, clock = _berth(tmp)
        b.moor("s-a", model="m", lab="l", runtime="claude-code")
        b.moor("s-b", model="m", lab="l", runtime="codex")
        ok, _, reason = b.claim("mesh", "s-a", note="0.7 F2")
        assert ok
        ok, lease, reason = b.claim("mesh", "s-b")
        assert not ok and lease["holder"] == "s-a" and "held by s-a" in reason
        assert b.who("mesh")["holder"] == "s-a"
        assert b.ledger()[-1]["kind"] == "refuse"


def test_claim_takes_over_stale_holder_on_the_record():
    with tempfile.TemporaryDirectory() as tmp:
        b, clock = _berth(tmp)
        b.moor("s-a", ttl_s=60)
        b.moor("s-b")
        assert b.claim("mesh", "s-a")[0]
        clock.t += 120  # s-a's mooring expired
        ok, lease, reason = b.claim("mesh", "s-b")
        assert ok and lease["holder"] == "s-b" and "stale" in reason
        ev = b.ledger()[-1]
        assert ev["kind"] == "claim" and ev["took_over_from"] == "s-a"


def test_release_and_unmoor_free_the_resource():
    with tempfile.TemporaryDirectory() as tmp:
        b, clock = _berth(tmp)
        b.moor("s-a")
        b.claim("cargo", "s-a")
        assert b.release("cargo", "s-a")
        assert b.who("cargo") is None
        b.claim("cargo", "s-a")
        b.unmoor("s-a")
        assert b.who("cargo") is None and b.mooring("s-a") is None


def test_mooring_carries_the_commitment_shape():
    with tempfile.TemporaryDirectory() as tmp:
        b, clock = _berth(tmp)
        m = b.moor("s-a", model="claude-fable-5-1", lab="anthropic", runtime="claude-code",
                   principal="human@example", task="lane", writes=["a", "b"])
        assert m["recipient"] == {"model": "claude-fable-5-1", "lab": "anthropic", "runtime": "claude-code", "session": "s-a"}
        assert m["bounds"]["write_set"] == ["a", "b"] and m["bounds"]["ttl_s"] == berth.DEFAULT_TTL_S
        ev = b.ledger()[-1]
        assert ev["kind"] == "moor" and ev["provider"] == "human@example"


if __name__ == "__main__":
    failed = 0
    for name, fn in sorted(globals().items()):
        if name.startswith("test_") and callable(fn):
            try:
                fn()
                print(f"ok   {name}")
            except Exception as e:  # noqa: BLE001
                failed += 1
                print(f"FAIL {name}: {e!r}")
    sys.exit(1 if failed else 0)
