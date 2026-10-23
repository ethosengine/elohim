"""Contract tests for the shared Claude/Codex edit adapter; no files are edited."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
from types import SimpleNamespace
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
SPEC = importlib.util.spec_from_file_location("rust_style_hook", ROOT / ".claude/hooks/rust-style.py")
HOOK = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(HOOK)


class Adapter(unittest.TestCase):
    def invoke(self, tool, data, code=0, failures=None, stderr=""):
        result = SimpleNamespace(returncode=code, stdout=json.dumps({"failures": failures or []}), stderr=stderr)
        with patch.object(HOOK.subprocess, "run", return_value=result) as native:
            answer = HOOK.inspect(ROOT, {"tool_name": tool, "tool_input": data})
        return answer, native

    def test_native_denial_reaches_both_write_names(self):
        for name in ("Write", "write_file"):
            result, native = self.invoke(name, {"file_path": "fixture.rs", "content": "#[allow(unused)] fn f() {}"}, 1, ["RS-GOV-04 new finding"])
            self.assertIn("RS-GOV-04", result["deny"])
            self.assertEqual(native.call_count, 1)

    def test_unavailable_is_explicitly_unmeasured(self):
        result, _ = self.invoke("Write", {"path": "fixture.rs", "content": ""}, 2, stderr="checker stale")
        self.assertIn("UNMEASURED", result["context"])
        self.assertNotIn("deny", result)

    def test_unrelated_tools_and_opaque_edits_make_no_clean_claim(self):
        result, native = self.invoke("Read", {"path": "README.md"})
        self.assertIsNone(result)
        self.assertFalse(native.called)

    def test_codex_freeform_events_surface_the_active_rule(self):
        for tool, source in (
            ("apply_patch", "*** Begin Patch\n*** Add File: src/lib.rs\n+fn f() {}\n*** End Patch"),
            ("functions.exec", 'await tools.exec_command({cmd: "cargo test"})'),
        ):
            result = HOOK.inspect(ROOT, {"tool_name": tool, "tool_input": source})
            self.assertIn("RS-GOV-04", result["context"])
            self.assertIn("not been certified clean", result["context"])
        result, native = self.invoke("apply_patch", {"path": "fixture.rs"})
        self.assertIn("not been measured", result["context"])
        self.assertFalse(native.called)

    def test_edit_reconstructs_the_complete_candidate(self):
        with tempfile.TemporaryDirectory(dir=ROOT / "genesis/orchestrator/rust-style") as tmp:
            path = Path(tmp) / "fixture.rs"
            path.write_text("pub fn old() {}\n")
            result, native = self.invoke("Edit", {"file_path": str(path), "old_string": "old", "new_string": "new"})
            self.assertIn("pass", result["context"])
            self.assertEqual(native.call_args.kwargs["input"], "pub fn new() {}\n")

    def test_registered_in_both_runtimes(self):
        for config in (".claude/settings.json", ".codex/hooks.json"):
            self.assertIn("rust-style.py", (ROOT / config).read_text())


class CiBaseline(unittest.TestCase):
    def test_first_multicommit_branch_uses_integration_and_missing_evidence_refuses(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            script = root / "genesis/orchestrator/rust-style-ci.sh"
            script.parent.mkdir(parents=True)
            script.write_text((ROOT / "genesis/orchestrator/rust-style-ci.sh").read_text())
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root, text=True).strip()
            git("init", "-q")
            def commit(source):
                (root / "fixture.rs").write_text(source)
                git("add", "fixture.rs")
                git("-c", "user.name=Style fixture", "-c", "user.email=style@example.invalid", "commit", "-qm", "fixture")
                return git("rev-parse", "HEAD")
            base = commit("pub fn clean() {}")
            git("update-ref", "refs/remotes/origin/dev", base)
            commit("#[allow(dead_code)] fn new_violation() {}")
            head = commit("#[allow(dead_code)] fn new_violation() {} // later edit")
            def selected(branch, previous=""):
                env = {**os.environ, "BRANCH_NAME": branch, "CHANGE_TARGET": "", "GIT_PREVIOUS_SUCCESSFUL_COMMIT": previous}
                return subprocess.run(["bash", str(script), "--print-base"], cwd=root, env=env, text=True, capture_output=True)
            first = selected("feature/style")
            self.assertEqual(first.returncode, 0, first.stderr)
            self.assertEqual(first.stdout.strip(), base)
            self.assertEqual(selected("dev").returncode, 2)
            self.assertEqual(selected("dev", base).stdout.strip(), base)
            self.assertEqual(selected("dev", head).stdout.strip(), git("rev-parse", "HEAD^"))
            git("update-ref", "-d", "refs/remotes/origin/dev")
            self.assertEqual(selected("feature/style").returncode, 2)


if __name__ == "__main__":
    unittest.main()
