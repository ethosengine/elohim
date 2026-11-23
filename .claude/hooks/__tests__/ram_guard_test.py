#!/usr/bin/env python3
"""ram-guard hook: time-based session cursor, date format, SessionStart deadline.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'ram_guard_test.py'
"""
from __future__ import annotations

import importlib.machinery
import importlib.util
import json
import os
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent


def load():
    path = str(HOOKS / "ram-guard.py")
    loader = importlib.machinery.SourceFileLoader("ram_guard_under_test", path)
    spec = importlib.util.spec_from_file_location("ram_guard_under_test", path, loader=loader)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


rg = load()


def ev(ts, n=0):
    return {"ts": ts, "tier": "compile", "comm": "cargo", "rss_kb": 2048, "age_s": 5, "pct": 90, "cmd": f"cargo build {n}"}


def write_ledger(d, events):
    with open(os.path.join(d, "events.jsonl"), "w") as f:
        for e in events:
            f.write(json.dumps(e) + "\n")


def append(d, e):
    with open(os.path.join(d, "events.jsonl"), "a") as f:
        f.write(json.dumps(e) + "\n")


class Store(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.d = self._t.name

    def tearDown(self):
        self._t.cleanup()


class CursorAtSessionStart(Store):
    def test_old_events_are_not_since_your_last_look(self):
        write_ledger(self.d, [ev(1.0, i) for i in range(31)])
        rg.start_session_cursor(self.d, "new", "startup")
        self.assertEqual(rg.unseen_events(self.d, "new"), [])

    def test_events_during_the_session_still_show(self):
        write_ledger(self.d, [ev(1.0, i) for i in range(5)])
        rg.start_session_cursor(self.d, "s", "startup")
        append(self.d, ev(2.0, 99))
        got = rg.unseen_events(self.d, "s")
        self.assertEqual([e["cmd"] for e in got], ["cargo build 99"])
        self.assertEqual(rg.unseen_events(self.d, "s"), [])

    def test_no_ledger_is_fine(self):
        self.assertEqual(rg.start_session_cursor(self.d, "s"), 0)
        self.assertEqual(rg.unseen_events(self.d, "s"), [])


def trim_head(d, keep):
    """What the daemon's StateStore.append_event does: drop events from the head of the ledger."""
    p = Path(d, "events.jsonl")
    lines = p.read_text().splitlines(True)
    p.write_text("".join(lines[-keep:]))


def cursor_text(d, s):
    return Path(rg._cursor_path(d, s)).read_text()


class TimeCursor(Store):
    def test_survives_a_daemon_trim_and_shows_a_later_event_once(self):
        write_ledger(self.d, [ev(float(i), i) for i in range(1, 11)])
        rg.start_session_cursor(self.d, "s", "startup")
        trim_head(self.d, 3)  # head dropped: a line-count cursor would now point past the end
        self.assertEqual(rg.unseen_events(self.d, "s"), [])
        append(self.d, ev(50.0, 77))
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "s")], ["cargo build 77"])
        self.assertEqual(rg.unseen_events(self.d, "s"), [])
        trim_head(self.d, 2)
        self.assertEqual(rg.unseen_events(self.d, "s"), [])

    def test_trim_does_not_hide_an_unseen_event(self):
        write_ledger(self.d, [ev(1.0, 1), ev(2.0, 2)])
        rg.start_session_cursor(self.d, "s", "startup")
        for i in (3, 4, 5):
            append(self.d, ev(float(i), i))
        trim_head(self.d, 2)  # drops 3 of the 5; the two newest remain
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "s")], ["cargo build 4", "cargo build 5"])

    def test_two_sessions_do_not_disturb_each_other(self):
        write_ledger(self.d, [ev(1.0, 1)])
        rg.start_session_cursor(self.d, "a", "startup")
        rg.start_session_cursor(self.d, "b", "startup")
        append(self.d, ev(2.0, 2))
        self.assertEqual(len(rg.unseen_events(self.d, "a")), 1)
        self.assertEqual(rg.unseen_events(self.d, "a"), [])
        append(self.d, ev(3.0, 3))
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "b")], ["cargo build 2", "cargo build 3"])
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "a")], ["cargo build 3"])
        self.assertEqual(rg.unseen_events(self.d, "b"), [])

    def test_ledger_is_never_rewritten_by_the_hook(self):
        write_ledger(self.d, [ev(float(i), i) for i in range(600)])
        before = Path(self.d, "events.jsonl").read_text()
        rg.start_session_cursor(self.d, "s", "startup")
        rg.unseen_events(self.d, "s")
        self.assertEqual(Path(self.d, "events.jsonl").read_text(), before)
        self.assertFalse(hasattr(rg, "LEDGER_CAP"))


