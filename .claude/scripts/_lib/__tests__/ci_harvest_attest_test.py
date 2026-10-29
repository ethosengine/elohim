"""S2d′: device attestations, independent observation retries, and bounded reach-back."""
import contextlib
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

ROOT = Path(__file__).resolve().parents[4]
spec = importlib.util.spec_from_file_location("ci_harvest", ROOT / ".claude/scripts/ci-harvest.py")
ci = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci)
SHA = "e67c6cef8a53" + "a" * 28
GRAPH = {"commitSha": SHA, "buildNumber": 2175, "results": {
    "elohim-edge": {"buildNumber": 1756, "result": "SUCCESS", "url": "https://jenkins/1756/", "duration": 10}}}


def completed(numbers=(2175,)):
    return {"completed": [(n, "SUCCESS") for n in numbers]}


def rendered(out, hook=False):
    return ci.render([], [], [], [], [], hook, waves=out)


class HarvestTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.cursor = {"jobs": {}}
        self.calls = []
        self.graph_reads = []
        self.offer = {"minted": "yes", "state": "active", "offer": "bafyreioffer"}
        self.attestation = {"commit": SHA, "written": [{"pipeline": "elohim-edge", "step": "ci:elohim-edge", "kind": "tree"}], "deferred": []}
        self.outcome = {"build": 1756, "basis": "graph", "process": "bafyprocess", "appended": 21, "present": 0}
        self.observe_exit = 0
        self.observe_stderr = ""
        self.graph = GRAPH
        for target, kwargs in [
            ("brit_build_ref_bin", {"return_value": "/stub/brit"}),
            ("jenkins_bridge_bin", {"return_value": "/stub/bridge"}),
            ("get_json", {"side_effect": self.get_graph}),
            ("_get_describe", {"return_value": {"id": "1756", "status": "SUCCESS", "stages": []}}),
            ("subprocess.run", {"side_effect": self.run_command}),
            ("CURSOR_PATH", {"new": str(Path(self.tmp.name) / "cursor.json")}),
        ]:
            p = patch.object(ci, target, **kwargs) if "." not in target else patch.object(ci.subprocess, "run", **kwargs)
            value = p.start()
            self.addCleanup(p.stop)
            if target == "_get_describe":
                self.describe = value

    def get_graph(self, path):
        self.graph_reads.append(path)
        return self.graph

    def run_command(self, args, **kwargs):
        self.calls.append(args)
        if args[0] == "node":
            self.assertEqual(args[1:3], [ci.BRIT_ATTEST, "--json"])
            self.assertEqual(json.loads(Path(args[3]).read_text()), self.graph)
            self.assertEqual(kwargs["env"]["REPO_ROOT"], ci.PROJECT)
            self.assertEqual(kwargs["env"]["BRIT_BUILD_REF_BIN"], "/stub/brit")
            return subprocess.CompletedProcess(args, 0, json.dumps(self.attestation), "")
        if args[1:3] == ["offer", "status"]:
            return subprocess.CompletedProcess(args, 0, json.dumps(self.offer), "")
        self.assertEqual(args[1], "observe")
        self.assertEqual(args[args.index("--offer") + 1], ci.OFFER_PATH)
        self.assertEqual(json.loads(Path(args[args.index("--graph") + 1]).read_text()), self.graph)
        return subprocess.CompletedProcess(args, self.observe_exit, json.dumps(self.outcome), self.observe_stderr)

    def count(self, word):
        return sum(word in call for call in self.calls)

    def test_candidates_oldest_first_capped(self):
        self.offer = {"minted": "no", "offer": "bafyreioffer"}
        self.cursor["attested"] = {"2173": SHA[:12]}
        result = ci.attest_waves(completed(range(2161, 2176)), self.cursor)
        self.assertEqual(result["attested"], list(range(2161, 2171)))
        self.assertEqual(len(self.graph_reads), 10)
        self.assertIn("4 wave(s) waiting", rendered(result))
        self.assertEqual(self.count("status"), 1)

    def test_tree_record_memoises_and_only_system_message_changes(self):
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.cursor["attested"]["2175"], SHA[:12])
        self.assertEqual(out["attested"], [2175])
        hook = json.loads(rendered(out, True))
        self.assertIn("attested 1 wave(s) (#2175…#2175)", hook["systemMessage"])
        self.assertEqual(hook["hookSpecificOutput"]["additionalContext"], "")

    def test_deferred_attestation_does_not_memoise(self):
        self.attestation["deferred"] = [{"why": f"commit {SHA} not in this clone"}]
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.cursor["attested"], {})
        self.assertIn(f"commit {SHA} not in this clone", rendered(out))

    def test_offer_gate_once_no_wfapi_unminted_or_proposed(self):
        for offer in [{"minted": "no", "offer": "bafyreioffer"},
                      {"minted": "yes", "state": "proposed", "offer": "bafyreioffer", "missing": "distinct Steward verdict"}]:
            with self.subTest(offer=offer):
                self.calls.clear()
                self.offer = offer
                out = ci.attest_waves(completed([2174, 2175]), {})
                self.assertEqual(self.count("status"), 1)
                self.assertEqual(self.count("node"), 2)
                self.describe.assert_not_called()
                self.assertEqual(self.count("observe"), 0)
                self.assertEqual(out["observed"], [])
                self.assertIn("observe deferred: offer bafyreioffer", rendered(out))
                if offer["minted"] == "no":
                    self.assertIn("unminted — jenkins-bridge offer mint", rendered(out))
                else:
                    self.assertIn(offer["missing"], rendered(out))

    def test_observe_once_memo_and_cleared_memo_replay_is_present(self):
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.cursor["observed"]["2175"], {"edge": 1756, "process": "bafyprocess", "appended": 21, "present": 0})
        self.assertIn("observed edge #1756 (+21/=0) bafyprocess", rendered(out))
        self.calls.clear()
        ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.count("observe"), 0)
        self.assertEqual(self.count("node"), 0)
        self.cursor["observed"] = {}
        self.outcome.update(appended=0, present=21)
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.count("observe"), 1)
        self.assertEqual(self.count("node"), 0)
        self.assertIn("+0/=21", rendered(out))

    def test_missing_edges_do_not_starve_later_observation_debt(self):
        self.cursor["attested"] = {str(n): SHA[:12] for n in range(2161, 2176)}
        def graph_at(path):
            n = int(path.split("/artifact/")[0].split("/")[-1])
            return {**GRAPH, "results": {}} if n < 2175 else GRAPH
        with patch.object(ci, "get_json", side_effect=graph_at):
            out = ci.attest_waves(completed(range(2161, 2176)), self.cursor)
        self.assertEqual(self.count("node"), 0)
        self.assertEqual(self.count("observe"), 1)
        self.assertEqual(out["observed"][0]["edge"], 1756)

    def test_unminted_then_active_retries_already_attested_wave(self):
        self.offer = {"minted": "no", "offer": "bafyreioffer"}
        ci.attest_waves(completed(), self.cursor)
        self.offer = {"minted": "yes", "state": "active"}
        self.calls.clear()
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.count("node"), 0)
        self.assertEqual(self.count("observe"), 1)
        self.assertEqual(len(out["observed"]), 1)

    def test_exit_65_retried_exit_3_and_74_verbatim(self):
        self.observe_exit = 65
        self.observe_stderr = "build is IN_PROGRESS"
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.cursor["observed"], {})
        self.assertNotIn("observe deferred: offer", rendered(out))
        self.observe_exit = 3
        self.observe_stderr = "the network has not projected recipe edge-pipeline@1 — run epr flow project first"
        out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.count("observe"), 2)
        self.assertIn(self.observe_stderr, out["deferred"])
        self.observe_exit = 74
        self.observe_stderr = "store unavailable"
        out = ci.attest_waves(completed(), self.cursor)
        self.assertIn("store unavailable", out["deferred"])
        self.assertEqual(self.cursor["observed"], {})

    def test_queued_or_missing_edge_build_observes_nothing(self):
        for edge in [{}, {"result": "DISPATCHED", "buildNumber": None}]:
            self.graph = {**GRAPH, "results": {"elohim-edge": edge}}
            out = ci.attest_waves(completed(), {})
            self.assertEqual(out["attested"], [2175])
        self.describe.assert_not_called()
        self.assertEqual(self.count("observe"), 0)

    def test_no_brit_hook_json_well_formed(self):
        job = {**completed(), "job": ci.ATTEST_JOB, "urgent": None, "last": 2175, "builds_seen": []}
        with patch.object(ci, "brit_build_ref_bin", return_value=None), \
             patch.object(ci, "harvest_job", return_value=job), \
             patch.object(ci, "load_cursor", return_value=self.cursor), \
             patch.object(ci, "load_taxonomy", return_value={}), \
             patch.object(ci, "reconcile", return_value=([], [], [], [])):
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                ci.run_harvest([ci.ATTEST_JOB], as_hook=True)
            hook = json.loads(output.getvalue())
            self.assertIn("brit-build-ref absent", hook["systemMessage"])
            self.assertEqual(self.count("node"), 0)

    def test_wait_removed(self):
        with patch.object(sys, "argv", ["ci-harvest.py", "--wait", "elohim"]), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit) as raised:
                ci.main()
        self.assertEqual(raised.exception.code, 2)
        self.assertFalse(hasattr(ci, "wait_mode"))

    def test_attest_reaches_past_memo_and_window(self):
        self.cursor.update(attested={"2150": SHA[:12]}, observed={"2150": self.outcome})
        with patch.object(sys, "argv", ["ci-harvest.py", "--attest", "2150"]), \
             patch.object(ci, "load_cursor", return_value=self.cursor), contextlib.redirect_stdout(io.StringIO()):
            ci.main()
        self.assertEqual(self.count("node"), 1)
        self.assertEqual(len(self.graph_reads), 1)
        self.assertIn("/2150/artifact/", self.graph_reads[0])

    def test_missing_bridge_defers_but_attests(self):
        with patch.object(ci, "jenkins_bridge_bin", return_value=None):
            out = ci.attest_waves(completed(), self.cursor)
        self.assertIn("observe deferred: jenkins-bridge not on PATH", rendered(out))
        self.assertIn("just gate jenkins-bridge", rendered(out))
        self.assertEqual(out["attested"], [2175])
        self.describe.assert_not_called()

    def test_unreadable_graph_not_memoised(self):
        self.graph = None
        out = ci.attest_waves(completed(), self.cursor)
        self.assertIn("#2175: graph unreadable", out["deferred"])
        self.assertEqual(self.cursor["attested"], {})

    def test_failed_node_and_budget_timeout_are_deferred(self):
        with patch.object(ci.subprocess, "run", side_effect=subprocess.TimeoutExpired("node", 60)):
            out = ci.attest_waves(completed(), self.cursor)
        self.assertEqual(self.cursor["attested"], {})
        self.assertTrue(out["deferred"])


