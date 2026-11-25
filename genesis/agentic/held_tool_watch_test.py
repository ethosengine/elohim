"""held-tool-watch: a tool call pending past a bound is seen from outside its session.

Run (from the repository root): python3 -m unittest genesis/agentic/held_tool_watch_test.py
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


class NoStateDir(unittest.TestCase):
    def tearDown(self):
        self.assertFalse(Path(W["REPO_ROOT"], ".eprfs/status/held-tool-watch").exists())
        self.assertFalse(Path(W["main_checkout"](W["REPO_ROOT"]), ".eprfs/status/held-tool-watch").exists())


CID_NEW = "bafyreinewnewnewnewnewnewnewnewnewnewnewnewnewnewnewnewnew"
CID_OLD = "bafyreioldoldoldoldoldoldoldoldoldoldoldoldoldoldoldoldold"
HELP_NEW = ("epr session stasis fold ...\nepr session stasis observe --session ID --source watcher:NAME "
            "--tool NAME [--pending-since RFC3339] [--agent ID] [--journal-root DIR]\n")
HELP_OLD = "epr session stasis fold ...\nepr session stasis status ...\n"


class ObserveTest(Fixture, NoStateDir):
    """`observe`: one pass through the TARGET session's own pinned epr, faked as executables in a
    temporary bin-store; the real journals and the real store are never touched."""

    def setUp(self):
        super().setUp()
        self.store = self.root / "bin-store"
        self.log = self.root / "calls.jsonl"
        for cid, help_text in ((CID_NEW, HELP_NEW), (CID_OLD, HELP_OLD)):
            self.fake_epr(cid, help_text)
        self.sessions = self.root / "s.json"
        self.sessions.write_text(json.dumps({"moorings": [{"session": SESSION}]}))
        self.lines = []

    def fake_epr(self, cid, help_text):
        """An executable that logs its argv and answers `--help` with help_text; an old one (no
        `stasis observe` in its help) refuses `observe` the way the real pre-observe epr does."""
        binary = self.store / "epr" / cid / "epr"
        binary.parent.mkdir(parents=True)
        knows = "stasis observe" in help_text
        binary.write_text(f"""#!/usr/bin/env python3
import json, sys
with open({str(self.log)!r}, "a") as f:
    f.write(json.dumps({{"cid": {cid!r}, "argv": sys.argv[1:]}}) + "\\n")
if "--help" in sys.argv:
    sys.stdout.write({help_text!r}); sys.exit(0)
if sys.argv[1:4] == ["session", "stasis", "observe"]:
    if not {knows!r}:
        sys.stderr.write("epr session stasis: unexpected stasis argument --source\\n"); sys.exit(1)
    print(json.dumps({{"recorded": True}})); sys.exit(0)
