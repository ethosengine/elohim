#!/usr/bin/env python3
"""Tests for the harvester-blind self-check (runtime-harvest.py's `harvester_blind_finding`).

Grounding: genesis/data/timeline/backlog/
runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11.md — the observed failure was
`.claude/data/runtime-cursor.json` reading `{poll_index: 66, windows: {}}`: the poller HAD been
invoked 66 times (poll_index increments unconditionally in `harvest()`) but never once stored a
sample for any watched node, so `rh.evaluate`/`p2p_status_findings` never even ran. Each individual
node-unreachable is an honest per-poll absence (D3, degrade-quiet); 66 of them in a row with zero
samples stored is a DIFFERENT fact — the harvester itself is blind — and this predicate is the
layer that names that fact as its own finding, on the SAME ledger, through the SAME
fingerprint/reconcile core, exactly the way _lib/residual_channel.py's C14 channel binds rather
than forks (proven there by residual_channel_test.py; mirrored here).

Run: python3 .claude/scripts/_lib/__tests__/harvester_blind_test.py  (exit 0 = pass).
pytest is NOT installed — this is the bespoke harness, same shape as runtime_harvest_test.py.
"""
import http.server
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

here = Path(__file__).resolve()
_root = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        _root = here
        break
    here = here.parent
assert _root, "repo root not found"

from _lib import runtime_harvest as rh  # noqa: E402

# runtime-harvest.py is a hyphenated filename (not import-able as a package member) — load it the
# same way ci_harvest_echo_test.py loads ci-harvest.py.
_spec = importlib.util.spec_from_file_location(
    "runtime_harvest_shell", _root / ".claude" / "scripts" / "runtime-harvest.py"
)
shell = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(shell)

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── pure predicate: harvester_blind_finding(cursor, nodes) ──
check("class constant matches the ledger vocabulary", shell.HARVESTER_BLIND_CLASS == "harvester-blind")
check("min-polls threshold is 3 per the backlog atom's Done-when", shell.HARVESTER_BLIND_MIN_POLLS == 3)

below_threshold = {"poll_index": shell.HARVESTER_BLIND_MIN_POLLS - 1, "windows": {}}
check("silent below the poll threshold even with empty windows",
      shell.harvester_blind_finding(below_threshold, ["alpha", "alpha-b"]) is None)

at_threshold_blind = {"poll_index": shell.HARVESTER_BLIND_MIN_POLLS, "windows": {}}
f = shell.harvester_blind_finding(at_threshold_blind, ["alpha", "alpha-b"])
check("fires at the threshold when every watched node's window is empty", f is not None)
check("finding node is 'harvester', not a peer node", f["node"] == "harvester")
check("finding class is harvester-blind", f["class"] == shell.HARVESTER_BLIND_CLASS)
check("line names the atom", "runtime-sensing-gap-poller-unscheduled-no-throttle-alert-2026-09-11.md"
      in f["line"])

past_threshold_still_blind = {"poll_index": 66, "windows": {}}
check("still fires well past the threshold (the observed 66-poll case)",
      shell.harvester_blind_finding(past_threshold_still_blind, ["alpha", "alpha-b"]) is not None)

present_for_one_node = {"poll_index": 10, "windows": {"alpha": [{"health": True}]}}
check("silent once ANY watched node has a stored sample",
      shell.harvester_blind_finding(present_for_one_node, ["alpha", "alpha-b"]) is None)

empty_list_still_counts_as_blind = {"poll_index": 10, "windows": {"alpha": []}}
check("an explicit empty-list window still counts as blind (no samples, not merely no key)",
      shell.harvester_blind_finding(empty_list_still_counts_as_blind, ["alpha"]) is not None)

unwatched_node_has_samples = {"poll_index": 10, "windows": {"someone-else": [{"health": True}]}}
check("a sample for a node we are NOT currently watching does not clear the check",
      shell.harvester_blind_finding(unwatched_node_has_samples, ["alpha", "alpha-b"]) is not None)

# ── fingerprint stability (same contract runtime_harvest_test.py asserts for other classes) ──
fp_a = rh.fingerprint("harvester", shell.HARVESTER_BLIND_CLASS, "cursor:all-windows-empty")
fp_b = rh.fingerprint("harvester", shell.HARVESTER_BLIND_CLASS, "cursor:all-windows-empty")
check("fingerprint is stable across identical (node, class, provenance)", fp_a == fp_b)
check("fingerprint is invariant to poll_index (the finding LINE, not the provenance, carries the "
      "count)", fp_a == rh.fingerprint("harvester", shell.HARVESTER_BLIND_CLASS,
                                        "cursor:all-windows-empty"))

# ── end-to-end (Task-mirrors-Task-6 shape): 3 blind polls against an unreachable node -> live
#    ledger entry; a healthy responder never produces one at all ──
_script = str(_root / ".claude" / "scripts" / "runtime-harvest.py")

_tmp_blind = tempfile.mkdtemp()
_env_blind = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp_blind)
os.makedirs(os.path.join(_tmp_blind, ".claude", "data"), exist_ok=True)
for _ in range(shell.HARVESTER_BLIND_MIN_POLLS):
    subprocess.run(["python3", _script, "--nodes", "ghost", "--base", "http://127.0.0.1:9"],
                    env=_env_blind, capture_output=True, text=True, timeout=60)
_ledger_blind_path = os.path.join(_tmp_blind, ".claude", "data", "runtime-findings.jsonl")
_ledger_blind = ([json.loads(x) for x in open(_ledger_blind_path)]
                  if os.path.exists(_ledger_blind_path) else [])
check("empty windows after 3 polls -> a harvester-blind finding is present on the live ledger",
      any(e["class"] == "harvester-blind" for e in _ledger_blind))
_cursor_blind = json.load(open(os.path.join(_tmp_blind, ".claude", "data", "runtime-cursor.json")))
check("the cursor backing that finding really is empty (proves the fixture, not the assertion)",
      _cursor_blind["windows"] == {})

_healthy_body = json.dumps({"ok": True})


class _Healthy(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.end_headers()
        self.wfile.write(_healthy_body.encode())

    def log_message(self, *a):
        pass


_srv = http.server.HTTPServer(("127.0.0.1", 0), _Healthy)
threading.Thread(target=_srv.serve_forever, daemon=True).start()
_port = _srv.server_address[1]

_tmp_healthy = tempfile.mkdtemp()
_env_healthy = dict(os.environ, CLAUDE_PROJECT_DIR=_tmp_healthy)
os.makedirs(os.path.join(_tmp_healthy, ".claude", "data"), exist_ok=True)
for _ in range(shell.HARVESTER_BLIND_MIN_POLLS + 1):
    subprocess.run(["python3", _script, "--nodes", "t", "--base", f"http://127.0.0.1:{_port}"],
                    env=_env_healthy, capture_output=True, text=True, timeout=60)
_ledger_healthy_path = os.path.join(_tmp_healthy, ".claude", "data", "runtime-findings.jsonl")
_ledger_healthy = ([json.loads(x) for x in open(_ledger_healthy_path)]
                    if os.path.exists(_ledger_healthy_path) else [])
check("samples present every poll -> no harvester-blind finding ever files",
      not any(e["class"] == "harvester-blind" for e in _ledger_healthy))
_srv.shutdown()

print(f"\n  {_p} assertions passed ✅")
