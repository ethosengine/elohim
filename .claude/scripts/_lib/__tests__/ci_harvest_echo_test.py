"""ci-harvest console-scan precision test — the set -x echo over-capture class.

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

print(f"ci_harvest_echo_test: {_passed} checks passed")
