"""stasis-probe: the permission verdicts are judged from recorded evidence, and the shapes a
reviewer showed could read as a false `yes` read as `no` or `unknown` instead.

Run: python3 -m unittest genesis.agentic.stasis_probe_test
"""
import json
import os
import runpy
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

_NS = runpy.run_path(str(Path(__file__).parent / "bin/stasis-probe"))
judge = _NS["judge_permission"]
MARK = _NS["DENY_MARK"]
emit = _NS["harness_capabilities"]

REQ_KEYS = ["cwd", "hook_event_name", "permission_mode", "prompt_id", "session_id",
            "tool_input", "tool_name", "transcript_path"]


def event(name, command=None, keys=None, **payload):
    if command is not None:
        payload["tool_input"] = {"command": command}
    return {"event": name, "keys": keys if keys is not None else sorted(payload), "payload": payload}


def good_run():
    return [
        event("UserPromptSubmit"),
        event("PreToolUse", "touch probe-ask-marker.txt", tool_use_id="t1"),
        event("PermissionRequest", "touch probe-ask-marker.txt", keys=REQ_KEYS),
        event("PreToolUse", "echo second-step", tool_use_id="t2"),
        event("PostToolUse", "echo second-step", tool_use_id="t2", duration_ms=12),
        event("Stop", keys=["background_tasks", "last_assistant_message", "session_id",
                            "stop_hook_active", "transcript_path"]),
    ]


GOOD_RESULT = {"result": f"{MARK}: denied", "permission_denials": [
    {"tool_name": "Bash", "tool_use_id": "t1", "tool_input": {"command": "touch probe-ask-marker.txt"}}]}


def verdicts(ev, result=GOOD_RESULT, marker=False, rc=0):
    return {k.split("[")[0]: v["verdict"] for k, v in judge("none", rc, result, ev, marker).items()}


class JudgePermission(unittest.TestCase):
    def test_a_faithful_run_is_all_yes(self):
        self.assertEqual(set(verdicts(good_run()).values()), {"yes"})

    def test_missing_request_fields_is_no(self):
        ev = good_run()
        ev[2] = event("PermissionRequest", "touch probe-ask-marker.txt", keys=["tool_name", "tool_input"])
        self.assertEqual(verdicts(ev)["permission-request-fields"], "no")

    def test_a_tool_use_id_on_the_request_is_no(self):
        ev = good_run()
        ev[2] = event("PermissionRequest", "touch probe-ask-marker.txt", keys=REQ_KEYS + ["tool_use_id"])
        self.assertEqual(verdicts(ev)["permission-request-fields"], "no")

    def test_an_echo_before_the_deny_does_not_prove_continuation(self):
        ev = good_run()
        ev.insert(1, event("PostToolUse", "echo second-step", tool_use_id="t0", duration_ms=3))
        del ev[5]  # drop the echo that followed the request
        self.assertEqual(verdicts(ev)["deny-continues-turn"], "no")

    def test_the_denied_command_running_is_no(self):
        self.assertEqual(verdicts(good_run(), marker=True)["deny-continues-turn"], "no")

    def test_empty_permission_denials_is_no(self):
        result = {"result": f"{MARK}: denied", "permission_denials": []}
        self.assertEqual(verdicts(good_run(), result)["result-permission-denials"], "no")

    def test_a_non_numeric_duration_is_no(self):
        ev = good_run()
        ev[4] = event("PostToolUse", "echo second-step", tool_use_id="t2", duration_ms=None)
        self.assertEqual(verdicts(ev)["posttooluse-duration-ms"], "no")

    def test_no_request_leaves_dependent_verdicts_unknown(self):
        ev = [e for e in good_run() if e["event"] != "PermissionRequest"]
        v = verdicts(ev)
        self.assertEqual(v["permission-request-on-ask-rule"], "no")
        for cid in ("deny-message-reaches-agent", "deny-continues-turn", "permission-request-fields"):
            self.assertEqual(v[cid], "unknown", cid)

    def test_a_failed_run_is_unknown_not_no(self):
        self.assertEqual(verdicts(good_run(), rc=1)["permission-request-on-ask-rule"], "unknown")


def report(stamp, checks, version=None):
    return {"schema": "stasis-probe/1", "stamp": stamp, "claudeVersion": version,
            "capabilities": {cid: {"verdict": value, "evidence": "fixture"}
                             for cid, value in checks.items()}}