class LegacyCursor(Store):
    def put(self, text):
        os.makedirs(os.path.join(self.d, "seen"), exist_ok=True)
        with open(rg._cursor_path(self.d, "old"), "w") as f:
            f.write(text)

    def test_integer_cursor_is_converted_to_the_last_seen_events_ts(self):
        write_ledger(self.d, [ev(100.0 + i, i) for i in range(5)])
        self.put("3")  # lines 0-2 seen
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "old")], ["cargo build 3", "cargo build 4"])
        self.assertEqual(float(cursor_text(self.d, "old")), 104.0)

    def test_integer_cursor_past_the_end_becomes_the_newest_ts(self):
        write_ledger(self.d, [ev(100.0 + i, i) for i in range(4)])
        self.put("10")
        self.assertEqual(rg.unseen_events(self.d, "old"), [])
        self.assertEqual(float(cursor_text(self.d, "old")), 103.0)
        append(self.d, ev(200.0, 9))
        self.assertEqual(len(rg.unseen_events(self.d, "old")), 1)

    def test_zero_legacy_cursor_shows_everything_once(self):
        write_ledger(self.d, [ev(100.0 + i, i) for i in range(2)])
        self.put("0")
        self.assertEqual(len(rg.unseen_events(self.d, "old")), 2)
        self.assertEqual(rg.unseen_events(self.d, "old"), [])


class SessionSource(Store):
    def test_compact_leaves_the_cursor_so_an_earlier_shed_still_shows(self):
        write_ledger(self.d, [ev(1.0, 1)])
        rg.start_session_cursor(self.d, "s", "startup")
        append(self.d, ev(2.0, 2))  # shed before the compaction, not yet shown
        rg.start_session_cursor(self.d, "s", "compact")
        self.assertEqual([e["cmd"] for e in rg.unseen_events(self.d, "s")], ["cargo build 2"])

    def test_resume_and_clear_leave_the_cursor(self):
        for src in ("resume", "clear"):
            write_ledger(self.d, [ev(1.0, 1)])
            rg.start_session_cursor(self.d, src, "startup")
            append(self.d, ev(2.0, 2))
            rg.start_session_cursor(self.d, src, src)
            self.assertEqual(len(rg.unseen_events(self.d, src)), 1, src)

    def test_startup_resets_the_cursor_to_the_end(self):
        write_ledger(self.d, [ev(1.0, 1)])
        rg.start_session_cursor(self.d, "s", "startup")
        append(self.d, ev(2.0, 2))
        rg.start_session_cursor(self.d, "s", "startup")
        self.assertEqual(rg.unseen_events(self.d, "s"), [])

    def test_no_source_sets_only_when_no_cursor_exists(self):
        write_ledger(self.d, [ev(1.0, 1)])
        rg.start_session_cursor(self.d, "s")
        append(self.d, ev(2.0, 2))
        rg.start_session_cursor(self.d, "s")  # cursor exists: untouched
        self.assertEqual(len(rg.unseen_events(self.d, "s")), 1)


class DateFormat(unittest.TestCase):
    def test_shed_line_carries_iso_date_and_time(self):
        ts = time.mktime((2026, 11, 22, 9, 8, 7, 0, 0, -1))
        self.assertIn("2026-11-22 09:08:07 shed", rg._fmt_event(ev(ts)))


class FakeClock:
    def __init__(self):
        self.t = 100.0

    def __call__(self):
        return self.t


