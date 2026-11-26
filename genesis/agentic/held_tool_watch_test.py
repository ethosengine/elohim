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
from types import SimpleNamespace
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

    def test_a_torn_last_record_is_unknown_this_pass_not_pending(self):
        # The torn record may be the very result that resolves t1.
        self.write(use("t1", "Bash", T0, command="ls"), tail='{"type": "user", "message": {"content": [{"ty')
        self.assertEqual(self.scan(T0 + 500), [])
        self.write(use("t1", "Bash", T0, command="ls"))
        self.assertEqual(len(self.scan(T0 + 500)), 1)

    def test_resumption_is_never_read_from_a_tail_that_does_not_reach_the_call(self):
        since = W["rfc3339"](T0)
        self.write(use("held", "Bash", T0, command="ls"),
                   *[use(f"f{i}", "Read", T0 + 1 + i, file_path="/x" * 200) for i in range(600)],
                   *[result(f"f{i}", T0 + 1000 + i) for i in range(600)])
        # Reaching back, the tail grows for this file until it holds the call: still pending.
        self.assertIs(W["still_pending"](str(self.transcripts), SESSION, "main", "Bash", since), True)
        # Capped short of it: unknown, never "resumed".
        self.assertIsNone(W["still_pending"](str(self.transcripts), SESSION, "main", "Bash", since,
                                             tail_bytes=W["FIRST_TAIL_BYTES"]))
        self.write(use("held", "Bash", T0, command="ls"), result("held", T0 + 50))
        self.assertIs(W["still_pending"](str(self.transcripts), SESSION, "main", "Bash", since), False)

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

    def test_a_long_transcript_costs_a_small_tail_and_an_old_one_nothing(self):
        filler = [use(f"f{i}", "Read", T0 - 5000 + i, file_path="/x" * 200) for i in range(3000)]
        filler += [result(f"f{i}", T0 - 4000 + i) for i in range(3000)]
        self.write(*filler, use("held", "Bash", T0, command="ls"))
        size = self.main.stat().st_size
        self.assertGreater(size, 2 * 1024 * 1024)
        budget = W["Budget"]()
        held = W["scan"]([SESSION], str(self.transcripts), 120, T0 + 500, str(self.stasis), budget=budget)
        self.assertEqual([h["toolUseId"] for h in held], ["held"])
        self.assertEqual(budget.read, W["FIRST_TAIL_BYTES"])
        old = T0 - 3 * 86400
        os.utime(self.main, (old, old))
        budget = W["Budget"]()
        with mock.patch.dict(W["scan"].__globals__["time"].__dict__, {"time": lambda: T0}):
            self.assertEqual(W["scan"]([SESSION], str(self.transcripts), 120, T0 + 500, budget=budget), [])
        self.assertEqual(budget.read, 0)

    def test_one_huge_last_record_grows_the_tail_for_that_file_only(self):
        self.write(use("held", "Bash", T0, command="x" * (600 * 1024)))
        budget = W["Budget"]()
        held = W["scan"]([SESSION], str(self.transcripts), 120, T0 + 500, budget=budget)
        self.assertEqual([h["toolUseId"] for h in held], ["held"])
        self.assertLess(budget.read, 4 * self.main.stat().st_size)

    def test_the_pass_budget_stops_reading_with_one_line(self):
        other = "0123abcd-aaaa-bbbb-cccc-000000000002"
        self.write(use("t1", "Bash", T0, command="ls"))
        self.write(use("t2", "Bash", T0, command="ls"), path=self.transcripts / "proj" / f"{other}.jsonl")
        lines = []
        budget = W["Budget"](self.main.stat().st_size)
        held = W["scan"]([SESSION, other], str(self.transcripts), 120, T0 + 500, budget=budget,
                         warn=lines.append)
        self.assertEqual(len(held), 1)
        self.assertEqual(len(lines), 1)
        self.assertIn("read budget", lines[0])

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


class ObserveFixture(Fixture, NoStateDir):
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
if sys.argv[1:4] == ["session", "stasis", "status"]:
    session = sys.argv[sys.argv.index("--session") + 1]
    try:
        with open({str(self.root / "open-incidents.json")!r}) as f:
            ids = json.load(f).get(session, [])
    except FileNotFoundError:
        ids = []
    print(json.dumps({{"sessions": [{{"session": session, "openIncidents": [
        {{"id": i, "stallClass": "silence", "state": "open", "ageSeconds": 1}} for i in ids]}}]}}))
    sys.exit(0)
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


class ObserveTest(ObserveFixture):
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


