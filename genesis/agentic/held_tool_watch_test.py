"""held-tool-watch: a tool call pending past a bound is seen from outside its session.

Run: python3 -m unittest genesis/agentic/held_tool_watch_test.py
"""
import contextlib
import io
import json
import os
import runpy
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
os.environ["BERTH_UNDER_TEST"] = "1"
W = runpy.run_path(str(HERE / "bin/held-tool-watch"))
BERTH = runpy.run_path(str(HERE / "bin/berth"))
Berth = BERTH["Berth"]

SESSION = "0123abcd-aaaa-bbbb-cccc-000000000001"
T0 = 1_800_000_000.0


def iso(t):
    import datetime
    return datetime.datetime.fromtimestamp(t, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S.000Z")


def use(tool_id, name, at, **inputs):
    return json.dumps({"type": "assistant", "timestamp": iso(at), "message": {"content": [
        {"type": "tool_use", "id": tool_id, "name": name, "input": inputs}]}})


def result(tool_id, at):
    return json.dumps({"type": "user", "timestamp": iso(at), "message": {"content": [
        {"type": "tool_result", "tool_use_id": tool_id, "content": "ok"}]}})


class Fixture(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name)
        self.transcripts = self.root / "projects"
        (self.transcripts / "proj").mkdir(parents=True)
        self.main = self.transcripts / "proj" / f"{SESSION}.jsonl"
        self.stasis = self.root / "stasis"
        self.state = self.root / "state"

    def write(self, *lines, path=None, tail=""):
        (path or self.main).write_text("\n".join(lines) + "\n" + tail)

    def scan(self, now, bound=120):
        return W["scan"]([SESSION], str(self.transcripts), bound, now, str(self.stasis))


class ScanTest(Fixture):
    def test_dangling_bash_past_bound_reported_and_under_bound_not(self):
        self.write(use("t1", "Bash", T0, command="git rebase --exec x"))
        self.assertEqual(self.scan(T0 + 100), [])
        held = self.scan(T0 + 7000)
        self.assertEqual([(h["tool"], h["agent"], h["pendingS"]) for h in held], [("Bash", "main", 7000)])

    def test_matched_call_not_reported(self):
        self.write(use("t1", "Bash", T0, command="ls"), result("t1", T0 + 1))
        self.assertEqual(self.scan(T0 + 7000), [])

    def test_long_running_tools_not_reported(self):
        self.write(use("a", "Bash", T0, command="sleep 900", run_in_background=True),
                   use("b", "Agent", T0, prompt="x"), use("c", "Monitor", T0, command="tail -f x"),
                   use("d", "Bash", T0, command="make", timeout=600000))
        self.assertEqual(self.scan(T0 + 300), [])  # d is within its own 600 s timeout
        self.assertEqual([h["toolUseId"] for h in self.scan(T0 + 700)], ["d"])  # past timeout + grace

    def test_subagent_dangling_call_reported_under_its_agent_id(self):
        sub = self.transcripts / "proj" / SESSION / "subagents"
        sub.mkdir(parents=True)
        self.write(use("m", "Bash", T0, command="ls"), result("m", T0 + 1))
        self.write(use("s1", "Bash", T0, command="rm x"), path=sub / "agent-a1b2.jsonl")
        held = self.scan(T0 + 500)
        self.assertEqual([(h["agent"], h["toolUseId"]) for h in held], [("a1b2", "s1")])

    def test_partial_last_line_does_not_crash(self):
        self.write(use("t1", "Bash", T0, command="ls"), tail='{"type": "assistant", "timestamp": "20')
        self.assertEqual(len(self.scan(T0 + 500)), 1)

    def test_cause_is_approval_evidence_only_with_a_journal_entry(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        self.assertEqual(self.scan(T0 + 500)[0]["cause"], "unknown")
        journal = self.stasis / SESSION
        journal.mkdir(parents=True)
        (journal / "events.jsonl").write_text(json.dumps(
            {"kind": "permission-requested", "source": {"precedingToolUseId": "other"}}) + "\n")
        self.assertEqual(self.scan(T0 + 500)[0]["cause"], "unknown")
        (journal / "events.jsonl").write_text(json.dumps(
            {"kind": "permission-requested", "source": {"precedingToolUseId": "t1"}}) + "\n")
        self.assertEqual(self.scan(T0 + 500)[0]["cause"], "approval-evidence")

    def test_redaction_masks_token_like_strings(self):
        shape = W["redact"]("curl -H 'Authorization: Bearer abc123' https://x.io/a?key=SECRET "
                            "ghp_abcdefghijklmnop1234 PASSWORD=hunter2")
        for leaked in ("abc123", "SECRET", "ghp_abcdefghij", "hunter2"):
            self.assertNotIn(leaked, shape)
        self.assertLessEqual(len(W["redact"]("x" * 500)), 80)

    def test_cli_exit_codes(self):
        sessions = self.root / "s.json"
        sessions.write_text(json.dumps({"moorings": [{"session": SESSION}]}))
        self.write(use("t1", "Bash", T0, command="ls"))
        args = ["scan", "--transcripts", str(self.transcripts), "--sessions-json", str(sessions),
                "--stasis-root", str(self.stasis)]
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(W["main"](args + ["--now", str(T0 + 500)]), 3)
            self.assertEqual(W["main"](args + ["--now", str(T0 + 5)]), 0)
        self.assertIn("0123abcd  main  Bash  500s  cause: unknown", out.getvalue())


class WatchTest(Fixture):
    def setUp(self):
        super().setUp()
        self.said, self.logs = [], []

    def run_pass(self, now, push=None):
        return W["run_pass"](str(self.state), [SESSION], str(self.transcripts), 120, 5, now,
                             str(self.stasis), say=self.said.append,
                             push=push or (lambda t, b: (_ for _ in ()).throw(LookupError("push: no transport configured"))),
                             log=self.logs.append)

    def test_announces_once_across_passes_and_once_on_resolution(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        self.run_pass(T0 + 200)
        self.run_pass(T0 + 205)
        self.assertEqual(len(self.said), 1)
        self.assertTrue(self.said[0].startswith("HELD: session 0123abcd Bash pending 200s (cause unknown)"))
        self.assertTrue((self.state / "held" / f"{SESSION}.json").exists())
        self.write(use("t1", "Bash", T0, command="ls"), result("t1", T0 + 300))
        self.run_pass(T0 + 310)
        self.run_pass(T0 + 315)
        self.assertEqual(len(self.said), 2)
        self.assertEqual(self.said[1], "HELD-CLEARED: session 0123abcd Bash returned after 310s")
        self.assertFalse((self.state / "held" / f"{SESSION}.json").exists())

    def test_missing_push_transport_logs_and_does_not_fail(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        self.run_pass(T0 + 200)
        self.assertIn("push: no transport configured", self.logs)
        self.assertEqual(len(self.said), 1)

    def test_one_failing_channel_does_not_stop_the_others(self):
        pushed = []
        self.write(use("t1", "Bash", T0, command="ls"))

        def broken(_):
            raise OSError("exchange down")
        W["run_pass"](str(self.state), [SESSION], str(self.transcripts), 120, 5, T0 + 200,
                      str(self.stasis), say=broken, push=lambda t, b: pushed.append(b), log=self.logs.append)
        self.assertEqual(len(pushed), 1)
        self.assertTrue(any(line.startswith("berth: failed") for line in self.logs))


class BerthStatusTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        patch = mock.patch.dict(os.environ, {"HELD_TOOL_WATCH_DIR": str(self.dir / "w")})
        patch.start()
        self.addCleanup(patch.stop)
        self.now = 1000
        self.b = Berth(str(self.dir / "berth"), now=lambda: self.now, policy_path="/nonexistent/pool-policy.json")
        guard = mock.patch.dict(Berth.moor.__globals__, {"envelope_guard": lambda *a: contextlib.nullcontext(1)})
        guard.start()
        self.addCleanup(guard.stop)
        self.b.moor("live", model="m", lab="l", runtime="r")
        self.render = BERTH["render_status"]

    def marker(self, refreshed):
        path = self.dir / "w" / "held" / "live.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(dict(session="live", agent="main", tool="Bash", since=self.now - 700,
                                        refreshedAt=refreshed, intervalS=5)))

    def status(self):
        with mock.patch.dict(self.render.__globals__, {"_guard_state": lambda _: None,
                                                       "_steward_view": lambda b: (None, None)}):
            return self.render(self.b)

    def test_held_replaces_live_and_leaves_liveness_alone(self):
        self.assertIn("  live  live", self.status())
        self.marker(self.now - 3)
        text = self.status()
        self.assertIn("  held  live", text)
        self.assertIn("    held: Bash pending 700s since ", text)
        self.assertTrue(self.b.is_live(self.b.mooring("live")))
        row = BERTH["status_json"](self.b, no_peer=True)["moorings"][0]
        self.assertEqual((row["held"]["tool"], row["held"]["pendingS"], row["held"]["watcherSilent"]),
                         ("Bash", 700, False))

    def test_stale_marker_renders_held_question_mark(self):
        self.marker(self.now - 11)  # older than 2 x the 5 s interval
        text = self.status()
        self.assertIn("  held? live", text)
        self.assertIn("held? (watcher silent): Bash pending 700s", text)
        self.assertTrue(BERTH["status_json"](self.b, no_peer=True)["moorings"][0]["held"]["watcherSilent"])


if __name__ == "__main__":
    unittest.main()
