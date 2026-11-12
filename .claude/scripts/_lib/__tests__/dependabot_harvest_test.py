#!/usr/bin/env python3
"""Offline tests. Run: python3 .claude/scripts/_lib/__tests__/dependabot_harvest_test.py"""
import contextlib
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SCRIPTS = HERE.parents[1]
ROOT = SCRIPTS.parents[1]
FIXTURE = HERE / "fixtures/dependabot-alerts-open-sample.json"
ALERTS = json.loads(FIXTURE.read_text())
sys.path.insert(0, str(SCRIPTS))
from _lib import dependabot_harvest as dh


def load_module(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


shell = load_module("dependabot_shell", SCRIPTS / "dependabot-harvest.py")
gate = load_module("delivery_gate", ROOT / ".claude/hooks/delivery-gate.py")
scoreboard = load_module("delivery_scoreboard", SCRIPTS / "delivery-scoreboard.py")


class CoreTests(unittest.TestCase):
    def test_normalize_real_fixture(self):
        rows = [dh.normalize(a) for a in ALERTS]
        self.assertEqual(len(rows), 4)
        self.assertEqual(rows[0], {
            "number": 1097, "severity": "low", "ecosystem": "npm", "package": "node-forge",
            "manifest_path": "pnpm-lock.yaml", "ghsa_id": ALERTS[0]["security_advisory"]["ghsa_id"],
            "cve_id": ALERTS[0]["security_advisory"]["cve_id"],
            "summary": ALERTS[0]["security_advisory"]["summary"],
            "vulnerable_range": "<= 1.4.2", "first_patched": "1.4.3",
            "created_at": "2026-11-11T11:17:13Z",
            "html_url": "https://github.com/ethosengine/elohim/security/dependabot/1097",
        })
        self.assertEqual([r["severity"] for r in rows], ["low", "medium", "medium", "medium"])
        alert = copy.deepcopy(ALERTS[0])
        alert["security_vulnerability"]["first_patched_version"] = None
        alert["security_advisory"]["cve_id"] = None
        self.assertIsNone(dh.normalize(alert)["first_patched"])
        self.assertIsNone(dh.normalize(alert)["cve_id"])

    def test_fingerprint_identity(self):
        n = dh.normalize(ALERTS[0])
        fp = dh.fingerprint(n)
        self.assertRegex(fp, r"^[0-9a-f]{12}$")
        self.assertEqual(fp, dh.fingerprint({**n, "number": 1, "severity": "critical", "summary": "new"}))
        for field in ("ghsa_id", "ecosystem", "package", "manifest_path"):
            self.assertNotEqual(fp, dh.fingerprint({**n, field: "other"}))

    def test_reconcile_lifecycle_and_no_mutation(self):
        raw = copy.deepcopy(ALERTS)
        new, still, closed = dh.reconcile([], raw, "t1")
        self.assertEqual((len(new), still, closed), (4, [], []))
        before = copy.deepcopy(new)
        new2, still, closed = dh.reconcile(new, raw[:2], "t2")
        self.assertEqual((new2, len(still), len(closed)), ([], 2, 2))
        self.assertTrue(all(e["first_seen"] == "t1" and e["last_seen"] == "t2" and e["seen"] == 2 for e in still))
        self.assertTrue(all(e["closed_at"] == "t2" and e["last_seen"] == "t1" for e in closed))
        reappeared, retained, closed2 = dh.reconcile(still, raw, "t3")
        self.assertEqual((len(reappeared), len(retained), closed2), (2, 2, []))
        self.assertTrue(all(e["first_seen"] == "t3" and e["seen"] == 1 for e in reappeared))
        self.assertEqual(new, before)
        self.assertEqual(raw, ALERTS)

    def test_renumbering_and_duplicates_are_one_observation(self):
        entries, _, _ = dh.reconcile([], ALERTS, "t1")
        alerts = copy.deepcopy(ALERTS)
        alerts[0]["number"] = 9000
        new, still, closed = dh.reconcile(entries, alerts + alerts, "t2")
        self.assertEqual((new, len(still), closed), ([], 4, []))
        self.assertTrue(all(e["seen"] == 2 for e in still))
        self.assertIn(9000, [e["number"] for e in still])

    def test_clusters_and_patch_order(self):
        n = dh.normalize(ALERTS[0])
        entries = [dict(n, first_patched="1.9.0", severity="critical"),
                   dict(n, first_patched="1.10.0-rc.1", severity="high", manifest_path="other.lock"),
                   dict(n, first_patched="1.10.0"), dict(n, first_patched=None),
                   dict(n, ecosystem="cargo", first_patched="0.3.0")]
        group, other = dh.clusters(entries)
        self.assertEqual(group["count"], 4)
        self.assertEqual(group["severities"], {"critical": 1, "high": 1, "medium": 0, "low": 2})
        self.assertEqual(group["manifests"], ["other.lock", "pnpm-lock.yaml"])
        self.assertEqual(group["first_patched"], "1.10.0")
        self.assertTrue(group["unpatchable"])
        self.assertEqual(other["ecosystem"], "cargo")
        self.assertFalse(other["unpatchable"])
        self.assertIsNone(dh.clusters([dict(n, first_patched=None)])[0]["first_patched"])

    def test_summary_and_pressure(self):
        entries = [dh.normalize(a) for a in ALERTS]
        self.assertEqual(dh.summary_line(entries),
                         "dependabot: 4 open (0 critical, 0 high) · 0 unpatchable · top: node-forge 4 → /deprecation-stasis")
        self.assertIn("0 open", dh.summary_line([]))
        entries[0].update(severity="critical", first_patched=None)
        self.assertIn("1 critical", dh.summary_line(entries))
        self.assertIn("1 unpatchable", dh.summary_line(entries))
        self.assertEqual(dh.pressure(entries)["weighted"], 14)

    def test_complete_paginated_response(self):
        for text in (json.dumps(ALERTS), json.dumps(ALERTS[:2]) + "\n" + json.dumps(ALERTS[2:]),
                     json.dumps([ALERTS[:2], ALERTS[2:]])):
            self.assertEqual(shell.parse_alerts(text), ALERTS)
        self.assertEqual(shell.parse_alerts("[]"), [])
        for text in ("", "{}", "null", '[{}]', json.dumps(ALERTS) + "\n[", '[null]'):
            with self.subTest(text=text[:30]), self.assertRaises((ValueError, KeyError)):
                shell.parse_alerts(text)


class ShellTests(unittest.TestCase):
    def setUp(self):
        # All scratch stays within this worktree, never in the real data directory.
        self.tmp = tempfile.TemporaryDirectory(prefix=".dependabot-test-", dir=HERE)
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.data = self.root / "data"
        self.env = {**os.environ, "PATH": str(self.root / "no-gh"), "PYTHONDONTWRITEBYTECODE": "1"}

    def run_cli(self, *args):
        return subprocess.run([sys.executable, str(SCRIPTS / "dependabot-harvest.py"),
                               "--data-dir", str(self.data), *map(str, args)],
                              env=self.env, text=True, capture_output=True, timeout=10)

    def read(self, suffix):
        path = self.data / f"dependabot-{suffix}"
        if suffix.endswith("jsonl"):
            return [json.loads(line) for line in path.read_text().splitlines()]
        return json.loads(path.read_text())

    def fixture(self, alerts):
        path = self.root / "alerts.json"
        path.write_text(json.dumps(alerts))
        return path

    def test_end_to_end_closure_reappearance_and_json(self):
        result = self.run_cli("--from-file", FIXTURE, "--json")
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        report = json.loads(result.stdout)
        self.assertEqual(report["cursor"]["opened"], 4)
        self.assertEqual(report["cursor"]["counts"]["open"], 4)
        initial = self.read("findings.jsonl")
        result = self.run_cli("--from-file", self.fixture(ALERTS[:1]))
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertEqual(len(self.read("findings.jsonl")), 1)
        self.assertEqual(len(self.read("closed.jsonl")), 3)
        self.assertEqual(self.read("cursor.json")["closed"], 3)
        live = self.read("findings.jsonl")[0]
        original = next(e for e in initial if e["fp"] == live["fp"])
        self.assertEqual(live["first_seen"], original["first_seen"])
        self.assertEqual(live["seen"], 2)
        self.assertTrue(all("closed_at" in row for row in self.read("closed.jsonl")))
        self.assertEqual(self.run_cli("--from-file", FIXTURE).returncode, 0)
        self.assertEqual(self.read("cursor.json")["opened"], 3)
        self.assertEqual(len(self.read("closed.jsonl")), 3)
        # A second closure of a reappeared alert is a distinct throughput event.
        self.assertEqual(self.run_cli("--from-file", self.fixture([])).returncode, 0)
        self.assertEqual(self.read("findings.jsonl"), [])
        self.assertEqual(len(self.read("closed.jsonl")), 7)
        self.assertEqual(self.run_cli("--from-file", self.fixture([])).returncode, 0)
        self.assertEqual(len(self.read("closed.jsonl")), 7)

    def test_cluster_table(self):
        result = self.run_cli("--from-file", FIXTURE, "--clusters")
        self.assertEqual(result.returncode, 0, result.stdout)
        self.assertIn("npm\tnode-forge\t4\t0\t0\t3\t1\t1.4.3\tno\tpnpm-lock.yaml", result.stdout)

    def test_hook_missing_gh_and_nonhook_error(self):
        result = self.run_cli("--hook", "--repo", "owner/repo")
        self.assertEqual(result.returncode, 0)
        self.assertEqual(result.stdout.strip(), "dependabot: unmeasured (gh not found)")
        self.assertEqual(result.stderr, "")
        self.assertFalse((self.data / "dependabot-findings.jsonl").exists())
        self.assertFalse((self.data / "dependabot-cursor.json").exists())
        self.assertEqual(self.run_cli("--repo", "owner/repo").returncode, 1)

    def test_hook_cache_then_stale_failure_preserves_evidence(self):
        self.assertEqual(self.run_cli("--from-file", FIXTURE).returncode, 0)
        cursor_path = self.data / "dependabot-cursor.json"
        before = cursor_path.read_bytes()
        live_before = (self.data / "dependabot-findings.jsonl").read_bytes()
        result = self.run_cli("--hook")  # PATH has no gh: must use cache
        self.assertEqual(result.returncode, 0)
        self.assertTrue(result.stdout.startswith("dependabot: 4 open"))
        self.assertEqual(cursor_path.read_bytes(), before)
        stale = self.read("cursor.json")
        stale["last_poll"] = "2000-01-01T00:00:00Z"
        cursor_path.write_text(json.dumps(stale))
        result = self.run_cli("--hook")
        self.assertEqual(result.returncode, 0)
        self.assertIn("unmeasured", result.stdout)
        self.assertEqual(self.read("cursor.json"), stale)
        self.assertEqual((self.data / "dependabot-findings.jsonl").read_bytes(), live_before)

    def test_malformed_snapshot_cannot_close_live_findings(self):
        self.run_cli("--from-file", FIXTURE)
        before = {p.name: p.read_bytes() for p in self.data.iterdir()}
        path = self.fixture([])
        for text in ("[]\n[", "{}", '[{}]'):
            path.write_text(text)
            result = self.run_cli("--hook", "--from-file", path)
            self.assertEqual(result.returncode, 0)
            self.assertIn("unmeasured", result.stdout)
            self.assertEqual({p.name: p.read_bytes() for p in self.data.iterdir()}, before)

    def test_api_command_pagination_and_errors_without_network(self):
        responses = [subprocess.CompletedProcess([], 0, json.dumps({"nameWithOwner": "owner/repo"}), ""),
                     subprocess.CompletedProcess([], 0, json.dumps(ALERTS[:2]) + json.dumps(ALERTS[2:]), "")]
        with patch.object(shell.subprocess, "run", side_effect=responses) as run, contextlib.redirect_stdout(io.StringIO()):
            self.assertEqual(shell.main(["--data-dir", str(self.data)]), 0)
        self.assertEqual(run.call_args_list[0].args[0], ["gh", "repo", "view", "--json", "nameWithOwner"])
        self.assertEqual(run.call_args_list[1].args[0], ["gh", "api", "-X", "GET",
            "repos/owner/repo/dependabot/alerts?state=open&per_page=100", "--paginate"])
        cursor = self.read("cursor.json")
        cursor["last_poll"] = "2000-01-01T00:00:00Z"
        (self.data / "dependabot-cursor.json").write_text(json.dumps(cursor))
        before = {p.name: p.read_bytes() for p in self.data.iterdir()}
        for response in (subprocess.CompletedProcess([], 1, "[]", "authentication failed\nnetwork unavailable"),
                         subprocess.TimeoutExpired("gh", 60)):
            output = io.StringIO()
            kwargs = {"side_effect": response} if isinstance(response, Exception) else {"return_value": response}
            with patch.object(shell.subprocess, "run", **kwargs), contextlib.redirect_stdout(output):
                self.assertEqual(shell.main(["--hook", "--repo", "owner/repo", "--data-dir", str(self.data)]), 0)
            self.assertEqual(len(output.getvalue().splitlines()), 1)
            self.assertIn("unmeasured", output.getvalue())
            self.assertEqual({p.name: p.read_bytes() for p in self.data.iterdir()}, before)

    def test_delivery_gate_and_scoreboard_keep_pressures_distinct(self):
        data = self.root / ".claude/data"
        data.mkdir(parents=True)
        entries = [dict(dh.normalize(ALERTS[0]), severity="critical")]
        (data / "dependabot-findings.jsonl").write_text(json.dumps(entries[0]) + "\n")
        (data / "deprecations.jsonl").write_text('{"status":"open"}\n{"status":"fixed"}\n')
        (data / "ci-findings.jsonl").write_text('{"status":"open"}\n' * 3)
        output = io.StringIO()
        with patch.object(gate, "PROJECT", str(self.root)), patch.object(gate, "claimed_unverified", return_value=0), contextlib.redirect_stdout(output):
            gate.main()
        context = json.loads(output.getvalue())["hookSpecificOutput"]["additionalContext"]
        self.assertIn("1 live deprecation/warning findings", context)
        self.assertIn("Highest-leverage if the session has room: 1 live Dependabot alerts (1 critical)", context)
        with patch.object(scoreboard, "PROJECT", str(self.root)), patch.object(scoreboard, "AS_JSON", False):
            text = " ".join(scoreboard.ledgers())
        self.assertIn("1 live Dependabot alerts (1 critical)", text)
        self.assertIn("1 live deprecation/warning findings", text)
        with patch.object(scoreboard, "PROJECT", str(self.root)), patch.object(scoreboard, "AS_JSON", True):
            report = scoreboard.ledgers()
        self.assertEqual(report["dep_live"], 1)
        self.assertEqual(report["dependabot"]["weighted"], 8)

    def test_hook_registration(self):
        settings = json.loads((ROOT / ".claude/settings.json").read_text())
        hooks = [h for group in settings["hooks"]["SessionStart"] for h in group["hooks"]
                 if "dependabot-harvest.py" in h.get("command", "")]
        self.assertEqual(len(hooks), 1)
        self.assertTrue(hooks[0]["async"])
        self.assertEqual(hooks[0]["timeout"], 90)
        self.assertEqual(hooks[0]["statusMessage"], "Harvesting Dependabot alerts...")


if __name__ == "__main__":
    unittest.main(verbosity=2)