class EmitCapabilities(unittest.TestCase):
    def test_verdict_mapping_and_report_provenance(self):
        source = report("20261030T143310Z", {
            "proven": "yes", "disproved": "no", "unmeasured": "unknown",
            "needs-human": "interactive", "missing": None,
        }, "2.1.314 (Claude Code)")
        del source["capabilities"]["missing"]["verdict"]
        caps = emit([source])["harnessCapabilities"]
        self.assertEqual({cid: row["status"] for cid, row in caps.items()}, {
            "proven": "yes", "disproved": "no", "unmeasured": "unsupported",
            "needs-human": "unsupported", "missing": "unsupported",
        })
        for row in caps.values():
            self.assertEqual(row["evidence"], "probe/20261030T143310Z")
            self.assertEqual(row["cliVersion"], source["claudeVersion"])

    def test_newest_stamp_wins_independent_of_argument_order(self):
        older = report("20261030T143310Z", {"retracted": "yes", "confirmed": "no"}, "old")
        newer = report("20261030T163031Z", {"retracted": "no", "confirmed": "yes"}, "new")
        self.assertEqual(emit([newer, older]), emit([older, newer]))
        caps = emit([newer, older])["harnessCapabilities"]
        self.assertEqual(caps["retracted"], {
            "status": "no", "evidence": "probe/20261030T163031Z", "cliVersion": "new"})
        self.assertEqual(caps["confirmed"]["status"], "yes")

    def test_unexercised_checks_never_replace_a_yes_or_its_provenance(self):
        older = report("20261030T143310Z", {"proven": "yes"}, "old")
        for value in ("unknown", "interactive", None):
            with self.subTest(value=value):
                later = report("20261030T163031Z", {"proven": value})
                self.assertEqual(emit([later, older]), emit([older]))
        self.assertEqual(emit([older, report("20261030T163031Z", {})]), emit([older]))

    def test_latest_unknown_does_not_resurrect_a_retracted_yes(self):
        reports = [report("20261030T143310Z", {"cap": "yes"}),
                   report("20261030T145038Z", {"cap": "no"}),
                   report("20261030T163031Z", {"cap": "unknown"})]
        self.assertEqual(emit(reports)["harnessCapabilities"]["cap"], {
            "status": "unsupported", "evidence": "probe/20261030T163031Z", "cliVersion": None})

    def test_partial_interactive_report_keeps_headless_evidence_and_null_version(self):
        headless = report("20261030T143310Z", {"keeper": "yes", "dialog": "interactive"}, "old")
        interactive = report("20261030T163031Z", {"dialog": "yes"})
        del interactive["claudeVersion"]
        caps = emit([headless, interactive])["harnessCapabilities"]
        self.assertEqual(caps["keeper"]["evidence"], "probe/20261030T143310Z")
        self.assertEqual(caps["dialog"], {
            "status": "yes", "evidence": "probe/20261030T163031Z", "cliVersion": None})
        self.assertEqual(caps.get("never-probed", {}).get("status", "unsupported"), "unsupported")

    def test_invalid_reports_are_rejected(self):
        source = report("20261030T143310Z", {"cap": "yes"})
        for invalid in ({**source, "schema": "unrelated/1"}, {**source, "stamp": "yesterday"},
                        {**source, "capabilities": []}, {**source, "capabilities": {"cap": "yes"}}):
            with self.subTest(invalid=invalid), self.assertRaises(ValueError):
                emit([invalid])

    def test_cli_emits_only_json_without_running_a_probe_or_creating_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            sources = [report("20261030T143310Z", {"keeper": "yes"}, "old"),
                       report("20261030T163031Z", {"dialog": "yes"})]
            paths = [root / f"report-{i}.json" for i in range(len(sources))]
            for path, source in zip(paths, sources):
                path.write_text(json.dumps(source))
            result = subprocess.run([
                sys.executable, str(Path(__file__).parent / "bin/stasis-probe"),
                "--out", str(root / "unused"), "--emit-capabilities", *map(str, paths),
            ], capture_output=True, text=True, env={**os.environ, "PATH": ""})
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(result.stderr, "")
            self.assertEqual(json.loads(result.stdout), emit(sources))
            self.assertFalse((root / "unused").exists())


if __name__ == "__main__":
    unittest.main()
