#!/usr/bin/env python3
"""Behavior and fail-open contract for the WARN-only command-shape hook."""
import json
from pathlib import Path
import subprocess
import sys
import unittest


HOOK = Path(__file__).resolve().parents[1] / "command-shape.py"


class CommandShapeHook(unittest.TestCase):
    def invoke(self, payload, runtime="claude", raw=False):
        event = payload if raw else json.dumps(payload)
        result = subprocess.run(
            [sys.executable, str(HOOK), runtime],
            input=event,
            text=True,
            capture_output=True,
            check=False,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout.strip()

    def context(self, command, tool="Bash", runtime="claude"):
        output = self.invoke({"tool_name": tool, "tool_input": {"command": command}}, runtime)
        if not output:
            return ""
        packet = json.loads(output)
        hook_output = packet["hookSpecificOutput"]
        self.assertEqual(hook_output["hookEventName"], "PreToolUse")
        self.assertNotIn("permissionDecision", hook_output)
        self.assertNotIn("permissionDecisionReason", hook_output)
        return hook_output["additionalContext"]

    def assert_warns(self, rule, command, **kwargs):
        self.assertIn(f"command-shape: {rule} —", self.context(command, **kwargs))

    def assert_passes(self, command, **kwargs):
        self.assertEqual(self.context(command, **kwargs), "")

    def test_one_operation_warns_for_mixed_read_and_mutation(self):
        self.assert_warns("one-operation", "rg TODO . && rm stale.txt")
        self.assert_warns("one-operation", "printf x > result.txt; ls result.txt")
        self.assert_passes("rg TODO . && ls")
        self.assert_passes("mkdir out && cp source out/file")
        self.assert_passes("printf x >/dev/null && ls")

    def test_mutation_table_covers_declared_commands(self):
        mutations = [
            "git add x", "git commit -m x", "git push", "git merge topic", "git rebase main",
            "git reset HEAD", "git checkout topic", "git switch topic", "git worktree add ../x",
            "git worktree remove ../x", "brit add x", "brit commit -m x", "brit push",
            "rm x", "mv a b", "cp a b", "mkdir x", "ln -s a b", "berth claim cargo",
            "berth release cargo", "berth say hello", "berth moor --session x",
            "epr flow note --on x", "epr flow intend --on x", "epr covenant record",
            "epr session open --model x",
        ]
        for command in mutations:
            with self.subTest(command=command):
                self.assert_warns("one-operation", f"{command} && pwd")

    def test_leading_comment_and_single_brit_fallback_exemption(self):
        self.assert_warns("leading-comment", "# why this runs\nls")
        self.assert_passes("# Brit fallback: installed Brit lacks this operation\ngit status")
        self.assert_warns(
            "leading-comment",
            "# Brit fallback: installed Brit lacks this operation\n# more context\ngit status",
        )
        self.assert_warns("leading-comment", "# Brit fallback:\ngit status")

    def test_brit_fallback_names_landing_replacements(self):
        cases = {
            "git cherry-pick abc": "genesis/agentic/bin/landing-pick",
            "git cherry-pick --continue": "genesis/agentic/bin/landing-pick",
            "git cherry-pick --abort": "genesis/agentic/bin/landing-pick",
            "git rebase --exec 'git commit --amend --no-edit' main":
                "genesis/agentic/bin/landing-retrailer",
            "git commit-tree HEAD^{tree}": "genesis/agentic/bin/landing-tagged-tip",
        }
        for command, script in cases.items():
            with self.subTest(command=command):
                context = self.context(f"# Brit fallback: Brit lacks this operation\n{command}")
                self.assertIn("command-shape: leading-comment —", context)
                self.assertIn(f"{script} -C <worktree>", context)

    def test_no_cd_warns_and_working_directory_option_passes(self):
        self.assert_warns("no-cd", "cd /project && ls")
        self.assert_warns("no-cd", "pushd /project; pwd")
        self.assert_passes("git -C /project status")

    def test_code_heredoc_warns_and_plain_text_passes(self):
        self.assert_warns("no-code-heredoc", "python3 - <<'EOF'\nprint('x')\nEOF")
        self.assert_warns("no-code-heredoc", "cat <<'EOF' > helper.ts\nexport {}\nEOF")
        self.assert_passes("git commit -F - <<'EOF'\nplain commit message\nEOF")
        self.assert_passes("cat <<'EOF' > body.json\n{}\nEOF")

    def test_wait_warns_for_self_match_and_unbounded_loop(self):
        self.assert_warns("bounded-wait", "pgrep -f worker; echo worker")
        # The shell running the command carries the pattern in its own command line, so any
        # pgrep/pkill -f matches it, alone or inside a substitution.
        self.assert_warns("bounded-wait", "pgrep -f worker")
        self.assert_warns("bounded-wait", "p=$(pgrep -f 'epr peer serve --root /x' | head -1)")
        self.assert_warns("bounded-wait", "pkill -af worker")
        self.assert_warns("bounded-wait", "while test ! -f done; do sleep 1; done")
        self.assert_warns("bounded-wait", "while true; do echo $SECONDS; sleep 1; done")
        self.assert_warns("bounded-wait", "while true; do date +%s; sleep 1; done")
        self.assert_passes("pgrep worker")
        self.assert_passes("pgrep -x worker")
        self.assert_passes("timeout 5 bash -c 'while kill -0 123; do sleep 1; done'")
        self.assert_passes("while (( tries < 5 )); do sleep 1; ((tries++)); done")
        self.assert_passes("while [ \"$SECONDS\" -lt 5 ]; do sleep 1; done")
        self.assert_passes("while [ \"$(date +%s)\" -lt \"$deadline\" ]; do sleep 1; done")

    def test_env_unset_warns_and_explicit_assignment_passes(self):
        self.assert_warns("no-env-u", "env -u RUSTFLAGS CARGO_TARGET_DIR=/tmp cargo test")
        self.assert_passes("env RUSTFLAGS= CARGO_TARGET_DIR=/tmp cargo test")

    def test_non_bash_event_is_silent(self):
        self.assertEqual(
            self.invoke({"tool_name": "Edit", "tool_input": {"command": "cd /tmp"}}), ""
        )

    def test_malformed_stdin_reports_coverage_gap_and_exits_zero(self):
        output = self.invoke("{not-json", raw=True)
        self.assertIn("command-shape: coverage-gap", json.loads(output)["hookSpecificOutput"]["additionalContext"])

    def test_codex_runtime_uses_same_additional_context_shape(self):
        self.assert_warns("no-cd", "cd /tmp", runtime="codex")

    def test_at_most_three_findings_are_emitted(self):
        context = self.context(
            "# explanation\nenv -u X cd /tmp && rm x; python3 <<'EOF'\nprint('x')\nEOF"
        )
        self.assertLessEqual(len(context.splitlines()), 3)


if __name__ == "__main__":
    unittest.main()