class BinaryResolutionTests(unittest.TestCase):
    def test_stale_pool_is_not_used_fresh_pool_is(self):
        with tempfile.TemporaryDirectory() as tmp, patch.dict(os.environ, {"CARGO_POOL_ROOT": tmp}, clear=True), \
             patch.object(ci.shutil, "which", return_value=None), \
             patch.object(ci.subprocess, "run", return_value=subprocess.CompletedProcess([], 0, "200", "")) as git:
            path = Path(tmp) / "family/dev/jenkins-bridge/dev/debug/jenkins-bridge"
            path.parent.mkdir(parents=True)
            path.touch()
            os.utime(path, (100, 100))
            self.assertIsNone(ci.jenkins_bridge_bin())
            git.assert_called_once()
            os.utime(path, (201, 201))
            self.assertEqual(ci.jenkins_bridge_bin(), str(path))

    def test_environment_then_path(self):
        with patch.dict(os.environ, {"JENKINS_BRIDGE_BIN": "/explicit/bridge", "BRIT_BUILD_REF_BIN": "/explicit/brit"}), \
             patch.object(ci.shutil, "which", return_value="/path/tool"):
            self.assertEqual(ci.jenkins_bridge_bin(), "/explicit/bridge")
            self.assertEqual(ci.brit_build_ref_bin(), "/explicit/brit")
        with patch.dict(os.environ, {}, clear=True), patch.object(ci.shutil, "which", return_value="/path/tool"):
            self.assertEqual(ci.jenkins_bridge_bin(), "/path/tool")
            self.assertEqual(ci.brit_build_ref_bin(), "/path/tool")


if __name__ == "__main__":
    unittest.main(verbosity=2)
