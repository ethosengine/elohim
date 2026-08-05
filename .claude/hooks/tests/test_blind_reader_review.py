from __future__ import annotations

import json
import os
import subprocess
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / ".claude" / "hooks" / "blind-reader-review.py"


def run_hook(path: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["CLAUDE_PROJECT_DIR"] = str(REPO)
    return subprocess.run(
        ["python3", str(HOOK)],
        input=json.dumps({"tool_input": {"file_path": str(path)}}),
        text=True,
        capture_output=True,
        check=False,
        env=env,
    )


def context_for(path: Path) -> str:
    result = run_hook(path)
    if result.returncode != 0:
        raise AssertionError(result.stderr)
    return json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]


class BlindReaderReviewHookTest(unittest.TestCase):
    def test_a2o_feature_selects_story_profile(self) -> None:
        path = REPO / "genesis" / "a2o" / "features" / "content" / "epr-content-addressing.feature"
        context = context_for(path)

        self.assertIn("a2o-story-blind-reader-review", context)
        self.assertIn("`a2o-story`", context)
        self.assertIn("`blind-reader`", context)

    def test_manifesto_epic_selects_epic_profile(self) -> None:
        path = REPO / "genesis" / "docs" / "content" / "elohim-protocol" / "autonomous_entity" / "epic.md"
        context = context_for(path)

        self.assertIn("manifesto-epic-blind-reader-review", context)
        self.assertIn("`manifesto-epic`", context)

    def test_readme_selects_newcomer_profile(self) -> None:
        context = context_for(REPO / "README.md")

        self.assertIn("readme-blind-reader-review", context)
        self.assertIn("`readme`", context)

    def test_unmatched_markdown_write_is_silent(self) -> None:
        result = run_hook(REPO / "CLAUDE.md")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")

    def test_path_outside_repo_is_silent(self) -> None:
        result = run_hook(Path("/tmp/synthetic-blind-reader-test/README.md"))

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
