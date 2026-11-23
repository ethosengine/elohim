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


def status_json(*incidents):
    """What `epr session stasis status --json` prints: incidents are (session, id) pairs."""
    sessions = {}
    for session, incident in incidents:
        sessions.setdefault(session, []).append(
            {"id": incident, "stallClass": "silence", "state": "open", "ageSeconds": 500})
    return json.dumps({"sessions": [{"session": k, "openIncidents": v} for k, v in sessions.items()]})


class FakeEpr:
    """Fake runner for epr: records argv; `observe` and `status` answer as configured."""
    def __init__(self, status="{\"sessions\": []}", observe=(0, "", "")):
        self.calls, self.status, self.observe = [], status, observe

    def __call__(self, argv):
        self.calls.append(argv)
        return self.observe if argv[2] == "observe" else (0, self.status, "")


class NoStateDir(unittest.TestCase):
    def tearDown(self):
        self.assertFalse(Path(W["REPO_ROOT"], ".eprfs/status/held-tool-watch").exists())
        self.assertFalse(Path(W["main_checkout"](W["REPO_ROOT"]), ".eprfs/status/held-tool-watch").exists())


class WatchTest(Fixture, NoStateDir):
    def setUp(self):
        super().setUp()
        self.said, self.logs, self.announced = [], [], set()

    def run_pass(self, epr, now, push=None):
        return W["run_pass"]([SESSION], str(self.transcripts), 120, now, self.announced, str(self.stasis),
                             runner=epr, say=self.said.append,
                             push=push or (lambda t, b: (_ for _ in ()).throw(LookupError("push: no transport configured"))),
                             log=self.logs.append)

    def test_sensed_call_invokes_observe_once_per_pass_with_the_right_argv(self):
        sub = self.transcripts / "proj" / SESSION / "subagents"
        sub.mkdir(parents=True)
        self.write(use("t1", "Bash", T0, command="ls"))
        self.write(use("s1", "Bash", T0 + 10, command="rm x"), path=sub / "agent-a1b2.jsonl")
        epr = FakeEpr()
        self.run_pass(epr, T0 + 200)
        observes = [c for c in epr.calls if c[2] == "observe"]
        self.assertEqual(observes[0], ["session", "stasis", "observe", "--session", SESSION, "--source",
                                       "watcher:held-tool-watch", "--tool", "Bash", "--pending-since",
                                       W["rfc3339"](T0)])
        self.assertEqual(observes[1][-2:], ["--agent", "a1b2"])
        self.assertEqual(len(observes), 2)
        self.assertEqual(len([c for c in epr.calls if c[2] == "status"]), 1)

    def test_a_call_under_the_sensing_threshold_calls_nothing(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        epr = FakeEpr()
        self.run_pass(epr, T0 + 60)
        self.assertEqual(epr.calls, [])

    def test_unavailable_verb_prints_one_line_records_nothing_and_exits_4(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        epr = FakeEpr(observe=(1, "", "epr session stasis: unknown command observe"))
        held, unavailable = self.run_pass(epr, T0 + 200)
        self.assertTrue(unavailable)
        self.assertEqual(self.logs, [W["UNAVAILABLE_LINE"]])
        self.assertEqual(self.said, [])
        self.assertEqual([c[2] for c in epr.calls], ["observe"])
        sessions = self.root / "s.json"
        sessions.write_text(json.dumps({"moorings": [{"session": SESSION}]}))
        args = ["watch", "--transcripts", str(self.transcripts), "--sessions-json", str(sessions),
                "--stasis-root", str(self.stasis), "--max-passes", "1"]
        clock = mock.Mock(time=lambda: T0 + 200, sleep=lambda s: None)
        with mock.patch.dict(W["cmd_watch"].__globals__, {"time": clock}), \
                contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(W["cmd_watch"](W["parser"]().parse_args(args), runner=epr), 4)
        self.assertEqual(out.getvalue().strip().splitlines(), [W["UNAVAILABLE_LINE"]])
        self.assertEqual(sorted(p.name for p in self.root.iterdir()), ["projects", "s.json"])

    def test_open_incident_announces_once_across_two_passes(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        epr = FakeEpr(status=status_json((SESSION, "inc-1")))
        self.run_pass(epr, T0 + 200)
        self.run_pass(epr, T0 + 205)
        self.assertEqual(len(self.said), 1)
        self.assertIn("0123abcd", self.said[0])
        self.assertIn("inc-1", self.said[0])
        self.assertIn("push: no transport configured", self.logs)

    def test_closed_incident_and_the_observers_own_session_are_not_announced(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        self.run_pass(FakeEpr(status=status_json()), T0 + 200)
        self.run_pass(FakeEpr(status=status_json(("held-tool-watch", "inc-w"))), T0 + 205)
        self.assertEqual(self.said, [])

    def test_one_failing_channel_does_not_stop_the_others(self):
        pushed = []
        self.write(use("t1", "Bash", T0, command="ls"))

        def broken(_):
            raise OSError("exchange down")
        W["run_pass"]([SESSION], str(self.transcripts), 120, T0 + 200, set(), str(self.stasis),
                      runner=FakeEpr(status=status_json((SESSION, "i"))), say=broken,
                      push=lambda t, b: pushed.append(b), log=self.logs.append)
        self.assertEqual(len(pushed), 1)
        self.assertTrue(any(line.startswith("berth: failed") for line in self.logs))


class BerthStatusTest(NoStateDir):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.now = 1000
        self.b = Berth(str(self.dir / "berth"), now=lambda: self.now, policy_path="/nonexistent/pool-policy.json")
        guard = mock.patch.dict(Berth.moor.__globals__, {"envelope_guard": lambda *a: contextlib.nullcontext(1)})
        guard.start()
        self.addCleanup(guard.stop)
        self.b.moor("live", model="m", lab="l", runtime="r")
        self.render = BERTH["render_status"]

    def status(self, runner):
        with mock.patch.dict(self.render.__globals__, {"_guard_state": lambda _: None,
                                                       "_steward_view": lambda b: (None, None),
                                                       "_stasis_status_runner": lambda t: runner(t)}):
            return self.render(self.b)

    def test_held_renders_from_the_native_status_and_leaves_liveness_alone(self):
        calls = []
        text = self.status(lambda t: calls.append(t) or status_json(("live", "inc-9")))
        self.assertEqual(len(calls), 1)
        self.assertIn("  held  live", text)
        self.assertIn("    held: native stasis incident inc-9 open 500s", text)
        self.assertTrue(self.b.is_live(self.b.mooring("live")))
        with mock.patch.dict(BERTH["status_json"].__globals__, {"_stasis_status_runner": lambda t: status_json(("live", "inc-9"))}):
            self.b._read_peer_leases = mock.Mock(return_value=({"others": {}}, None))
            self.b._read_peer_status = mock.Mock(return_value=({"devices": []}, None))
            row = BERTH["status_json"](self.b)["moorings"][0]
        self.assertEqual((row["held"]["incident"], row["held"]["pendingS"]), ("inc-9", 500))

    def test_unavailable_or_failing_read_renders_plain_live(self):
        def boom(_):
            raise OSError("no epr")
        for runner in (boom, lambda t: "not json", lambda t: status_json()):
            text = self.status(runner)
            self.assertIn("  live  live", text)
            self.assertNotIn("held", text)


if __name__ == "__main__":
    unittest.main()