class Deadline(unittest.TestCase):
    def runner(self, clock, costs, seen):
        """A fake subprocess.run: records the timeout it was handed and advances the clock."""
        it = iter(costs)

        def run(argv, capture_output=True, text=True, timeout=None):
            seen.append((os.path.basename(argv[1]), argv[2] if len(argv) > 2 else "", timeout))
            cost = next(it)
            if cost is None or cost >= timeout:
                clock.t += timeout
                raise subprocess.TimeoutExpired(argv, timeout)
            clock.t += cost
            return subprocess.CompletedProcess(argv, 0, stdout="ok\n", stderr="")
        return run

    def test_budget_arithmetic(self):
        c = FakeClock()
        b = rg.Budget(8.0, clock=c)
        self.assertEqual(b.step_timeout(), 6)
        c.t += 5
        self.assertAlmostEqual(b.step_timeout(), 3.0)
        c.t += 2.9
        self.assertIsNone(b.step_timeout())  # 0.1 left < MIN_STEP_S
        c.t += 10
        self.assertEqual(b.remaining(), 0.0)

    def test_each_step_gets_what_remains(self):
        c = FakeClock()
        seen = []
        b = rg.Budget(8.0, clock=c)
        out = rg.io_guard_and_berth_lines("s", b, run=self.runner(c, [4, 2, 1, 1], seen))
        self.assertEqual([round(t, 3) for _, _, t in seen], [6, 4.0, 2.0, 1.0])
        self.assertNotIn("skipped", out)

    def test_total_never_exceeds_deadline_and_skip_is_named(self):
        c = FakeClock()
        seen = []
        b = rg.Budget(8.0, clock=c)
        # io-guard start times out at 6s; berth moor gets the 2s that remain and times out too
        out = rg.io_guard_and_berth_lines("s", b, run=self.runner(c, [None, None], seen))
        self.assertLessEqual(c.t - 100.0, 8.0 + 1e-9)
        self.assertEqual([round(t, 3) for _, _, t in seen], [6, 2.0])
        self.assertIn("BERTH: unavailable (timed out after 2.0s)", out)
        self.assertIn("IO-GUARD: unavailable (timed out after 6.0s)", out)
        self.assertNotIn("TimeoutExpired", out)
        self.assertNotIn("['", out)  # no argv repr

    def test_skipped_line_when_deadline_spent(self):
        c = FakeClock()
        b = rg.Budget(8.0, clock=c)
        c.t += 7.95
        seen = []
        out = rg.io_guard_and_berth_lines("s", b, run=self.runner(c, [], seen))
        self.assertEqual(seen, [])
        self.assertIn("SESSION-START: skipped io-guard, berth moor/status", out)

    def test_status_skipped_keeps_moored_line_and_names_only_status(self):
        c = FakeClock()
        b = rg.Budget(8.0, clock=c)
        seen = []
        # 3 + 2.9 + 2.0 leaves 0.1s, under MIN_STEP_S: moor ran, status is skipped
        out = rg.io_guard_and_berth_lines("s", b, run=self.runner(c, [3, 2.9, 2.0], seen))
        self.assertIn("BERTH: moored as s", out)
        self.assertIn("berth status skipped", out)
        self.assertIn("berth inbox --session s", out)
        self.assertIn("SESSION-START: skipped berth status ", out)
        self.assertNotIn("io-guard,", out)
        self.assertIn(f"{rg.HOOK_TIMEOUT_S}s", out)

    def test_ensure_daemon_respects_budget(self):
        c = FakeClock()
        b = rg.Budget(8.0, clock=c)
        c.t += 8
        self.assertIn("skipped", rg.ensure_daemon(b, run=self.runner(c, [], [])))
        c2 = FakeClock()
        got = rg.ensure_daemon(rg.Budget(8.0, clock=c2), run=self.runner(c2, [None], []))
        self.assertIn("timed out after 6.0s", got)

    def test_deadline_under_hook_timeout(self):
        self.assertLess(rg.SESSION_DEADLINE_S, rg.HOOK_TIMEOUT_S)


if __name__ == "__main__":
    unittest.main()
