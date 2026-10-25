#!/usr/bin/env python3
"""The session covenant's hook legs (plan yes-plan-to-build-misty-flurry, operator 2026-10-19).

  blind-reader-review.py      PostToolUse — loop line, or the floor line under a held clause
  covenant-subagent-stamp.py  SubagentStart — carries what is held into a spawned agent
                              PreToolUse:Bash — a subagent never records a clause

The `epr` here is a stub on $EPR_BIN: `covenant check` answers $STUB_CHECK (or exits 2 when it is
unset, which is what an `epr` without the subcommand does), `covenant show` answers $STUB_SHOW.
Every invocation is logged to $STUB_LOG so a test can assert what was (not) asked.

Run: python3 -m unittest discover -s .claude/hooks/__tests__ -p 'covenant_hooks_test.py'
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HOOKS = Path(__file__).resolve().parent.parent
REPO = HOOKS.parent.parent
REVIEW = HOOKS / "blind-reader-review.py"
STAMP = HOOKS / "covenant-subagent-stamp.py"
FEATURE = REPO / "genesis/a2o/features/compute/ci-runner-stewarded.feature"

STUB = '''#!/usr/bin/env python3
import json, os, sys
argv = sys.argv[1:]
with open(os.environ["STUB_LOG"], "a") as fh:
    fh.write(json.dumps(argv) + "\\n")
key = {"check": "STUB_CHECK", "show": "STUB_SHOW"}.get(argv[1] if len(argv) > 1 else "")
if argv[:1] == ["covenant"] and key and os.environ.get(key):
    print(os.environ[key])
    sys.exit(0)
sys.stderr.write("epr: invalid arguments: unknown command `covenant`\\n")
sys.exit(2)
'''

HELD = {"status": "held", "variable": True, "clause": "bafyreiclause", "party": "human:matthew",
        "floor": "one considered fresh read; fix its correctness findings; close by judgment",
        "reason": "clause covers the path"}
NOT_HELD = {"status": "not-held", "variable": True, "clause": None, "party": None, "floor": None,
            "reason": "no clause covers the path"}


class CovenantHooksCase(unittest.TestCase):
    def setUp(self):
        self.assertTrue(FEATURE.exists(), FEATURE)
        self.tmp = Path(tempfile.mkdtemp(prefix="covenant-hooks-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.log = self.tmp / "stub.log"
        self.stub = self.tmp / "epr"
        self.stub.write_text(STUB.replace("#!/usr/bin/env python3", f"#!{sys.executable}", 1))
        self.stub.chmod(0o755)

    def invoke(self, hook: Path, payload: dict, **env_extra):
        env = {k: v for k, v in os.environ.items()
               if k not in ("STUB_CHECK", "STUB_SHOW", "CLAUDE_CODE_SESSION_ID")}
        env.update({"CLAUDE_PROJECT_DIR": str(REPO), "EPR_BIN": str(self.stub),
                    "STUB_LOG": str(self.log), "COVENANT_WITNESS_ROOT": str(self.tmp),
                    "ELOHIM_SESSION_ID": "envelope-key", "ELOHIM_SESSION_OWNER_PID": str(os.getpid()), "EPR_SESSION_RESOLVING": "1"})
        env.update({k: (json.dumps(v) if isinstance(v, dict) else v) for k, v in env_extra.items()})
        result = subprocess.run([sys.executable, str(hook)], input=json.dumps(payload),
                              capture_output=True, text=True, env=env, timeout=60)
        if self.log.exists():
            for line in self.log.read_text().splitlines():
                call = json.loads(line)
                if "--session" in call:
                    self.assertEqual(call[call.index("--session") + 1], "envelope-key")
        return result

    def context(self, result, event: str) -> str:
        self.assertEqual(result.returncode, 0, result.stderr)
        if not result.stdout.strip():
            return ""
        out = json.loads(result.stdout)["hookSpecificOutput"]
        self.assertEqual(out["hookEventName"], event)
        return out.get("additionalContext", "")

    def calls(self) -> list[list[str]]:
        if not self.log.exists():
            return []
        return [json.loads(l) for l in self.log.read_text().splitlines() if l.strip()]

    def review(self, **env):
        payload = {"session_id": "sess-1", "tool_name": "Edit", "hook_event_name": "PostToolUse",
                   "tool_input": {"file_path": str(FEATURE)}}
        payload.update(env.pop("payload_extra", {}))
        return self.context(self.invoke(REVIEW, payload, **env), "PostToolUse")

    # ── blind-reader-review ─────────────────────────────────────────────────────────────────
    def test_no_clause_keeps_the_loop_and_offers_one_question(self):
        ctx = self.review(STUB_CHECK=NOT_HELD)
        self.assertIn("repeat with another blind reader until READY", ctx)
        self.assertIn("offer the steward of record that floor as ONE question", ctx)
        self.assertIn("never take the floor yourself", ctx)
        check = next(a for a in self.calls() if a[:2] == ["covenant", "check"])
        self.assertEqual(check[check.index("--session") + 1], "envelope-key")
        self.assertEqual(check[check.index("--rule") + 1], "a2o-story-blind-reader-review")
        self.assertEqual(check[check.index("--path") + 1],
                         "genesis/a2o/features/compute/ci-runner-stewarded.feature")

    def test_a_held_clause_gives_the_floor_not_the_loop(self):
        ctx = self.review(STUB_CHECK=HELD)
        self.assertIn("[BLIND-READER REVIEW · COVENANT]", ctx)
        self.assertIn("bafyreiclause", ctx)
        self.assertIn("one considered fresh read", ctx)
        self.assertIn("Be careful", ctx)
        self.assertNotIn("until READY", ctx)
        rows = (self.tmp / ".claude/data/governance-findings.jsonl").read_text().splitlines()
        row = json.loads(rows[-1])
        self.assertEqual(row["gate"], "blind-reader-review")
        self.assertIn("variance: bafyreiclause", row["witness"])
        self.assertIn("agent: main-session", row["witness"])

    def test_an_unreadable_covenant_applies_the_full_rule_and_says_so(self):
        ctx = self.review()  # stub exits 2: an `epr` without `covenant`
        self.assertIn("until READY", ctx)
        self.assertIn("Covenant unreadable", ctx)
        self.assertIn("The full rule applies", ctx)

    def test_a_subagent_is_not_told_to_ask_the_operator(self):
        ctx = self.review(STUB_CHECK=NOT_HELD,
                          payload_extra={"agent_id": "a-1", "agent_type": "general-purpose"})
        self.assertIn("until READY", ctx)
        self.assertNotIn("ONE question", ctx)

    # ── SubagentStart stamp ─────────────────────────────────────────────────────────────────
    SHOW = {"session": "sess-1", "steward": "human:matthew",
            "purpose": {"purpose": "authoring the runner story", "writeSet": ["genesis/a2o/**"]},
            "variances": [{"clause": "bafyreiclause", "rule": "a2o-story-blind-reader-review",
                           "paths": ["genesis/a2o/features/compute/**"], "floor": "one read",
                           "party": "human:matthew", "validUntil": None}],
            "clauses": []}

    def stamp(self, agent_type: str, **env):
        payload = {"session_id": "sess-1", "hook_event_name": "SubagentStart",
                   "agent_id": "a-1", "agent_type": agent_type}
        return self.context(self.invoke(STAMP, payload, **env), "SubagentStart")

    def test_a_spawned_agent_is_stamped_with_what_is_held(self):
        ctx = self.stamp("general-purpose", STUB_SHOW=self.SHOW)
        self.assertIn("[SESSION COVENANT]", ctx)
        self.assertIn("steward of record: human:matthew", ctx)
        self.assertIn("authoring the runner story", ctx)
        self.assertIn("`a2o-story-blind-reader-review` is held at its floor within "
                      "genesis/a2o/features/compute/**", ctx)
        self.assertIn("You cannot grant, extend or record", ctx)

    def test_the_blind_reader_is_never_stamped(self):
        self.assertEqual(self.stamp("blind-reader", STUB_SHOW=self.SHOW), "")
        self.assertEqual(self.calls(), [], "the isolated reader's launch must not even read")

    def test_nothing_held_or_unreadable_stamps_nothing(self):
        empty = {"session": "sess-1", "steward": None, "purpose": None, "variances": [],
                 "clauses": []}
        self.assertEqual(self.stamp("general-purpose", STUB_SHOW=empty), "")
        self.assertEqual(self.stamp("general-purpose"), "")

    # ── PreToolUse:Bash guard ───────────────────────────────────────────────────────────────
    def guard(self, command: str, agent: bool):
        payload = {"session_id": "sess-1", "hook_event_name": "PreToolUse", "tool_name": "Bash",
                   "tool_input": {"command": command}}
        if agent:
            payload.update({"agent_id": "a-1", "agent_type": "general-purpose"})
        result = self.invoke(STAMP, payload)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)["hookSpecificOutput"] if result.stdout.strip() else None

    def test_a_subagent_cannot_record_a_clause(self):
        for command in ("epr covenant record --kind variesRule --rule x",
                        "brit-build-ref --repo . covenant put --session s --kind k"):
            out = self.guard(command, agent=True)
            self.assertEqual(out["permissionDecision"], "deny", command)

    def test_reads_and_the_main_session_pass(self):
        self.assertIsNone(self.guard("epr covenant check --rule x --path p", agent=True))
        self.assertIsNone(self.guard("epr covenant record --kind variesRule", agent=False))


if __name__ == "__main__":
    unittest.main()