sys.exit(9)
""")
        binary.chmod(0o755)

    def pin(self, session=SESSION, cid=CID_NEW, raw=None):
        sessions = self.store / "epr" / "sessions"
        sessions.mkdir(parents=True, exist_ok=True)
        (sessions / session).write_text(raw if raw is not None else json.dumps(
            {"cid": cid, "session": session, "pinnedAt": "2026-11-25T00:00:00Z", "provisional": False}))

    def calls(self):
        if not self.log.exists():
            return []
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def observes(self):
        return [c for c in self.calls() if c["argv"][:3] == ["session", "stasis", "observe"]]

    def run_main(self, now, extra=()):
        args = ["observe", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--bin-store", str(self.store), "--now", str(now),
                *extra]
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = W["main"](args)
        return code, out.getvalue().splitlines(), err.getvalue().splitlines()

    def test_pending_past_the_threshold_makes_exactly_one_observe_with_the_right_argv(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))
        code, out, err = self.run_main(T0 + 200)
        self.assertEqual((code, out, err), (0, [], []))
        observes = self.observes()
        self.assertEqual(len(observes), 1)
        self.assertEqual(observes[0]["cid"], CID_NEW)
        self.assertEqual(observes[0]["argv"], [
            "session", "stasis", "observe", "--session", SESSION, "--source", "watcher:held-tool",
            "--tool", "Bash", "--pending-since", W["rfc3339"](T0), "--journal-root", str(self.stasis)])

    def test_a_repeat_run_names_the_same_pending_since_so_the_native_side_folds_one_incident(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))
        self.run_main(T0 + 200)
        self.run_main(T0 + 900)
        since = {tuple(c["argv"][c["argv"].index("--pending-since") + 1:][:1]) for c in self.observes()}
        self.assertEqual(since, {(W["rfc3339"](T0),)})

    def test_a_subagent_call_carries_its_agent_id(self):
        self.pin()
        sub = self.transcripts / "proj" / SESSION / "subagents"
        sub.mkdir(parents=True)
        self.write(use("m", "Bash", T0, command="ls"), result("m", T0 + 1))
        self.write(use("s1", "Read", T0 + 10, file_path="/x"), path=sub / "agent-a1b2.jsonl")
        self.run_main(T0 + 500)
        [only] = self.observes()
        self.assertEqual(only["argv"][only["argv"].index("--agent") + 1], "a1b2")
        self.assertEqual(only["argv"][only["argv"].index("--tool") + 1], "Read")

    def test_pending_under_the_threshold_makes_no_call(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))
        self.assertEqual(self.run_main(T0 + 60), (0, [], []))
        self.assertEqual(self.calls(), [])

    def test_a_resolved_call_makes_no_call(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"), result("t1", T0 + 1))
        self.assertEqual(self.run_main(T0 + 7000), (0, [], []))
        self.assertEqual(self.calls(), [])

    def test_a_session_pinned_to_an_epr_without_the_verb_is_not_written_and_one_line_says_so(self):
        self.pin(cid=CID_OLD)
        self.write(use("t1", "Bash", T0, command="ls"))
        code, out, err = self.run_main(T0 + 200)
        self.assertEqual(code, 0)
        self.assertEqual(out, [f"held-tool-watch: session {SESSION} runs an epr without stasis observe; "
                               "not observed"])
        self.assertEqual(err, [])
        self.assertEqual(self.observes(), [])
        self.assertEqual([c["argv"] for c in self.calls()], [["session", "stasis", "--help"]])
        self.assertFalse(self.stasis.exists())

    def test_the_writer_is_the_target_sessions_pin_not_the_newest_epr(self):
        other = "0123abcd-aaaa-bbbb-cccc-000000000002"
        self.sessions.write_text(json.dumps({"moorings": [{"session": SESSION}, {"session": other}]}))
        self.pin(cid=CID_OLD)
        self.pin(session=other, cid=CID_NEW)
        self.write(use("t1", "Bash", T0, command="ls"))
        self.write(use("t2", "Bash", T0, command="ls"), path=self.transcripts / "proj" / f"{other}.jsonl")
        code, out, _ = self.run_main(T0 + 200)
        self.assertEqual(code, 0)
        self.assertEqual([(c["cid"], c["argv"][4]) for c in self.observes()], [(CID_NEW, other)])
        self.assertEqual(len(out), 1)
        self.assertIn(SESSION, out[0])

    def test_an_undeterminable_pin_is_not_written_and_one_line_says_so(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        cases = {
            "no pin file": None,
            "pin names another session": json.dumps({"cid": CID_NEW, "session": "someone-else"}),
            "pin has no valid cid": json.dumps({"cid": "../../bin/sh", "session": SESSION}),
            "unreadable pin": "{not json",
            "absent": json.dumps({"cid": "bafyreiabsentabsentabsentabsentabsent", "session": SESSION}),
        }
        for why, raw in cases.items():
            with self.subTest(why):
                pin = self.store / "epr" / "sessions" / SESSION
                if raw is None:
                    pin.unlink(missing_ok=True)
                else:
                    self.pin(raw=raw)
                code, out, err = self.run_main(T0 + 200)
                self.assertEqual((code, err), (0, []))
                self.assertEqual(len(out), 1)
                self.assertTrue(out[0].startswith(f"held-tool-watch: session {SESSION} has no determinable "
                                                  "epr pin ("), out[0])
                self.assertIn(why, out[0])
                self.assertTrue(out[0].endswith("; not observed"))
        self.assertEqual(self.calls(), [])

    def test_no_bin_store_at_all_writes_nothing(self):
        self.write(use("t1", "Bash", T0, command="ls"))
        resolve = []

        def runner(argv, timeout=60):
            resolve.append(argv)
            return 1, "", "epr: no bin-store"
        args = ["observe", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--now", str(T0 + 200)]
        with contextlib.redirect_stdout(io.StringIO()) as out:
            self.assertEqual(W["main"](args, runner=runner), 0)
        self.assertEqual(resolve, [["epr", "bin", "resolve", "--tool", "epr", "--json"]])
        self.assertIn("(no bin-store); not observed", out.getvalue())

    def test_the_bin_store_comes_from_the_native_resolve_path(self):
        path = str(self.store / "epr" / CID_NEW / "epr")
        root = W["native_bin_store"](lambda argv, timeout=60: (0, json.dumps({"path": path}), ""))
        self.assertEqual(root, str(self.store))

    def test_a_malformed_transcript_exits_non_zero_with_one_line_and_no_call(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"), "this is not json", use("t2", "Bash", T0, command="ls"))
        code, out, err = self.run_main(T0 + 200)
        self.assertEqual(code, 1)
        self.assertEqual(out, [])
        self.assertEqual(len(err), 1)
        self.assertTrue(err[0].startswith("held-tool-watch: ValueError: malformed transcript"), err[0])
        self.assertEqual(self.calls(), [])

    def test_a_failing_observe_is_one_stderr_line_and_exit_1_never_2(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))

        def runner(argv, timeout=60):
            if "--help" in argv:
                return 0, HELP_NEW, ""
            return 101, "", "thread main panicked\nat journal.rs"
        code = W["observe_pass"](W["scan"]([SESSION], str(self.transcripts), 120, T0 + 200), str(self.stasis),
                                 str(self.store), runner=runner, say=self.fail, warn=self.lines.append)
        self.assertEqual(code, 1)
        self.assertEqual(self.lines, [f"held-tool-watch: observe for session {SESSION} failed (exit 101): "
                                      "thread main panicked at journal.rs"])

    def test_a_session_without_a_journal_is_one_line_not_a_failure(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))

        def runner(argv, timeout=60):
            if "--help" in argv:
                return 0, HELP_NEW, ""
            return 2, "", f"epr session stasis: no journal for session {SESSION}\n"
        said = []
        failed = W["observe_pass"](W["scan"]([SESSION], str(self.transcripts), 120, T0 + 200), str(self.stasis),
                                   str(self.store), runner=runner, say=said.append, warn=self.fail)
        self.assertEqual((failed, said), (0, [W["NO_JOURNAL_LINE"].format(session=SESSION)]))

    def test_any_internal_error_is_one_line_and_exit_1(self):
        for argv in (["bogus"], ["observe", "--sessions-json", str(self.root / "missing.json")]):
            with self.subTest(argv=argv), contextlib.redirect_stderr(io.StringIO()) as err:
                self.assertEqual(W["main"](argv), 1)
                self.assertEqual(len(err.getvalue().strip().splitlines()), 1)



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