class CarryFixture(Fixture, NoStateDir):
    """`carry` with fake epr/berth/git/node/just calls; no real repository effect is possible."""

    def setUp(self):
        super().setUp()
        self.repo = self.root / "repo"
        (self.repo / ".worktrees").mkdir(parents=True)
        self.pool = self.root / "cargo-pool"
        (self.pool / "family").mkdir(parents=True)
        self.store = self.root / "bin-store"
        self.head = self.store / "epr" / CID_NEW / "epr"
        self.head.parent.mkdir(parents=True)
        self.head.write_text("#!/bin/sh\nexit 99\n")
        self.head.chmod(0o755)
        (self.store / "epr" / "current.json").write_text(json.dumps({"cid": CID_NEW}))
        self.commit = "a" * 40
        self.closure = "b" * 64
        self.now = 1_900_000_000
        self.calls = []
        self.room = {"verdict": "admit", "reason": "two seats free"}
        self.holder = None
        self.fetch = 0
        self.computed = self.closure
        self.gate_code = 0
        self.gate_timeout = False
        self.gate_stop = None
        self.gate_settled = True
        self.gate_verdict = True
        self.capacity = {"ram": "ok", "room": "admit", "pool": "ok"}
        self.release_failures = 0
        self.claim_replies = []
        self.make_slot = False
        self.commitment = "bafyreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        self.misses = [self.miss()]
        Path(self.root, W["CARRY_ON_FILE"]).write_text(W["rfc3339"](self.now + 7 * 86400) + "\n")
        self.pool_env = mock.patch.dict(os.environ, {"CARGO_TARGET_POOL_ROOT": str(self.pool)})
        self.pool_env.start()
        self.addCleanup(self.pool_env.stop)

    def miss(self, **changes):
        row = {"intent": "bafyreiintentintentintentintentintentintentintentintentintent",
               "event": "bafyreieventeventeventeventeventeventeventeventeventevent",
               "announcer": "did:key:z6MkSIBLING", "change": "change", "commit": self.commit,
               "gitRef": "refs/heads/self/integration", "project": "eprfs", "closure": self.closure,
               "p90S": 100, "waitUntil": self.now + 2000, "carriedBy": []}
        row.update(changes)
        return row

    def args(self, **changes):
        values = dict(repo_root=str(self.repo), stasis_root=str(self.stasis), bin_store=str(self.store),
                      now=self.now, carry_deadline_s=None, carry_seat_wait_s=0, parent_pid=None,
                      state="pass", until=None)
        values.update(changes)
        return SimpleNamespace(**values)

    def runner(self, argv, timeout=60, cwd=None, env=None):
        argv = [str(v) for v in argv]
        self.calls.append(dict(argv=argv, timeout=timeout, cwd=str(cwd) if cwd else None,
                               env=dict(env) if env is not None else None))
        if argv[:2] == [str(self.head), "peer"]:
            # As the real CLI answers (epr-cli peer.rs render_intents): under --json, --misses
            # prints only the misses list; without it, the whole output, device and misses among
            # its keys. A carry that asked with --misses would see no device and take nothing.
            if "--misses" in argv:
                return 0, json.dumps(self.misses), ""
            return 0, json.dumps({
                "advisory": [], "device": "did:key:z6MkLOCAL", "error": None, "intents": [],
                "maxSkewS": 120, "misses": self.misses, "now": int(self.now or 0),
                "settledMisses": [], "siblingsRead": 1, "siblingsWhy": None,
                "standing": "device", "store": None, "unjoined": [],
            }), ""
        if "claim" in argv and "cargo" in argv:
            return (0 if self.room.get("verdict") == "admit" else 3), json.dumps(self.room), ""
        if "room" in argv and "cargo" in argv:
            room = {"verdict": self.capacity["room"], "reason": "capacity fixture"}
            return 0, json.dumps(room), ""
        if "release" in argv and "cargo" in argv:
            return 0, "released", ""
        if "say" in argv:
            return 0, "", ""
        if "who" in argv and "dev-integration" in argv:
            return 0, json.dumps(self.holder), ""
        if argv[:4] == ["git", "-C", str(self.repo), "fetch"]:
            return self.fetch, "", "fetch refused" if self.fetch else ""
        if argv[0].endswith("ram-guard"):
            return (0, "RAM-GUARD  OK\n", "") if self.capacity["ram"] == "ok" else (3, "RAM-GUARD  HARD\n", "")
        if argv[0].endswith("cargo-pool"):
            return (0, "Volume: fixture (status=ok)\n", "") if self.capacity["pool"] == "ok" else (3, "Volume: fixture (status=hard)\n", "")
        if "cat-file" in argv:
            return 0, "", ""
        if "worktree" in argv and "add" in argv:
            Path(argv[-2]).mkdir(parents=True)
            return 0, "", ""
        if "status" in argv and argv[0] == "git":
            return 0, "", ""
        if argv[:3] == ["node", "--input-type=module", "--eval"]:
            return 0, json.dumps({"key": self.computed, "line": "miss"}), ""
        if argv[:3] == [str(self.head), "flow", "intend"]:
            if "--release-carry" in argv and self.release_failures:
                self.release_failures -= 1
                return 7, "", "release refused"
            if "--carry" in argv and self.claim_replies:
                return self.claim_replies.pop(0)
            return 0, json.dumps({"commitment": self.commitment}), ""
        if "worktree" in argv and "remove" in argv:
            import shutil
            shutil.rmtree(argv[-1], ignore_errors=True)
            return 0, "", ""
        raise AssertionError(f"unexpected call: {argv}")

    def gate(self, trusted, worktree, commit, project, env, deadline, stop_reason, runner=None):
        self.calls.append({"argv": ["just", "gate", project], "timeout": deadline,
                           "cwd": str(worktree), "env": dict(env)})
        if self.make_slot:
            slot = self.pool / "family" / "cd" / f"eprfs-wt-{Path(worktree).name}"
            slot.mkdir(parents=True)
        if self.gate_stop:
            self.holder = {"holder": "local-lander"} if self.gate_stop == "landing" else self.holder
            if self.gate_stop == "off":
                Path(self.root, W["CARRY_ON_FILE"]).unlink()
            if self.gate_stop == "expired":
                Path(self.root, W["CARRY_ON_FILE"]).write_text(W["rfc3339"](W["time"].time() - 1) + "\n")
            stopped = stop_reason()
        else:
            stopped = None
        run_log = self.root / "gate.run"
        events = [{"kind": "run-open"}]
        if self.gate_verdict and not stopped:
            events.append({"kind": "gate-verdict", "project": project})
        if self.gate_settled:
            events.append({"kind": "run-close", "outcome": "interrupted" if stopped else "passed"})
        run_log.write_text("".join(json.dumps(event) + "\n" for event in events))
        code = 17 if stopped else self.gate_code
        error = "stopped" if stopped else ("infra red" if self.gate_code else "")
        return code, "", error, stopped, str(run_log), self.gate_settled, self.gate_verdict and not stopped

    def run_carry(self, **changes):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = W["cmd_carry"](self.args(**changes), runner=self.runner, gate_runner=self.gate)
        return code, out.getvalue().splitlines()

    def claims(self):
        return [c for c in self.calls if "--carry" in c["argv"]]

    def releases(self):
        return [c for c in self.calls if "--release-carry" in c["argv"]]

    def gates(self):
        return [c for c in self.calls if c["argv"][:2] == ["just", "gate"]]


