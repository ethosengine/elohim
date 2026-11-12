#!/usr/bin/env python3
"""Offline hook fixtures; retire with native sense poll/reconcile/paydown judgement."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
HOOK = ROOT / ".claude/hooks/dependabot-inflight.py"


class HookTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix=".inflight-test-", dir=HERE)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.ledger = self.root / ".claude/data/dependabot-findings.jsonl"
        self.ledger.parent.mkdir(parents=True)
        self.row = {"package": "fixture-crate", "ecosystem": "cargo", "manifest_path": "crate/Cargo.lock",
                    "severity": "critical", "first_patched": "0.2.5", "status": "open"}
        self.env = {**os.environ, "CLAUDE_PROJECT_DIR": str(self.root), "PYTHONDONTWRITEBYTECODE": "1"}

    def write(self, rows):
        self.ledger.write_text("".join(json.dumps(row) + "\n" for row in rows))

    def invoke(self, tool="Edit", **data):
        start = time.monotonic()
        result = subprocess.run([sys.executable, str(HOOK)], input=json.dumps({"tool_name": tool, "tool_input": data, "cwd": str(self.root)}),
                                env=self.env, capture_output=True, text=True, timeout=3)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        return result.stdout, time.monotonic() - start

    def test_edit_write_bash_context_and_read_only_budget(self):
        self.write([self.row, self.row])
        before = self.ledger.read_bytes()
        for tool, data in (("Edit", {"file_path": "crate/Cargo.toml"}),
                           ("Write", {"file_path": str(self.root / "crate/Cargo.lock")}),
                           ("Bash", {"command": "cd crate && cat Cargo.lock"}),
                           ("Bash", {"command": "cat<crate/Cargo.lock"}),
                           ("Bash", {"command": "cargo test --manifest-path=crate/Cargo.toml"})):
            text, elapsed = self.invoke(tool, **data)
            output = json.loads(text)["hookSpecificOutput"]
            self.assertEqual(output["hookEventName"], "PostToolUse")
            self.assertIn("fixture-crate (cargo) · 2 open", output["additionalContext"])
            self.assertIn("(cd crate && cargo update -p fixture-crate --precise 0.2.5)", output["additionalContext"])
            self.assertEqual(len(output["additionalContext"].splitlines()), 1)
            self.assertLess(elapsed, .3)
        self.assertEqual(self.ledger.read_bytes(), before)

    def test_silence_for_absent_empty_unrelated_closed_and_corrupt_ledger(self):
        self.assertEqual(self.invoke(file_path="crate/Cargo.lock")[0], "")
        for rows in ([], [{**self.row, "status": "closed"}], [{**self.row, "manifest_path": "elsewhere/Cargo.lock"}]):
            self.write(rows)
            self.assertEqual(self.invoke(file_path="crate/Cargo.lock")[0], "")
        self.write([self.row])
        self.assertEqual(self.invoke(file_path="crate/src/lib.rs")[0], "")
        self.assertEqual(self.invoke("Bash", command="echo hello")[0], "")
        self.assertEqual(self.invoke(file_path="/outside/crate/Cargo.lock")[0], "")
        self.ledger.write_text("broken json")
        self.assertEqual(self.invoke(file_path="crate/Cargo.lock")[0], "")

    def test_pnpm_root_mirrored_override_and_unpatchable(self):
        self.write([{**self.row, "manifest_path": "pnpm-lock.yaml", "ecosystem": "npm", "package": "@scope/pkg"},
                    {**self.row, "manifest_path": "pnpm-lock.yaml", "ecosystem": "npm", "package": "no-fix", "first_patched": None}])
        text, _ = self.invoke(file_path="pnpm-workspace.yaml")
        context = json.loads(text)["hookSpecificOutput"]["additionalContext"]
        self.assertIn('pnpm.overrides["@scope/pkg"]="0.2.5"', context)
        self.assertIn("mirror overrides in pnpm-workspace.yaml", context)
        self.assertIn("no patched version", context)

    def test_cluster_limit_and_478_row_budget(self):
        self.write([{**self.row, "package": f"p{n % 12}"} for n in range(478)])
        text, elapsed = self.invoke(file_path="crate/Cargo.lock")
        lines = json.loads(text)["hookSpecificOutput"]["additionalContext"].splitlines()
        self.assertEqual(len(lines), 9)
        self.assertEqual(lines[-1], "… 4 more → dependabot-harvest.py --clusters")
        self.assertLess(elapsed, .3)

    def test_registration(self):
        settings = json.loads((ROOT / ".claude/settings.json").read_text())
        hooks = [(group["matcher"], h) for group in settings["hooks"]["PostToolUse"] for h in group["hooks"]
                 if "dependabot-inflight.py" in h.get("command", "")]
        self.assertEqual(len(hooks), 1)
        self.assertEqual(hooks[0][0], "Edit|Write|Bash")
        self.assertEqual(hooks[0][1]["timeout"], 3)


if __name__ == "__main__":
    unittest.main(verbosity=2)
