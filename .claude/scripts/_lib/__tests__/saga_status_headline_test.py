#!/usr/bin/env python3
"""saga-status.py headline(): names its report, its age and the commits since, or says unmeasured."""
import importlib.util
import json
import subprocess
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location("saga_status", ROOT / ".claude" / "scripts" / "saga-status.py")
saga = importlib.util.module_from_spec(spec)
spec.loader.exec_module(saga)


class Ch:
    num, name = "01", "first"


def rows():
    return [{"chapter": Ch(), "state": "red"}]


def vcs(repo, *a):
    return subprocess.run(["git", "-C", str(repo), *a], check=True, capture_output=True, text=True).stdout.strip()


class Headline(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        repo = Path(self.tmp.name)
        vcs(repo, "init", "-q")
        for i in range(3):
            vcs(repo, "-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty", "-m", f"c{i}")
        self.first = vcs(repo, "rev-list", "--max-parents=0", "HEAD")
        self._root = saga.ROOT
        saga.ROOT = repo

    def tearDown(self):
        saga.ROOT = self._root
        self.tmp.cleanup()

    def test_full_provenance(self):
        rep = {"runId": "R1", "generatedAt": "2026-10-18T15:47:23.398Z", "gitCommit": self.first}
        now = datetime(2026, 11, 26, 16, 0, tzinfo=timezone.utc)
        self.assertEqual(saga.report_provenance(rep, now), "report R1, 39d old, 2 commits since (all paths)")
        line = saga.headline(rows(), rep, False)
        self.assertTrue(line.startswith("SAGA resiliency: 0/1 green · frontier: ch01 first (red)"), line)
        self.assertIn(" · report R1, ", line)
        self.assertIn("2 commits since (all paths)", line)

    def test_no_report_kept(self):
        self.assertTrue(saga.headline(rows(), None, False).endswith(" · (no report)"))

    def test_missing_commit_is_unmeasured(self):
        out = saga.report_provenance({"runId": "R", "generatedAt": "2026-11-01T00:00:00Z"})
        self.assertIn("commits since unmeasured (no gitCommit in report)", out)

    def test_absent_commit_is_unmeasured_not_zero(self):
        out = saga.report_provenance({"runId": "R", "generatedAt": "2026-11-01T00:00:00Z", "gitCommit": "e" * 40})
        self.assertIn("not in this checkout's object store", out)
        self.assertNotIn("0 commits", out)

    def test_bad_date_is_unmeasured(self):
        out = saga.report_provenance({"runId": "R", "generatedAt": "yesterday", "gitCommit": self.first})
        self.assertIn("age unmeasured (generatedAt unparseable: 'yesterday')", out)
        out = saga.report_provenance({"runId": "R", "gitCommit": self.first})
        self.assertIn("age unmeasured (no generatedAt)", out)

    def test_load_report_keeps_git_commit_and_raw_date(self):
        reports = Path(self.tmp.name) / "genesis" / "a2o" / "reports"
        reports.mkdir(parents=True)
        (reports / "sprint-report-household-X-1.json").write_text(json.dumps(
            {"runId": "X-1", "generatedAt": "not a date", "gitCommit": "abc", "summary": {}}))
        rep = saga.load_report()
        self.assertEqual(rep["gitCommit"], "abc")
        self.assertEqual(rep["generatedAt"], "not a date")


if __name__ == "__main__":
    unittest.main()
