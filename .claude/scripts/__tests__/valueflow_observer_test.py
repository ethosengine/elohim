#!/usr/bin/env python3
"""Subprocess integration test for the PostToolUse valueflow observer."""
from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / ".claude" / "hooks" / "valueflow-observer.py"
EPR = Path("/opt/rust/cargo/bin/epr")


@unittest.skipUnless(EPR.is_file(), "real /opt/rust/cargo/bin/epr is not installed")
class ValueflowObserverTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self._write(
            ".claude/epr-meta/recipes.yaml",
            """version: 1
recipes:
  - id: observer-fixture
    version: 1
    description: observer fixture
    stages:
      - name: plan
        artifactKind: "doc:plan"
        paths: ["plans/**/*.md"]
      - name: intent
        artifactKind: "gap:item"
        paths: ["gap-items/*.json"]
    edges: []
""",
        )
        self._write("plans/epic.md", "---\nid: fixture-epic\n---\n# Fixture epic\n")
        self._write(
            "gap-items/epic.json",
            json.dumps(
                {
                    "doc": "plans/epic.md",
                    "items": [
                        {"id": "epic#1", "state": "OPEN"},
                        {"id": "epic#2", "state": "OPEN"},
                    ],
                }
            ),
        )
        self._git("init", "-q")
        self._git("add", ".")
        self._git("commit", "-q", "-m", "observer fixture")
        projected = subprocess.run(
            [
                str(EPR),
                "flow",
                "project",
                "--root",
                str(self.root),
                "--recipes",
                str(self.root / ".claude/epr-meta/recipes.yaml"),
            ],
            cwd=self.root,
            capture_output=True,
            text=True,
            check=False,
        )
        self.assertEqual(projected.returncode, 0, projected.stdout + projected.stderr)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def _write(self, relative: str, text: str) -> Path:
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
        return path

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

    def _observe(self, path: Path) -> subprocess.CompletedProcess[str]:
        env = {
            **os.environ,
            "CLAUDE_PROJECT_DIR": str(self.root),
            "PATH": f"{EPR.parent}:{os.environ.get('PATH', '')}",
        }
        return subprocess.run(
            ["python3", str(HOOK)],
            cwd=self.root,
            env=env,
            input=json.dumps(
                {"tool_name": "Write", "tool_input": {"file_path": str(path)}}
            ),
            capture_output=True,
            text=True,
            check=False,
        )

    def _records(self) -> list[dict]:
        ledger = self.root / ".eprfs/status/flows.jsonl"
        return [json.loads(line)["record"] for line in ledger.read_text().splitlines()]

    def test_briefs_claim_and_reports_fulfill_or_observe(self) -> None:
        brief = self._write(
            "briefs/task-1-brief.md",
            "---\ngap: epic#1\nactor: agent:implementer@test-model\n---\nDo it.\n",
        )
        claimed = self._observe(brief)
        self.assertEqual(claimed.returncode, 0, claimed.stdout + claimed.stderr)
        self.assertIn('"appended":true', claimed.stdout.replace(" ", ""))

        report = self._write(
            "reports/task-1-report.md",
            "---\ngap: epic#1\nactor: agent:implementer@test-model\n"
            "status: DONE\ncommits: [abc123, def456]\n---\nGate green.\n",
        )
        fulfilled = self._observe(report)
        self.assertEqual(fulfilled.returncode, 0, fulfilled.stdout + fulfilled.stderr)
        records = self._records()
        report_slots = next(
            record["classifiedAs"]
            for record in records
            if "report:DONE" in record.get("classifiedAs", [])
        )
        self.assertIn("commit:abc123", report_slots)
        self.assertIn("commit:def456", report_slots)

        count = len(records)
        repeated = self._observe(report)
        self.assertEqual(repeated.returncode, 0, repeated.stdout + repeated.stderr)
        self.assertEqual(len(self._records()), count, "epr makes a repeated observation idempotent")

        blocked_brief = self._write(
            "briefs/task-2-brief.md",
            "---\ngap: epic#2\nactor: agent:implementer@test-model\n---\nTry it.\n",
        )
        self.assertEqual(self._observe(blocked_brief).returncode, 0)
        blocked_report = self._write(
            "reports/task-2-report.md",
            "---\ngap: epic#2\nactor: agent:implementer@test-model\n"
            "status: BLOCKED\n---\nWaiting for the fixture.\n",
        )
        observed = self._observe(blocked_report)
        self.assertEqual(observed.returncode, 0, observed.stdout + observed.stderr)
        direct = subprocess.run(
            [
                str(EPR),
                "flow",
                "note",
                "--on",
                "epic#2",
                "--kind",
                "observation",
                "--reason",
                "BLOCKED: Waiting for the fixture.",
                "--as",
                "agent:implementer@test-model",
                "--json",
            ],
            cwd=self.root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        self.assertEqual(observed.stdout, direct.stdout, "an epr refusal is printed verbatim")
        self.assertEqual(observed.returncode, 0, "the hook never blocks the completed write")

    def test_missing_frontmatter_names_the_convention_without_blocking(self) -> None:
        brief = self._write("briefs/task-3-brief.md", "No frontmatter.\n")
        result = self._observe(brief)
        self.assertEqual(result.returncode, 0)
        self.assertEqual(len(result.stdout.splitlines()), 1)
        self.assertIn("frontmatter requires gap and actor", result.stdout)


if __name__ == "__main__":
    unittest.main()
