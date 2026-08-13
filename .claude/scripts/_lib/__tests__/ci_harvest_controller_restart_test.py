"""ci-harvest CONTROLLER_RESTART classification test — the ORPHANED-WAVE class.

Grounding: the 2026-08-13 Jenkins controller restart (~19:05 UTC) orphaned an
entire dispatched wave. Three distinct symptoms, ONE cause:

  elohim-edge/dev #1343   ABORTED  — agent pod gone; the 2h stage timeout had
                                     already expired during the outage, so the
                                     resume immediately cancelled and all 11
                                     remaining stages read "skipped due to
                                     earlier failure(s)".
  elohim/dev     #1664    FAILURE  — same restart, same "Waiting for
                                     reconnection of <pod>".
  elohim-orchestrator/dev #1669 FAILURE — its persisted CPS program failed to
                                     REPARSE on resume: a Groovy
                                     MultipleCompilationErrorsException naming
                                     a line that is perfectly valid (the job's
                                     very next build compiled the same file).

fp 2ec906730fe7 was filed UNCLASSIFIED as "red build, stage:elohim-edge",
which is positionally true and causally misleading — the orchestrator died on
resume, never on a downstream verdict. This test pins the fix so the class
self-classifies and costs no further background triage dispatch.

Run: python3 .claude/scripts/_lib/__tests__/ci_harvest_controller_restart_test.py
(exit 0 = pass)
"""
import importlib.util
import json
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


taxonomy = ci_harvest.load_taxonomy()
by_name = {t[0]: t for t in taxonomy}

# ---------------------------------------------------------------- category
check("CONTROLLER_RESTART category exists", "CONTROLLER_RESTART" in by_name)

# It must be FIRST. _scan_console spends MAX_CONSOLE_FINDINGS_PER_BUILD (4) in
# taxonomy order; museum trap #14 is that a crowded budget DISPLACES the real
# cause. A restart is the causally-dominant fact in its tail — it goes first.
check("CONTROLLER_RESTART is scanned first", taxonomy[0][0] == "CONTROLLER_RESTART")

rx = by_name["CONTROLLER_RESTART"][2]

# It must apply to every harvested job — a controller restart is not per-pipeline.
pipes = by_name["CONTROLLER_RESTART"][1]
for job in ci_harvest.JOBS:
    check(f"CONTROLLER_RESTART applies to {job}", any(p in job for p in pipes))

# ------------------------------------------------- decisive lines classify
ORPHANED_POD = (
    "Waiting for reconnection of elohim-edge-dev-1343-2xj5w-f2tpg-w93g9 "
    "before proceeding with build"
)
NO_INTERRUPT = (
    "Could not connect to elohim-edge-dev-1343-2xj5w-f2tpg-w93g9 to send "
    "interrupt signal to process"
)
CPS_RESUME_DEAD = "ERROR: Failed to load program"

for label, line in [
    ("orphaned agent pod", ORPHANED_POD),
    ("no interrupt channel", NO_INTERRUPT),
    ("CPS program failed to reload", CPS_RESUME_DEAD),
]:
    check(f"classifies: {label}", rx.search(line))

# ------------------------------------------------------- non-capture rails
# "Resuming build … after Jenkins restart" is BENIGN on its own — a build that
# resumed cleanly and then failed for a real code reason must keep its real
# cause. Capturing it would be museum trap #14 (displacement) all over again.
BENIGN_RESUME = "Resuming build at Thu Aug 13 21:35:01 UTC 2026 after Jenkins restart"
check("benign resume NOT captured", not rx.search(BENIGN_RESUME))

# A plain timeout kill is a DIFFERENT concern (a genuine hang) — don't absorb it.
check(
    "grace-period kill NOT captured",
    not rx.search("Body did not finish within grace period; terminating with extreme prejudice"),
)
check("plain timeout NOT captured", not rx.search("Timeout expired 33 min ago"))

# Real build failures must not be swallowed by the new first-place category.
for line in [
    "#33 128.4 Compiling fixt v0.7.0-dev.1",
    "error[E0308]: mismatched types",
    "error TS2739: Type is missing the following properties",
    "+ nerdctl -n k8s.io rmi elohim-storage:1.0.0-dev-c1fd38c7",
]:
    check(f"real/benign build line NOT captured: {line[:40]}", not rx.search(line))

# ------------------------------------------- fingerprint stability (churn)
# Without the pod-name scrub in normalize(), every restart mints a NEW
# fingerprint for the same concern — one background triage dispatch per pod
# name. Two different pods, two different jobs, two build numbers: one ident.
a = ci_harvest.normalize(ORPHANED_POD)
b = ci_harvest.normalize(
    "Waiting for reconnection of elohim-dev-1664-p3hd7-87jz3-s2tk7 "
    "before proceeding with build"
)
check("agent-pod name scrubbed from ident", "2xj5w" not in a and "1343" not in a)
check("pod churn does not fork the fingerprint", a == b)

c = ci_harvest.normalize(NO_INTERRUPT)
check("interrupt-line pod name scrubbed", "f2tpg" not in c)

# The scrub must stay narrow — it must not eat ordinary hyphenated identifiers.
keep = ci_harvest.normalize("error: cannot find crate elohim-storage-client in registry")
check("ordinary hyphenated names survive normalize", "elohim-storage-client" in keep)

# ---------------------------------------------------------- taxonomy sanity
raw = json.loads((root / ".claude" / "data" / "failure-taxonomy.json").read_text())
cat = raw["categories"]["CONTROLLER_RESTART"]
check("carries a remediation hint", "etrigger" in cat["fix"])
check("blocks nothing (not a dependency fault)", cat["blocks"] == [])

print(f"ci_harvest_controller_restart_test: {_passed} checks passed")
sys.exit(0)
