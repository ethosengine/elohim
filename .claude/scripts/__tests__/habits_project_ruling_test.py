#!/usr/bin/env python3
"""Fixture-backed checks for the habit-status ruling gate."""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
SCRIPT = REPO / ".claude" / "scripts" / "habits-project.py"
SPEC = importlib.util.spec_from_file_location("habits_project", SCRIPT)
assert SPEC and SPEC.loader
habits_project = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(habits_project)


class HabitFlipRulingTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.atom = self.root / ".epr-meta" / "fixture-habit.habit.md"
        covenant = self.root / ".epr-meta" / "habits-covenant.md"
        covenant.parent.mkdir(parents=True)
        covenant.write_text(
            "---\nversion: 1\nupdated: 2026-09-04\nvision: fixture\n"
            "order: [fixture-habit]\n---\nFixture covenant.\n",
            encoding="utf-8",
        )
        self._write_atom("green")
        (self.root / "genesis" / "manifests").mkdir(parents=True)
        projection, errors = habits_project.render(self.root)
        self.assertEqual(errors, [])
        (self.root / "genesis" / "manifests" / "habits.yaml").write_text(
            projection, encoding="utf-8"
        )
        self._git("init", "-q")
        self._git("add", ".epr-meta", "genesis/manifests/habits.yaml")
        self._git("commit", "-q", "-m", "fixture baseline")

        self._write_atom("red")
        projection, errors = habits_project.render(self.root)
        self.assertEqual(errors, [])
        (self.root / "genesis" / "manifests" / "habits.yaml").write_text(
            projection, encoding="utf-8"
        )

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def _git(self, *args: str) -> None:
        env = {
            **os.environ,
            "GIT_AUTHOR_NAME": "Fixture Author",
            "GIT_AUTHOR_EMAIL": "fixture@example.test",
            "GIT_COMMITTER_NAME": "Fixture Author",
            "GIT_COMMITTER_EMAIL": "fixture@example.test",
            "GIT_AUTHOR_DATE": "2026-09-04T12:00:00Z",
            "GIT_COMMITTER_DATE": "2026-09-04T12:00:00Z",
        }
        subprocess.run(["git", *args], cwd=self.root, env=env, check=True)

    def _write_atom(self, status: str) -> None:
        self.atom.write_text(
            f"---\nepr-habit-version: 1\nid: fixture-habit\n"
            f"invariant: The fixture holds.\nstatus: {status}\nactive: false\n"
            "checks: [python3 fixture_check.py]\n"
            "retire-when: 'never: this is a permanent fixture floor'\n---\nEvidence.\n",
            encoding="utf-8",
        )

    def _check(self) -> tuple[int, str]:
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            exit_code = habits_project.main(self.root, ["--check"])
        return exit_code, output.getvalue()

    def _write_ruling(self, occurred_at: str, label: str | None = None) -> None:
        ledger = self.root / ".eprfs" / "status" / "flows.jsonl"
        ledger.parent.mkdir(parents=True, exist_ok=True)
        record = {
            "cid": "fixture-note-cid",
            "record": {
                "kind": "event",
                "action": "cite",
                "resource": "fixture-body-cid",
                "classifiedAs": [
                    "run:ruling",
                    label or ".epr-meta/fixture-habit.habit.md",
                    "reason:fixture evidence",
                ],
                "occurredAt": occurred_at,
            },
        }
        ledger.write_text(json.dumps(record) + "\n", encoding="utf-8")

    def test_flip_requires_exact_label_and_post_commit_ruling(self) -> None:
        exit_code, output = self._check()
        self.assertEqual(exit_code, 1)
        self.assertIn(
            "FLIP-WITHOUT-RULING fixture-habit green->red: record it with "
            "epr flow note --on .epr-meta/fixture-habit.habit.md --kind ruling",
            output,
        )

        self._write_ruling("2026-09-04T11:59:59Z")
        self.assertIn("FLIP-WITHOUT-RULING", self._check()[1])

        self._write_ruling("2026-09-04T13:00:00Z", "some/other/atom.habit.md")
        self.assertIn("FLIP-WITHOUT-RULING", self._check()[1])

        self._write_ruling("2026-09-04T13:00:00Z")
        exit_code, output = self._check()
        self.assertEqual(exit_code, 0, output)
        self.assertIn("genesis/manifests/habits.yaml is current", output)


if __name__ == "__main__":
    unittest.main()