class CarryTest(CarryFixture):
    def test_default_off_reads_no_misses_and_starts_no_subprocess(self):
        Path(self.root, W["CARRY_ON_FILE"]).unlink()
        code, lines = self.run_carry()
        self.assertEqual((code, lines, self.calls), (0, [], []))

    def test_carry_on_defaults_to_seven_days_and_records_the_until_time(self):
        Path(self.root, W["CARRY_ON_FILE"]).unlink()
        with mock.patch.dict(os.environ, {"ELOHIM_SESSION_ID": SESSION}):
            code, lines = self.run_carry(state="on")
        self.assertEqual(code, 0)
        until = W["rfc3339"](self.now + 7 * 86400)
        self.assertEqual(lines, [f"carry on until {until}; last: no carry pass recorded"])
        self.assertTrue(Path(self.root, W["CARRY_ON_FILE"]).is_file())
        self.assertEqual(Path(self.root, W["CARRY_ON_FILE"]).read_text(), until + "\n")
        self.assertFalse((self.repo / W["CARRY_ON_FILE"]).exists())
        self.assertIn("/.eprfs/status/*", Path(W["REPO_ROOT"], ".gitignore").read_text())
        log = Path(self.root, W["WATCH_LOG"]).read_text()
        self.assertIn(f"carry=on until={until} action=opt-in by={SESSION}", log)
        self.assertEqual(self.calls, [])

    def test_until_accepts_a_duration_and_an_iso_8601_utc_date(self):
        Path(self.root, W["CARRY_ON_FILE"]).unlink()
        code, lines = self.run_carry(state="on", until="12h")
        duration_until = W["rfc3339"](self.now + 12 * 3600)
        self.assertEqual((code, lines),
                         (0, [f"carry on until {duration_until}; last: no carry pass recorded"]))
        absolute = "2032-04-05T06:07:08Z"
        code, lines = self.run_carry(state="on", until=absolute)
        self.assertEqual((code, lines),
                         (0, [f"carry on until {absolute}; last: no carry pass recorded"]))
        self.assertEqual(Path(self.root, W["CARRY_ON_FILE"]).read_text(), absolute + "\n")

    def test_status_shows_on_until_and_expired_at(self):
        until = W["rfc3339"](self.now + 3600)
        Path(self.root, W["CARRY_ON_FILE"]).write_text(until + "\n")
        Path(self.root, W["LAST_CARRY"]).write_text("most recent carry state\n")
        code, lines = self.run_carry(state="status")
        self.assertEqual((code, lines),
                         (0, [f"carry on until {until}; last: most recent carry state"]))
        expired = W["rfc3339"](self.now - 1)
        Path(self.root, W["CARRY_ON_FILE"]).write_text(expired + "\n")
        Path(self.root, W["CARRY_EXPIRY_SAID_FILE"]).unlink(missing_ok=True)
        code, lines = self.run_carry(state="status")
        self.assertEqual((code, lines), (0, [
            f"carry opt-in expired at {expired}; run held-tool-watch carry on to renew",
            f"carry expired at {expired}; last: most recent carry state",
        ]))

    def test_expiry_turns_several_passes_off_and_is_said_once(self):
        expired = W["rfc3339"](self.now - 1)
        Path(self.root, W["CARRY_ON_FILE"]).write_text(expired + "\n")
        all_lines = []
        for _ in range(4):
            code, lines = self.run_carry()
            self.assertEqual(code, 0)
            all_lines.extend(lines)
        self.assertEqual(all_lines, [
            f"carry opt-in expired at {expired}; run held-tool-watch carry on to renew"
        ])
        self.assertEqual(self.calls, [])
        self.assertEqual(Path(self.root, W["CARRY_EXPIRY_SAID_FILE"]).read_text(), expired + "\n")

    def test_renewal_replaces_the_until_time_and_is_logged_as_a_renewal(self):
        old_until = Path(self.root, W["CARRY_ON_FILE"]).read_text()
        code, lines = self.run_carry(state="on", until="3d")
        renewed = W["rfc3339"](self.now + 3 * 86400)
        self.assertEqual((code, lines),
                         (0, [f"carry on until {renewed}; last: no carry pass recorded"]))
        self.assertNotEqual(Path(self.root, W["CARRY_ON_FILE"]).read_text(), old_until)
        self.assertEqual(Path(self.root, W["CARRY_ON_FILE"]).read_text(), renewed + "\n")
        self.assertIn(f"carry=on until={renewed} action=renewal",
                      Path(self.root, W["WATCH_LOG"]).read_text())

    def test_a_legacy_switch_without_an_until_time_reads_expired(self):
        switch = Path(self.root, W["CARRY_ON_FILE"])
        switch.write_text("")
        os.utime(switch, (self.now - 60, self.now - 60))
        expired = W["rfc3339"](self.now - 60)
        code, lines = self.run_carry(state="status")
        self.assertEqual((code, lines), (0, [
            f"carry opt-in expired at {expired}; run held-tool-watch carry on to renew",
            f"carry expired at {expired}; last: no carry pass recorded",
        ]))
        self.assertFalse(W["carry_enabled"](str(self.root), self.now))

    def test_carry_off_removes_the_switch(self):
        code, lines = self.run_carry(state="off")
        self.assertEqual((code, lines), (0, ["carry off; last: no carry pass recorded"]))
        self.assertFalse(Path(self.root, W["CARRY_ON_FILE"]).exists())

    def test_carry_asks_for_the_full_intents_output_never_the_misses_list(self):
        # Under --json, `epr peer intents --misses` prints only the misses list (epr-cli peer.rs
        # render_intents): no device, so a carry that asked with it could never see a miss.
        self.run_carry()
        intents = [c["argv"] for c in self.calls if c["argv"][1:3] == ["peer", "intents"]]
        self.assertEqual(len(intents), 1)
        self.assertIn("--json", intents[0])
        self.assertNotIn("--misses", intents[0])
        self.assertEqual(len(self.claims()), 1, "the miss is seen and claimed")

    def test_uses_device_head_exact_sha_and_ignores_own_device_misses(self):
        own = self.miss(announcer="did:key:z6MkLOCAL", project="own")
        self.misses.insert(0, own)
        code, _ = self.run_carry()
        self.assertEqual(code, 0)
        self.assertEqual(self.calls[0]["argv"][0], str(self.head))
        self.assertEqual(len(self.claims()), 1)
        self.assertIn("eprfs", self.claims()[0]["argv"])
        self.assertNotIn("own", self.claims()[0]["argv"])
        fetches = [c["argv"] for c in self.calls if c["argv"][:3] == ["git", "-C", str(self.repo)]
                   and "fetch" in c["argv"]]
        self.assertEqual(fetches, [["git", "-C", str(self.repo), "fetch", "--no-tags", "origin",
                                    self.commit]])

    def test_closure_recomputation_imports_only_the_runners_orchestrator(self):
        code, _ = self.run_carry()
        self.assertEqual(code, 0)
        [call] = [c for c in self.calls if c["argv"][:3] == ["node", "--input-type=module", "--eval"]]
        candidate = call["env"]["HELD_TOOL_CARRY_CANDIDATE_ROOT"]
        trusted = call["env"]["HELD_TOOL_CARRY_TRUSTED_ROOT"]
        self.assertEqual(trusted, W["REPO_ROOT"])
        self.assertNotEqual(candidate, trusted)
        self.assertEqual(call["cwd"], trusted)
        self.assertIn("`${trusted}/${p}`", call["argv"][3])
        self.assertNotIn("`${candidate}/genesis/orchestrator", call["argv"][3])
        self.assertIn("loadGateRegistry(trusted)", call["argv"][3])
        self.assertIn("HELD_TOOL_CARRY_EXECUTION_ROOT", call["argv"][3])
        self.assertTrue(Path(call["env"]["HELD_TOOL_CARRY_EXECUTION_ROOT"]).name.startswith("held-tool-closure-"))
        self.assertFalse(str(call["env"]["HELD_TOOL_CARRY_EXECUTION_ROOT"]).startswith(candidate + os.sep))
        self.assertNotEqual(call["env"]["HELD_TOOL_CARRY_CARGO_HOME"], candidate)

    def test_candidate_checkout_disables_hooks_lfs_and_attributes_filters(self):
        code, _ = self.run_carry()
        self.assertEqual(code, 0)
        [checkout] = [c for c in self.calls if "worktree" in c["argv"] and "add" in c["argv"]]
        argv = checkout["argv"]
        for setting in ("core.hooksPath=/dev/null", "core.attributesFile=/dev/null",
                        "filter.lfs.smudge=", "filter.lfs.clean=", "filter.lfs.process=",
                        "filter.lfs.required=false"):
            self.assertIn(setting, argv)
        self.assertEqual(checkout["env"]["GIT_LFS_SKIP_SMUDGE"], "1")
        self.assertEqual(checkout["env"]["GIT_ATTR_NOSYSTEM"], "1")

    def test_closure_mismatch_and_unfetchable_commit_make_no_claim(self):
        for mode in ("mismatch", "unfetchable"):
            with self.subTest(mode=mode):
                self.calls.clear()
                self.computed = "c" * 64 if mode == "mismatch" else self.closure
                self.fetch = 1 if mode == "unfetchable" else 0
                code, lines = self.run_carry()
                self.assertEqual(code, 0)
                self.assertEqual(self.claims(), [])
                self.assertEqual(len(lines), 1)
                self.assertIn("closure mismatch" if mode == "mismatch" else "not fetchable", lines[0])

    def test_no_free_seat_or_local_landing_holder_makes_no_carry_claim(self):
        for mode in ("seat", "landing"):
            with self.subTest(mode=mode):
                self.calls.clear()
                self.room = ({"verdict": "refuse", "axis": "seats:cargo", "reason": "full"}
                             if mode == "seat" else {"verdict": "admit", "reason": "free"})
                self.holder = {"holder": "local-lander"} if mode == "landing" else None
                code, lines = self.run_carry()
                self.assertEqual(code, 0)
                self.assertEqual(self.claims(), [])
                self.assertIn("cargo seat" if mode == "seat" else "local dev-integration", lines[-1])

    def test_failed_gate_verdict_settles_the_claim_and_releases_the_cargo_seat(self):
        self.gate_code = 7
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        [claim] = self.claims()
        valid = claim["argv"][claim["argv"].index("--valid-for") + 1]
        self.assertEqual(valid, "600")  # p90*3 floors to ten minutes
        self.assertEqual(len(self.claims()), 1)
        self.assertIn("only the gate's verdict stands", lines[-1])
        self.assertEqual(len(self.releases()), 0)
        self.assertTrue(any("release" in c["argv"] and "cargo" in c["argv"] for c in self.calls))

    def test_green_without_a_verdict_releases_the_claim_and_retries_release_once(self):
        self.gate_verdict = False
        self.release_failures = 1
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertIn("emitted no gate verdict", lines[-1])
        self.assertEqual(len(self.releases()), 2)

    def test_unparseable_successful_claim_is_replayed_released_and_declined_once(self):
        self.claim_replies = [
            (0, "not json", ""),
            (0, json.dumps({"commitment": self.commitment}), ""),
        ]
        self.release_failures = 1
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertEqual(len(self.claims()), 2)
        self.assertEqual(len(self.releases()), 2)
        self.assertEqual(self.gates(), [])
        self.assertEqual(len(lines), 1)
        self.assertIn("claim returned no commitment; replayed claim released; gate not started", lines[0])
        declines = [call for call in self.calls if "say" in call["argv"]]
        self.assertEqual(len(declines), 1)

    def test_capacity_is_rechecked_immediately_before_gate_and_each_refusal_declines(self):
        for axis, refused in (("ram", "hard"), ("room", "refuse"), ("pool", "hard")):
            with self.subTest(axis=axis):
                self.calls.clear()
                self.capacity = {"ram": "ok", "room": "admit", "pool": "ok"}
                self.capacity[axis] = refused
                self.misses = [self.miss(intent=f"bafyreiintentintentintentintentintentintentintent{axis}")]
                code, lines = self.run_carry()
                self.assertEqual(code, 0)
                self.assertEqual(self.gates(), [])
                self.assertEqual(len(self.releases()), 1)
                self.assertIn("refused", lines[-1])
                self.assertEqual(len([c for c in self.calls if "say" in c["argv"]]), 1)

    def test_gate_gets_head_epr_and_no_gate_evidence(self):
        with mock.patch.dict(os.environ, {"GATE_EVIDENCE": "reuse"}):
            code, _ = self.run_carry()
        self.assertEqual(code, 0)
        [gate] = self.gates()
        self.assertEqual(gate["argv"], ["just", "gate", "eprfs"])
        self.assertEqual(gate["env"]["EPR_BIN"], str(self.head))
        self.assertNotIn("GATE_EVIDENCE", gate["env"])

    def test_never_pushes_or_lands_and_removes_its_worktree_and_target_slot(self):
        self.make_slot = True
        code, _ = self.run_carry()
        self.assertEqual(code, 0)
        words = [word for call in self.calls for word in call["argv"]]
        self.assertNotIn("push", words)
        self.assertNotIn("--land", words)
        self.assertFalse(any((self.repo / ".worktrees").iterdir()))
        self.assertEqual(list((self.pool / "family").rglob(f"*-wt-carry-{self.commit[:12]}-eprfs")), [])

    def test_unconfirmed_gate_stop_retains_claim_worktree_slot_and_cargo_seat(self):
        self.gate_stop = "off"
        self.gate_settled = False
        self.make_slot = True
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertIn("claim, worktree, target slot and cargo seat retained", lines[-1])
        self.assertEqual(self.releases(), [])
        self.assertFalse(any("release" in c["argv"] and "cargo" in c["argv"] for c in self.calls))
        self.assertTrue((self.repo / ".worktrees" / f"carry-{self.commit[:12]}-eprfs").exists())

    def test_gate_runner_exception_after_start_retains_every_claimed_asset(self):
        def gate_raises(*_args, **_kwargs):
            raise RuntimeError("gate runner transport broke")

        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            code = W["cmd_carry"](self.args(), runner=self.runner, gate_runner=gate_raises)
        self.assertEqual(code, 1)
        self.assertIn("gate state unconfirmed", out.getvalue())
        self.assertEqual(self.releases(), [])
        self.assertFalse(any("release" in c["argv"] and "cargo" in c["argv"] for c in self.calls))
        self.assertTrue((self.repo / ".worktrees" / f"carry-{self.commit[:12]}-eprfs").exists())

    def test_both_off_files_win_over_the_on_marker_without_reading_misses(self):
        for stop in (W["STOP_FILE"], W["CARRY_STOP_FILE"]):
            with self.subTest(stop=stop):
                Path(self.root, stop).touch()
                code, lines = self.run_carry()
                self.assertEqual((code, lines, self.calls), (0, [], []))
                Path(self.root, stop).unlink()

    def test_local_landing_yields_the_gate_and_releases_the_carry(self):
        self.gate_stop = "landing"
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertIn("yielded: local landing started", lines[-1])
        self.assertEqual(len(self.releases()), 1)

    def test_switching_off_during_the_gate_stops_and_releases(self):
        self.gate_stop = "off"
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertIn("yielded: carry switched off", lines[-1])
        self.assertEqual(len(self.releases()), 1)

    def test_expiring_during_the_gate_stops_and_releases(self):
        self.gate_stop = "expired"
        code, lines = self.run_carry()
        self.assertEqual(code, 1)
        self.assertIn("carry opt-in expired at", lines[-2])
        self.assertIn("yielded: carry switched off", lines[-1])
        self.assertEqual(len(self.releases()), 1)

    def test_dead_runner_parent_stops_the_gate_and_releases(self):
        self.gate_stop = "parent"
        alive = mock.Mock(side_effect=[True, True, False])
        with mock.patch.dict(W["cmd_carry"].__globals__, {"_parent_alive": alive}):
            code, lines = self.run_carry(parent_pid=12345)
        self.assertEqual(code, 1)
        self.assertIn("runner parent exited", lines[-1])
        self.assertEqual(len(self.releases()), 1)

    def test_decline_is_said_only_once_for_the_same_miss(self):
        self.computed = "c" * 64
        self.run_carry()
        self.run_carry()
        notes = [c for c in self.calls if "say" in c["argv"]]
        self.assertEqual(len(notes), 1)

    def test_no_cargo_pool_declines_once_without_fetch_claim_or_gate(self):
        with mock.patch.dict(W["cmd_carry"].__globals__, {"cargo_pool_root": lambda: None}):
            code, lines = self.run_carry()
        self.assertEqual(code, 0)
        self.assertIn("no cargo pool", lines[-1])
        words = [word for call in self.calls for word in call["argv"]]
        self.assertNotIn("fetch", words)
        self.assertEqual(self.claims(), [])
        self.assertEqual(self.gates(), [])

    def test_gate_stop_uses_gate_live_now_instead_of_killing_the_wrapper_group(self):
        popens, stop_calls = [], []
        run_log = self.root / "gate.run"

        class GateProcess:
            def __init__(self):
                self.done = False

            def poll(self):
                return 17 if self.done else None

            def wait(self, timeout=None):
                self.done = True
                return 17

        gate_process = GateProcess()

        def popen(argv, **kwargs):
            popens.append(list(argv))
            return gate_process

        def runner(argv, timeout=60, cwd=None, env=None):
            self.calls.append({"argv": list(argv), "timeout": timeout, "cwd": str(cwd), "env": env})
            if "open" in argv:
                run_log.write_text(json.dumps({"kind": "run-open"}) + "\n")
                return 0, json.dumps({"path": str(run_log)}), ""
            if "stop" in argv:
                stop_calls.append(list(argv))
                gate_process.done = True
                with run_log.open("a") as stream:
                    stream.write(json.dumps({"kind": "gate-stopped", "project": "eprfs"}) + "\n")
                    stream.write(json.dumps({"kind": "run-close", "outcome": "interrupted"}) + "\n")
                return 0, "confirmed", ""
            if "close" in argv:
                return 0, "", ""
            raise AssertionError(argv)

        globals_ = W["run_carried_gate"].__globals__
        with mock.patch.object(globals_["subprocess"], "Popen", side_effect=popen), \
                mock.patch.object(globals_["time"], "sleep", return_value=None), \
                mock.patch.object(globals_["os"], "killpg", side_effect=AssertionError("killpg")):
            code, _, _, stopped, returned_log, settled, verdict = W["run_carried_gate"](
                W["REPO_ROOT"], self.repo, self.commit, "eprfs", {}, W["time"].monotonic() + 10,
                lambda: "carry switched off", runner=runner)
        self.assertEqual((code, stopped, returned_log, settled, verdict),
                         (17, "carry switched off", str(run_log), True, False))
        [stop] = stop_calls
        self.assertIn("--now", stop)
        self.assertEqual(stop[stop.index("--run") + 1], str(run_log))
        self.assertEqual(popens, [["just", "gate", "eprfs"]])

    def test_failed_berth_who_check_stops_the_gate_and_failed_stop_retries_three_times(self):
        run_log = self.root / "gate-retry.run"

        class GateProcess:
            def poll(self):
                return None

        attempts = []

        def runner(argv, timeout=60, cwd=None, env=None):
            if "open" in argv:
                run_log.write_text(json.dumps({"kind": "run-open"}) + "\n")
                return 0, json.dumps({"path": str(run_log)}), ""
            if "stop" in argv:
                attempts.append(list(argv))
                return 9, "", "stop transport failed"
            raise AssertionError(argv)

        globals_ = W["run_carried_gate"].__globals__
        with mock.patch.object(globals_["subprocess"], "Popen", return_value=GateProcess()):
            result = W["run_carried_gate"](
                W["REPO_ROOT"], self.repo, self.commit, "eprfs", {}, W["time"].monotonic() + 10,
                mock.Mock(side_effect=RuntimeError("berth who failed")), runner=runner)
        self.assertEqual(len(attempts), 3)
        self.assertFalse(result[5])
        self.assertIn("stop unconfirmed after 3 attempts", result[2])

    def test_stale_sweep_requires_a_closed_run_and_no_process_path(self):
        worktree = self.repo / ".worktrees" / "carry-aaaaaaaaaaaa-eprfs"
        worktree.mkdir(parents=True, exist_ok=True)
        os.utime(worktree, (self.now - 1000, self.now - 1000))
        said = []
        globals_ = W["sweep_carry_worktrees"].__globals__
        with mock.patch.dict(globals_, {"_closed_carry_run": lambda *_: None,
                                        "_carry_paths_in_use": lambda *_: False}):
            W["sweep_carry_worktrees"](str(self.repo), 10, self.runner, self.now, said.append,
                                       str(self.pool))
        self.assertTrue(worktree.exists())
        self.assertIn("no closed gate-live run", said[-1])

        said.clear()
        with mock.patch.dict(globals_, {"_closed_carry_run": lambda *_: "closed.run",
                                        "_carry_paths_in_use": lambda *_: True}):
            W["sweep_carry_worktrees"](str(self.repo), 10, self.runner, self.now, said.append,
                                       str(self.pool))
        self.assertTrue(worktree.exists())
        self.assertIn("process still names its assets", said[-1])

    def test_process_scan_sees_worktree_cwd_and_cargo_target_dir(self):
        worktree = self.repo / ".worktrees" / "carry-aaaaaaaaaaaa-eprfs"
        worktree.mkdir(parents=True, exist_ok=True)
        slot = self.pool / "family" / "cd" / f"eprfs-wt-{worktree.name}"
        slot.mkdir(parents=True)
        proc = self.root / "proc"
        pid = proc / "123"
        pid.mkdir(parents=True)
        os.symlink(worktree, pid / "cwd")
        (pid / "environ").write_bytes(b"PATH=/bin\0")
        self.assertTrue(W["_carry_paths_in_use"](str(worktree), str(self.pool), str(proc)))
        (pid / "cwd").unlink()
        os.symlink(self.root, pid / "cwd")
        (pid / "environ").write_bytes(f"CARGO_TARGET_DIR={slot}\0".encode())
        self.assertTrue(W["_carry_paths_in_use"](str(worktree), str(self.pool), str(proc)))
        (pid / "environ").write_bytes(b"PATH=/bin\0")
        self.assertFalse(W["_carry_paths_in_use"](str(worktree), str(self.pool), str(proc)))

    def test_slow_carry_does_not_stop_announce_cadence(self):
        announce_calls, carry_calls = [], []
        clock = [0.0]

        def run_pass(argv, timeout, cancelled=None):
            announce_calls.append(list(argv))
            return 0, "", ""

        def start(argv):
            carry_calls.append(list(argv))
            return {"still": "running"}

        def sleep(seconds):
            clock[0] += seconds

        args = ["run", "--stasis-root", str(self.stasis), "--bin-store", str(self.store),
                "--repo-root", str(self.repo), "--every", "60", "--carry-every", "60", "--passes", "3"]
        globals_ = W["cmd_run"].__globals__
        with mock.patch.dict(globals_["time"].__dict__, {"monotonic": lambda: clock[0], "sleep": sleep}), \
                mock.patch.dict(globals_, {"run_pass": run_pass, "start_async_pass": start,
                                           "poll_async_pass": lambda job: None}), \
                contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(W["main"](args), 0)
        self.assertEqual(len(announce_calls), 3)
        self.assertEqual(len(carry_calls), 1)
        self.assertEqual({argv[2] for argv in announce_calls}, {"announce"})


