"""Measured breaker behavior for the per-prompt run projection."""
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / ".claude/hooks/run-projection.py"
SPEC = importlib.util.spec_from_file_location("run_projection_under_test", HOOK)
RUN_PROJECTION = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUN_PROJECTION)


class Observation:
    @staticmethod
    def resolve_bin():
        return "/fixture/epr"

    @staticmethod
    def bootstrap_session_id(root, payload):
        return "fixture-session"


class Process:
    def __init__(self, *, timeout=False, stdout="ready\n", stderr="", returncode=0):
        self.pid = 4242
        self.timeout = timeout
        self.stdout = stdout
        self.stderr = stderr
        self.returncode = returncode
        self.calls = 0

    def communicate(self, timeout=None):
        self.calls += 1
        if self.timeout and self.calls == 1:
            raise subprocess.TimeoutExpired("epr", timeout)
        return self.stdout, self.stderr


class RunProjectionBreakerTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.breaker = self.root / ".eprfs/status/run-projection/breaker.json"
        self.ledger = self.root / ".eprfs/status/run-projection/overruns.jsonl"

    def project(self, process):
        with mock.patch.object(RUN_PROJECTION, "_observation_module", return_value=Observation()), \
                mock.patch("subprocess.Popen", return_value=process) as popen, \
                mock.patch.object(RUN_PROJECTION.os, "killpg"), \
                mock.patch.object(RUN_PROJECTION.os, "getloadavg", return_value=(21.0, 0.0, 0.0)):
            lines = RUN_PROJECTION.run_plane_lines(self.root, {})
        return lines, popen

    def test_overrun_opens_breaker_and_records_ledger_row(self):
        lines, popen = self.project(Process(timeout=True))
        self.assertEqual(lines, [f"bootstrap: skipped — recall open exceeded the {RUN_PROJECTION.TIMEOUT_S}s budget"])
        popen.assert_called_once()
        state = json.loads(self.breaker.read_text())
        [row] = [json.loads(line) for line in self.ledger.read_text().splitlines()]
        self.assertEqual(row, state)
        self.assertEqual((state["loadAverage1m"], state["consecutiveOverruns"], state["backoffSeconds"]),
                         (21.0, 1, 60))
        self.assertGreaterEqual(state["elapsedSeconds"], 0)

    def test_prompt_inside_window_skips_without_spawning(self):
        self.project(Process(timeout=True))
        with mock.patch.object(RUN_PROJECTION, "_observation_module", return_value=Observation()), \
                mock.patch("subprocess.Popen") as popen:
            [line] = RUN_PROJECTION.run_plane_lines(self.root, {})
        popen.assert_not_called()
        state = json.loads(self.breaker.read_text())
        self.assertIn(state["overrunAt"], line)
        self.assertIn(f'{state["elapsedSeconds"]:.3f}s', line)
        self.assertIn("retry at ", line)

    def test_prompt_after_window_spawns(self):
        self.project(Process(timeout=True))
        state = json.loads(self.breaker.read_text())
        state["overrunAtEpoch"] -= state["backoffSeconds"] + 1
        self.breaker.write_text(json.dumps(state))
        lines, popen = self.project(Process(stdout="one\ntwo\n"))
        popen.assert_called_once()
        self.assertEqual(lines, ["one", "two"])

    def test_a_wall_clock_set_back_before_the_overrun_does_not_hold_the_breaker_open(self):
        self.project(Process(timeout=True))
        state = json.loads(self.breaker.read_text())
        state["overrunAtEpoch"] += 3600  # the clock now reads an hour before the overrun
        self.breaker.write_text(json.dumps(state))
        lines, popen = self.project(Process(stdout="one\n"))
        popen.assert_called_once()
        self.assertEqual(lines, ["one"])

    def test_consecutive_overrun_doubles_backoff(self):
        self.project(Process(timeout=True))
        state = json.loads(self.breaker.read_text())
        state["overrunAtEpoch"] -= state["backoffSeconds"] + 1
        self.breaker.write_text(json.dumps(state))
        self.project(Process(timeout=True))
        state = json.loads(self.breaker.read_text())
        self.assertEqual((state["consecutiveOverruns"], state["backoffSeconds"]), (2, 120))
        self.assertEqual(len(self.ledger.read_text().splitlines()), 2)

    def test_in_budget_run_resets_breaker(self):
        self.project(Process(timeout=True))
        state = json.loads(self.breaker.read_text())
        state["overrunAtEpoch"] -= state["backoffSeconds"] + 1
        self.breaker.write_text(json.dumps(state))
        self.project(Process(returncode=7, stderr="measured failure\n"))
        self.assertFalse(self.breaker.exists())
        self.assertTrue(self.ledger.exists(), "reset keeps the measurement ledger")

    def test_corrupt_state_is_ignored(self):
        self.breaker.parent.mkdir(parents=True)
        self.breaker.write_text("{not json")
        lines, popen = self.project(Process(stdout="fresh\n"))
        popen.assert_called_once()
        self.assertEqual(lines, ["fresh"])
        self.assertFalse(self.breaker.exists())


if __name__ == "__main__":
    unittest.main()
