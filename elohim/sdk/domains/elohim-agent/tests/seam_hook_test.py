"""Privacy and degradation of the package-owned seam adapter, without remote services."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[5]
spec = importlib.util.spec_from_file_location("seam_hook", ROOT / ".claude/hooks/native-seam-guidance.py")
hook = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)
FAKE_EPR = "/nonexistent/epr-for-test"


class HookContract(unittest.TestCase):
    def run_hook(self, result=None, error=None, body=None, binary=FAKE_EPR):
        event = body or {"session_id": "private", "tool_name": "Write", "tool_input": {"file_path": "private.ts", "content": "private draft"}}
        stdout = io.StringIO()
        stdin = io.TextIOWrapper(io.BytesIO(json.dumps(event).encode()))
        with patch.object(sys, "stdin", stdin), patch.object(sys, "argv", ["hook", "claude"]), \
                patch.object(hook.epr_client, "resolve_binary", return_value=binary), \
                patch.object(hook.subprocess, "run", return_value=result, side_effect=error) as run, \
                contextlib.redirect_stdout(stdout):
            self.assertEqual(hook.main(), 0)
        return stdout.getvalue(), run

    def test_calls_only_native_process_with_private_stdin_and_deadline(self):
        out, run = self.run_hook(subprocess.CompletedProcess([], 0, '{"packet":"review this"}', ""))
        self.assertEqual(run.call_count, 1)
        args, kwargs = run.call_args
        self.assertEqual(args[0][0], FAKE_EPR)
        self.assertEqual(args[0][1:4], ["govern", "seam", "--repo"])
        self.assertNotIn("private draft", " ".join(args[0]))
        self.assertIn("private draft", kwargs["input"])
        self.assertLessEqual(kwargs["timeout"], 1.8)
        self.assertNotIn("shell", kwargs)
        output = json.loads(out)
        self.assertEqual(output["hookSpecificOutput"]["additionalContext"], "review this")
        self.assertNotIn("permissionDecision", out)
        self.assertNotIn("updatedInput", out)

    def test_missing_launcher_is_a_visible_gap_without_a_process(self):
        out, run = self.run_hook(binary=None)
        self.assertEqual(run.call_count, 0)
        self.assertIn("coverage gap", out)
        self.assertNotIn("private draft", out)

    def test_failures_are_visible_advisory_and_do_not_leak_process_output(self):
        for error in [FileNotFoundError(), subprocess.TimeoutExpired("epr", 1.8), subprocess.CalledProcessError(2, "epr", output="private draft")]:
            out, _ = self.run_hook(error=error)
            self.assertIn("coverage gap", out)
            self.assertNotIn("private draft", out)
            self.assertNotIn("permissionDecision", out)

    def test_invalid_or_oversized_native_output_is_named(self):
        for output in ["invalid", "{}", '{"packet":null}', json.dumps({"packet": "א" * 8192})]:
            out, _ = self.run_hook(subprocess.CompletedProcess([], 0, output, ""))
            self.assertIn("coverage gap", out)

    def test_clean_validated_and_not_enabled_reports_are_silent(self):
        for output in ['{"packet":""}', '{"advisoryOnly":true,"enabled":false,"candidates":[],"coverageGaps":[],"packet":""}']:
            out, _ = self.run_hook(subprocess.CompletedProcess([], 0, output, ""))
            self.assertEqual(out, "")

    def test_projected_adapters_share_exact_bytes(self):
        self.assertEqual((ROOT / ".claude/hooks/native-seam-guidance.py").read_bytes(), (ROOT / ".codex/hooks/native-seam-guidance.py").read_bytes())


if __name__ == "__main__":
    unittest.main()