LOCAL = "0123abcd-aaaa-bbbb-cccc-0000000000aa"   # another session moored on this device
SIBLING = "0123abcd-aaaa-bbbb-cccc-0000000000bb"  # a session moored on a sibling device
DID = "did:key:z6MkTESTdeviceTESTdeviceTESTdeviceTESTdevice"


class AnnounceTest(ObserveFixture):
    """`announce`: the observe pass, then one line per held call per addressee, read for once-ness
    from what a FAKE berth (on BERTH_BIN) recorded. No real journal, berth store or channel is touched."""

    def setUp(self):
        super().setUp()
        self.notes = self.root / "notes.jsonl"
        self.berth_log = self.root / "berth-calls.jsonl"
        self.status_file = self.root / "berth-status.json"
        self.status({"device": DID, "moorings": [{"session": SESSION}, {"session": LOCAL}],
                     "siblings": [{"device": "did:key:z6MkSIBLING", "session": SIBLING}]})
        fake = self.root / "fake-berth"
        fake.write_text(f"""#!/usr/bin/env python3
import json, os, sys, time
args = sys.argv[1:]
with open({str(self.berth_log)!r}, "a") as f:
    f.write(json.dumps(args) + "\\n")
def opt(name):
    if name in args:
        i = args.index(name); value = args[i + 1]; del args[i:i + 2]; return value
    return None
cmd = args.pop(0)
if os.path.exists({str(self.root / "berth-broken")!r}):
    sys.stderr.write("berth: store unavailable\\n"); sys.exit(4)
if cmd == "status":
    print(open({str(self.status_file)!r}).read()); sys.exit(0)
if cmd == "said":
    session = opt("--session")
    rows = [json.loads(l) for l in open({str(self.notes)!r})] if os.path.exists({str(self.notes)!r}) else []
    print(json.dumps([r for r in rows if r["session"] == session])); sys.exit(0)
if cmd == "say":
    session, to = opt("--session"), opt("--to")
    with open({str(self.notes)!r}, "a") as f:
        f.write(json.dumps({{"session": session, "to": to, "text": " ".join(args), "at": float(os.environ.get("FAKE_BERTH_NOW") or time.time()),
                            "from_device": None}}) + "\\n")
    sys.exit(0)
sys.exit(9)
""")
        fake.chmod(0o755)
        env = mock.patch.dict(os.environ, {"BERTH_BIN": str(fake)})
        env.start()
        self.addCleanup(env.stop)
        carry = mock.patch.dict(os.environ, {"HELD_TOOL_CARRY": "off"})
        carry.start()
        self.addCleanup(carry.stop)

    def status(self, data):
        """The full status (device id) and the --no-peer status the pass reads moorings and siblings from."""
        self.status_file.write_text(json.dumps(data))
        self.sessions.write_text(json.dumps({k: v for k, v in data.items() if k != "device"}))

    def hold_open(self, *ids):
        (self.root / "open-incidents.json").write_text(json.dumps({SESSION: list(ids)}))

    def said(self):
        if not self.notes.exists():
            return []
        return [json.loads(line) for line in self.notes.read_text().splitlines()]

    def announce(self, now):
        args = ["announce", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--bin-store", str(self.store), "--now", str(now)]
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err), \
                mock.patch.dict(os.environ, {"FAKE_BERTH_NOW": str(now)}):
            code = W["main"](args)
        return code, out.getvalue().splitlines(), err.getvalue().splitlines()

    def test_one_held_call_is_said_once_to_each_addressee_for_the_life_of_the_incident(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.write(use("t1", "Bash", T0, command="git rebase -i main"))
        # Minutes apart, then eight days on with the call still pending: no expiry, no repeat.
        for now in (T0 + 200, T0 + 260, T0 + 320, T0 + 900, T0 + 7000, T0 + 8 * 86400):
            self.assertEqual(self.announce(now), (0, [], []))
        said = self.said()
        self.assertEqual(sorted(n["to"] for n in said), sorted([LOCAL, SIBLING]))
        self.assertEqual({n["session"] for n in said}, {"held-tool-watch"})
        [text] = {n["text"] for n in said}
        kind, fields = W["parse_mark"](text)
        self.assertEqual((kind, fields), ("open", {"device": DID, "session": SESSION, "agent": "main",
                                                   "tool": "Bash", "since": W["rfc3339"](T0)}))
        self.assertIn("Nothing here approves, cancels or retries it", text)
        self.assertEqual(len(self.observes()), 6)  # the native side is told every pass; it folds them

    def test_nothing_of_the_call_itself_crosses_the_device(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        marker = "SECRETMARKER-9f3a"
        self.write(use("toolu_MARKERID", "Bash", T0, command=f"curl https://x.io/{marker} -d {marker}"))
        self.announce(T0 + 200)
        self.write(use("toolu_MARKERID", "Bash", T0, command=marker), result("toolu_MARKERID", T0 + 300))
        self.announce(T0 + 400)
        sent = self.berth_log.read_text() + self.notes.read_text()
        self.assertEqual(len(self.said()), 4)  # open and resumed, to two sessions
        for leaked in (marker, "toolu_MARKERID", "curl", "x.io", "cause", f"{SESSION}:7"):
            self.assertNotIn(leaked, sent)

    def test_recipients_are_capped_for_the_life_of_an_incident_then_the_household_once(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        many = [f"0123abcd-aaaa-bbbb-cccc-0000000001{i:02d}" for i in range(12)]
        self.status({"device": DID, "moorings": [{"session": SESSION}] + [{"session": s} for s in many[:10]],
                     "siblings": []})
        self.write(use("t1", "Bash", T0, command="ls"))
        self.announce(T0 + 200)
        self.status({"device": DID, "moorings": [{"session": s} for s in many[10:]], "siblings": []})
        self.announce(T0 + 300)
        self.announce(T0 + 400)
        targets = [n["to"] for n in self.said()]
        self.assertEqual(targets, many[:8] + [None])

    def berth_calls(self):
        if not self.berth_log.exists():
            return []
        return [json.loads(line)[0] for line in self.berth_log.read_text().splitlines()]

    def test_only_a_first_line_reads_the_full_status_and_a_quiet_pass_reads_none(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.announce(T0 + 200)  # nothing held yet: no store reader at all
        self.assertEqual(self.berth_calls(), ["said"])
        self.write(use("t1", "Bash", T0, command="ls"))
        self.announce(T0 + 300)  # first line: one full status, for the device id
        self.assertEqual(self.berth_calls()[1:], ["said", "status", "say", "say"])
        native = [c for c in self.calls() if c["argv"][:3] == ["session", "stasis", "status"]]
        for now in (T0 + 360, T0 + 420):  # held and told: neither the full status nor the native one
            self.announce(now)
        self.assertEqual(self.berth_calls()[5:], ["said", "said"])
        self.assertEqual(len([c for c in self.calls() if c["argv"][:3] == ["session", "stasis", "status"]]),
                         len(native))

    def test_a_session_that_moors_later_is_told_with_the_first_lines_device_id(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.status({"device": DID, "moorings": [{"session": SESSION}, {"session": LOCAL}], "siblings": []})
        self.write(use("t1", "Bash", T0, command="ls"))
        self.announce(T0 + 200)
        self.status({"device": "did:key:z6MkOTHERwrongwrongwrongwrong", "moorings": [{"session": SESSION},
                     {"session": LOCAL}], "siblings": [{"device": "did:key:z6MkSIB", "session": SIBLING}]})
        self.announce(T0 + 300)
        said = self.said()
        self.assertEqual([n["to"] for n in said], [LOCAL, SIBLING])
        self.assertEqual({W["parse_mark"](n["text"])[1]["device"] for n in said}, {DID})
        self.assertEqual(self.berth_calls().count("status"), 1)

    def test_resolution_says_resumed_once_then_nothing_more(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.write(use("t1", "Bash", T0, command="ls"))
        self.announce(T0 + 200)
        # Answered: the result is in the transcript. The native incident stays open until the agent's
        # turn ends, so the transcript, not the incident, says resumed.
        self.write(use("t1", "Bash", T0, command="ls"), result("t1", T0 + 2400))
        for now in (T0 + 2460, T0 + 2520, T0 + 9000):
            self.assertEqual(self.announce(now), (0, [], []))
        kinds = [(W["parse_mark"](n["text"])[0], n["to"]) for n in self.said()]
        self.assertEqual(sorted(kinds), sorted([("open", LOCAL), ("open", SIBLING),
                                                ("resumed", LOCAL), ("resumed", SIBLING)]))
        resumed = [n["text"] for n in self.said() if W["parse_mark"](n["text"])[0] == "resumed"]
        self.assertIn("is no longer pending", resumed[0])

    def test_a_pin_without_the_verb_gives_the_not_observed_line_and_no_note(self):
        self.pin(cid=CID_OLD)
        self.hold_open(f"{SESSION}:7")
        self.write(use("t1", "Bash", T0, command="ls"))
        code, out, err = self.announce(T0 + 200)
        self.assertEqual((code, err), (0, []))
        self.assertEqual(out, [W["NO_VERB_LINE"].format(session=SESSION)])
        self.assertEqual(self.said(), [])

    def test_nothing_is_said_while_the_native_side_holds_no_open_incident(self):
        self.pin()
        self.write(use("t1", "Bash", T0, command="ls"))
        self.assertEqual(self.announce(T0 + 200), (0, [], []))
        self.assertEqual(self.said(), [])

    def test_with_no_other_session_listed_the_line_goes_to_the_household(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.status({"device": DID, "moorings": [{"session": SESSION}], "siblings": []})
        self.write(use("t1", "Bash", T0, command="ls"))
        self.announce(T0 + 200)
        self.announce(T0 + 300)
        self.assertEqual([n["to"] for n in self.said()], [None])

    def test_an_unreadable_record_says_nothing_with_one_line_and_exit_1(self):
        self.pin()
        self.hold_open(f"{SESSION}:7")
        self.write(use("t1", "Bash", T0, command="ls"))
        (self.root / "berth-broken").touch()
        code, out, err = self.announce(T0 + 200)
        self.assertEqual((code, out, len(err)), (1, [], 1))
        self.assertTrue(err[0].startswith("held-tool-watch: cannot read what was already said"), err[0])
        self.assertEqual(self.said(), [])

    def test_the_off_switch_stops_a_running_loop_at_its_next_pass(self):
        args = ["run", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--bin-store", str(self.store), "--every", "0",
                "--passes", "3"]
        (self.root / W["STOP_FILE"]).touch()
        with contextlib.redirect_stderr(io.StringIO()) as err:
            self.assertEqual(W["main"](args), 0)
        self.assertIn("switched off", err.getvalue())
        self.assertFalse((self.root / W["LAST_PASS"]).exists())

    def test_a_failing_pass_backs_off_and_says_so_once_per_change_of_state(self):
        (self.root / "berth-broken").touch()
        self.write(use("t1", "Bash", 0, command="ls"))
        args = ["run", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--bin-store", str(self.store), "--every", "0",
                "--passes", "2"]
        sleeps = []
        clock = [0.0]

        def sleep(seconds):
            sleeps.append(seconds)
            clock[0] += seconds

        def failed_pass(argv, timeout, cancelled=None):
            return 1, "", "berth unavailable"

        globals_ = W["cmd_run"].__globals__
        with contextlib.redirect_stderr(io.StringIO()) as err, \
                mock.patch.dict(globals_["time"].__dict__, {"sleep": sleep, "monotonic": lambda: clock[0]}), \
                mock.patch.dict(globals_, {"run_pass": failed_pass}):
            self.assertEqual(W["main"](args), 0)
        lines = err.getvalue().splitlines()
        self.assertEqual(len(lines), 1, lines)
        self.assertIn("failing (exit 1), next pass in 1 s", lines[0])
        self.assertAlmostEqual(sum(sleeps), 1.0, delta=0.02)
        self.assertTrue(all(wait <= 0.1 for wait in sleeps))

    def dead(self, pid):
        import time as _time
        for _ in range(60):
            try:
                state = Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[0]
            except OSError:
                return True
            if state in ("Z", "X"):
                return True
            _time.sleep(0.05)
        return False

    def test_a_pass_takes_its_whole_process_group_with_it_and_nothing_else(self):
        import subprocess as sp
        decoy = sp.Popen(["sleep", "60"])  # outside the pass's group
        self.addCleanup(decoy.wait)
        self.addCleanup(decoy.kill)
        orphan, stuck, hung = (self.root / n for n in ("orphan.pid", "stuck.pid", "hung.pid"))
        # A child that forks a grandchild and exits: the grandchild ends with the pass.
        code, _, _ = W["run_pass"](["sh", "-c", f"sleep 60 & echo $! > {orphan}; exit 0"], timeout=10)
        self.assertEqual(code, 0)
        self.assertTrue(self.dead(int(orphan.read_text())))
        # A child that forks a grandchild and hangs past the timeout: both end, code None.
        code, _, _ = W["run_pass"](["sh", "-c", f"sleep 60 & echo $! > {stuck}; echo $$ > {hung}; sleep 60"],
                                   timeout=0.5)
        self.assertIsNone(code)
        self.assertTrue(self.dead(int(stuck.read_text())))
        self.assertTrue(self.dead(int(hung.read_text())))
        self.assertIsNone(decoy.poll())  # the decoy outside the group is untouched

    def test_a_call_inside_a_pass_has_its_own_timeout(self):
        with self.assertRaises(W["subprocess"].TimeoutExpired):
            W["run_command"](["sleep", "60"], timeout=0.3)

    def test_the_device_runner_is_one_per_device_and_announces_through_fresh_passes(self):
        import time as _time
        self.pin()
        self.hold_open(f"{SESSION}:7")
        start = _time.time() - 600
        self.write(use("t1", "Bash", start, command="ls"))
        args = ["run", "--transcripts", str(self.transcripts), "--sessions-json", str(self.sessions),
                "--stasis-root", str(self.stasis), "--bin-store", str(self.store), "--every", "0"]
        lock = self.root / W["LOOP_LOCK"]
        with W["file_lock"](str(lock), 0) as mine:
            self.assertTrue(mine)
            self.assertEqual(W["main"](args + ["--passes", "2"]), 0)  # another runner holds it: no pass
        self.assertEqual(self.said(), [])
        self.assertEqual(W["main"](args + ["--passes", "2"]), 0)
        self.assertEqual(sorted(n["to"] for n in self.said()), sorted([LOCAL, SIBLING]))
        self.assertIn("exit 0", (self.root / W["LAST_PASS"]).read_text())


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


REAL_INTENTS = HERE / "fixtures/peer-intents-json.real.json"


class RealIntentsShapeTest(unittest.TestCase):
    """Carry reads the shape the real CLI prints, captured from `epr peer intents --json`."""

    def test_the_captured_full_output_yields_the_announced_miss_and_skips_unannounced_rows(self):
        data = json.loads(REAL_INTENTS.read_text())
        self.assertIsInstance(data, dict)
        self.assertIn("device", data)
        self.assertIn("misses", data)
        before = min(m["waitUntil"] for m in data["misses"] if m.get("announcer")) - 1
        rows = W["_eligible_misses"](data, before)
        self.assertEqual([(r["project"], r["commit"]) for r in rows],
                         [("eprfs-live-corpus", "d12c22740c46766d8a5e4b68ce86e406b74244c2")])

    def test_the_misses_only_list_is_what_hid_every_miss(self):
        data = json.loads(REAL_INTENTS.read_text())
        before = min(m["waitUntil"] for m in data["misses"] if m.get("announcer")) - 1
        self.assertEqual(W["_eligible_misses"](data["misses"], before), [])

    def test_a_built_epr_prints_device_and_misses_under_intents_json(self):
        epr = os.environ.get("EPR_BIN")
        if not epr or not os.access(epr, os.X_OK):
            self.skipTest("no EPR_BIN to run; the captured fixture stands in")
        code, out, err = W["run_command"]([epr, "peer", "intents", "--json", "--root",
                                           W["main_checkout"](W["REPO_ROOT"])], timeout=60)
        if code != 0:
            self.skipTest(f"epr peer intents exited {code} here: {' '.join(err.split())[:120]}")
        data = json.loads(out)
        self.assertIsInstance(data, dict)
        self.assertIn("device", data)
        self.assertIsInstance(data.get("misses"), list)


if __name__ == "__main__":
    unittest.main()

