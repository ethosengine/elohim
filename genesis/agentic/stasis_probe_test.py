"""stasis-probe: the permission verdicts are judged from recorded evidence, and the shapes a
reviewer showed could read as a false `yes` read as `no` or `unknown` instead.

Run: python3 -m unittest genesis.agentic.stasis_probe_test
"""
import runpy
import unittest
from pathlib import Path

_NS = runpy.run_path(str(Path(__file__).parent / "bin/stasis-probe"))
judge = _NS["judge_permission"]
MARK = _NS["DENY_MARK"]

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


if __name__ == "__main__":
    unittest.main()
