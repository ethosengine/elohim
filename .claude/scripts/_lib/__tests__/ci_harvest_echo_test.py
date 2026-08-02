"""ci-harvest console-scan precision test — the OVER-CAPTURE class.

Two grounded instances of ONE shape: a taxonomy token with no error context
fingerprints the benign output of a SUCCEEDING step (museum trap #14).
  (a) `set -x` command echoes      — edge #1137, INFRASTRUCTURE/`nerdctl`
  (b) rollout progress narration   — edge #1195–#1293, DEPLOYMENT/`rollout`

Run: python3 .claude/scripts/_lib/__tests__/ci_harvest_echo_test.py  (exit 0 = pass)

Grounding (edge #1137, fingerprints e4cad4b435b1/b9ee178c936d/310cd811389a/
b0d29582b52f): the INFRASTRUCTURE taxonomy's bare `nerdctl` token matched four
`+ nerdctl -n k8s.io rmi …` bash-trace ECHOES of a cleanup step whose commands
all SUCCEEDED, burning the whole MAX_CONSOLE_FINDINGS_PER_BUILD budget on
non-failures while the one real rmi error in the same tail went uncaptured.
Two-part fix under test: (1) step-2 scan skips `set -x` command-echo lines
wholesale (fixes the class across every category); (2) the INFRASTRUCTURE
regex requires error context around tool names.
"""
import importlib.util
import json
import re
import sys
from pathlib import Path

here = Path(__file__).resolve()
root = None
for _ in range(8):
    if (here / ".claude" / "scripts").is_dir():
        root = here
        break
    here = here.parent
assert root, "repo root not found"

spec = importlib.util.spec_from_file_location(
    "ci_harvest", root / ".claude" / "scripts" / "ci-harvest.py"
)
ci_harvest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci_harvest)

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1


ECHO_SUCCESS = '+ nerdctl -n k8s.io rmi elohim-storage:1.0.0-dev-c1fd38c7'
ECHO_INDENTED = '  + nerdctl -n k8s.io rmi harbor.ethosengine.com/ethosengine/elohim-doorway:1.0.0-dev-x'
REAL_FAILURE = 'time="2026-07-02" level=fatal msg="nerdctl: no such image: elohim-agent-sdk:1.0.0-dev-x"'
REAL_OUTPUT = 'Deleted: sha256:abcd1234'

# (1) The scanner exposes a command-echo guard and it classifies correctly.
check("has _CMD_ECHO guard", hasattr(ci_harvest, "_CMD_ECHO"))
check("echo line skipped", ci_harvest._CMD_ECHO.match(ECHO_SUCCESS))
check("indented echo skipped", ci_harvest._CMD_ECHO.match(ECHO_INDENTED))
check("real failure NOT an echo", not ci_harvest._CMD_ECHO.match(REAL_FAILURE))
check("real output NOT an echo", not ci_harvest._CMD_ECHO.match(REAL_OUTPUT))

# (2) The INFRASTRUCTURE taxonomy regex needs error context, not a bare tool name.
tax = json.loads((root / ".claude" / "data" / "failure-taxonomy.json").read_text())
infra = tax["categories"]["INFRASTRUCTURE"]
rx = re.compile(infra["search"])
check("regex ignores succeeding echo", not rx.search(ECHO_SUCCESS))
check("regex catches real nerdctl failure", rx.search(REAL_FAILURE))
check("regex still catches denied", rx.search("pull access denied for harbor.x/y"))

# ---------------------------------------------------------------- instance (b)
# `kubectl rollout status` progress lines from rollouts that SUCCEEDED. Not
# `set -x` echoes (they are step output), so _CMD_ECHO cannot catch them.
PROG_NEW = 'Waiting for deployment "elohim-doorway-alpha-b" rollout to finish: 0 out of 1 new replicas have been updated...'
PROG_OLD = 'Waiting for deployment "elohim-doorway-alpha" rollout to finish: 1 old replicas are pending termination...'
PROG_SPEC = "Waiting for deployment spec update to be observed..."
ROLLED_OK = 'deployment "elohim-doorway-alpha-b" successfully rolled out'
# Real rollout failures — kubectl emits these as SEPARATE, non-progress lines.
ROLL_TIMEOUT = "error: timed out waiting for the condition"
ROLL_DEADLINE = 'error: deployment "elohim-doorway-alpha" exceeded its progress deadline'
CRASHLOOP = "  elohim-doorway-alpha-b-7d9  0/1  CrashLoopBackOff  5  3m"

# The echo guard alone is NOT enough here — that is why the class needed a
# second guard rather than a wider _CMD_ECHO.
for lbl, ln in (("new-replica", PROG_NEW), ("old-replica", PROG_OLD), ("spec-update", PROG_SPEC)):
    check(f"{lbl} progress is not a set -x echo", not ci_harvest._CMD_ECHO.match(ln))

check("has _BENIGN_PROGRESS guard", hasattr(ci_harvest, "_BENIGN_PROGRESS"))
bp = ci_harvest._BENIGN_PROGRESS
check("new-replica progress skipped", bp.match(PROG_NEW))
check("old-replica progress skipped", bp.match(PROG_OLD))
check("spec-update progress skipped", bp.match(PROG_SPEC))
check("successful rollout skipped", bp.match(ROLLED_OK))
# The guard must NEVER swallow a genuine rollout failure.
check("timeout error not benign", not bp.match(ROLL_TIMEOUT))
check("progress-deadline error not benign", not bp.match(ROLL_DEADLINE))
check("CrashLoopBackOff not benign", not bp.match(CRASHLOOP))

# The DEPLOYMENT taxonomy regex itself needs error context, not a bare `rollout`.
deploy = tax["categories"]["DEPLOYMENT"]
drx = re.compile(deploy["search"])
check("deploy regex ignores new-replica progress", not drx.search(PROG_NEW))
check("deploy regex ignores old-replica progress", not drx.search(PROG_OLD))
check("deploy regex ignores successful rollout", not drx.search(ROLLED_OK))
check("deploy regex catches rollout timeout", drx.search(ROLL_TIMEOUT))
check("deploy regex catches progress deadline", drx.search(ROLL_DEADLINE))
check("deploy regex still catches CrashLoopBackOff", drx.search(CRASHLOOP))
check("deploy regex still catches kubectl failure", drx.search("kubectl apply failed: connection refused"))

print(f"ci_harvest_echo_test: {_passed} checks passed")
