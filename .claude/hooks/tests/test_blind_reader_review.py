from __future__ import annotations

import json
import os
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / ".claude" / "hooks" / "blind-reader-review.py"
SCRIPTS_DIR = REPO / ".claude" / "scripts"

# Synthetic manifest exercising the same `route-to: blind-reader` selection the real corpus
# manifests use (genesis/a2o/.epr-meta, genesis/docs/content/elohim-protocol/.epr-meta,
# .epr-meta/manifest.md) — three profiles keyed off basename globs, so this suite tests the
# HOOK's wiring rather than the CONTENT of any particular live document. Renaming/moving those
# real files, or adding an unrelated `route-to: blind-reader` rule elsewhere, cannot break this
# file; only a change to the selection logic itself (_matching_reviews / _matches_when) can.
FIXTURE_MANIFEST = """
    ---
    epr-meta-version: 1
    root: true
    rules:
      - id: fixture-story-profile
        class: inject
        when: { write: "*.feature" }
        route-to: { dest: blind-reader }
        parameters: { review-profile: fixture-story }
      - id: fixture-epic-profile
        class: inject
        when: { write: "epic*.md" }
        route-to: { dest: blind-reader }
        parameters: { review-profile: fixture-epic }
      - id: fixture-newcomer-profile
        class: inject
        when: { write: "README*.md" }
        route-to: { dest: blind-reader }
        parameters: { review-profile: fixture-newcomer }
    ---
"""


def _write(path: Path, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(textwrap.dedent(body).lstrip())


def run_hook(path: Path, *, project_dir: Path) -> subprocess.CompletedProcess[str]:
    env = os.environ.copy()
    env["CLAUDE_PROJECT_DIR"] = str(project_dir)
    # The hook derives its `_lib.epr_meta` import path from CLAUDE_PROJECT_DIR. For a synthetic
    # fixture root (no .claude/scripts of its own) that derived path doesn't exist; PYTHONPATH
    # keeps the real resolver importable without coupling the fixture to repo content.
    env["PYTHONPATH"] = str(SCRIPTS_DIR)
    return subprocess.run(
        ["python3", str(HOOK)],
        input=json.dumps({"tool_input": {"file_path": str(path)}}),
        text=True,
        capture_output=True,
        check=False,
        env=env,
    )


def context_for(path: Path, *, project_dir: Path) -> str:
    result = run_hook(path, project_dir=project_dir)
    if result.returncode != 0:
        raise AssertionError(result.stderr)
    return json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]


class BlindReaderReviewHookTest(unittest.TestCase):
    def setUp(self) -> None:
        self._tmpdir = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmpdir.cleanup)
        self.root = Path(self._tmpdir.name).resolve()
        (self.root / ".git").mkdir()
        _write(self.root / ".epr-meta", FIXTURE_MANIFEST)

    def test_a2o_feature_selects_story_profile(self) -> None:
        path = self.root / "fixture-story.feature"
        _write(path, "Feature: fixture\n  Scenario: fixture\n    Given a fixture\n")
        context = context_for(path, project_dir=self.root)

        self.assertIn("fixture-story-profile", context)
        self.assertIn("`fixture-story`", context)
        self.assertIn("`blind-reader`", context)

    def test_manifesto_epic_selects_epic_profile(self) -> None:
        path = self.root / "epic-fixture.md"
        _write(path, "# Fixture Epic\n")
        context = context_for(path, project_dir=self.root)

        self.assertIn("fixture-epic-profile", context)
        self.assertIn("`fixture-epic`", context)

    def test_readme_selects_newcomer_profile(self) -> None:
        path = self.root / "README.md"
        _write(path, "# Fixture README\n")
        context = context_for(path, project_dir=self.root)

        self.assertIn("fixture-newcomer-profile", context)
        self.assertIn("`fixture-newcomer`", context)

    def test_unmatched_markdown_write_is_silent(self) -> None:
        path = self.root / "NOTES.md"
        _write(path, "# Fixture notes matching no blind-reader rule\n")
        result = run_hook(path, project_dir=self.root)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")

    def test_path_outside_repo_is_silent(self) -> None:
        result = run_hook(
            Path("/tmp/synthetic-blind-reader-test/README.md"), project_dir=self.root
        )

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
