"""The registrations of native-seam-guidance name the tools that write files, in every place they are declared.

The native evaluator (epr-cli seam/adapters.rs normalize()) supports claude Write/Edit and codex
apply_patch. Claude MultiEdit and NotebookEdit write files without an adapter and must keep
announcing an honest coverage gap rather than going unobserved. Read-only tools stay silent.

All three declarations are asserted here: .claude/settings.json, .codex/hooks.json and the hook
package. The gate that runs .claude/hooks/__tests__ declares .claude/settings.json as an input;
it does not declare .codex/hooks.json or the package JSON, so a change to only those two does not
select this test by itself (package-projections verify still reconciles the package against the
live configuration). The assertions are kept out of elohim/sdk/domains/elohim-agent/tests because
that path is an Edge build input, and a test-only change there would select a fleet build.
"""
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
CLAUDE_MATCHER = "Write|Edit|MultiEdit|NotebookEdit"
CODEX_MATCHER = "apply_patch"


def registered_matchers(path):
    groups = json.loads((ROOT / path).read_text())["hooks"]["PreToolUse"]
    return [g["matcher"] for g in groups
            if any("native-seam-guidance.py" in h["command"] for h in g["hooks"])]


class SeamGuidanceMatcher(unittest.TestCase):
    def test_claude_registration_matcher(self):
        self.assertEqual(registered_matchers(".claude/settings.json"), [CLAUDE_MATCHER])

    def test_codex_registration_matcher(self):
        self.assertEqual(registered_matchers(".codex/hooks.json"), [CODEX_MATCHER])

    def test_package_declares_the_same_matchers_and_an_exit(self):
        package = json.loads(
            (ROOT / ".epr-meta/elohim/packages/hooks/native-seam-guidance.json").read_text())
        self.assertEqual(package["registrations"]["claude"]["matcher"], CLAUDE_MATCHER)
        self.assertEqual(package["registrations"]["codex"]["matcher"], CODEX_MATCHER)
        self.assertIn("RETIRE_WHEN", package["source"]["body"])


if __name__ == "__main__":
    unittest.main()
