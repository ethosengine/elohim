"""ci-harvest BUDGET_EXHAUSTED classification — the unseen-timeout class.

Grounding: from 2026-09-19 to 09-22, elohim-orchestrator/dev hit its own
240-minute limit on seven of eleven runs (#1884-#1891) and elohim/dev (app)
delivered zero of eight dispatches. All of those builds read ABORTED, and
the harvester filtered them out (RED = {FAILURE, UNSTABLE}), so the ledger
stayed quiet while the pipeline starved. The museum is right that most aborts
are not verdicts (superseded, restart-orphaned, manual stop), but a build its
own timeout killed is a budget signal. This pins that exactly that shape,
and nothing else, becomes a finding.

Run: python3 .claude/scripts/_lib/__tests__/ci_harvest_budget_exhausted_test.py
(exit 0 = pass)
"""
import importlib.util
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


scan = ci_harvest._scan_for_budget_timeout

own_timeout = "\n".join([
    "[Pipeline] // stage",
    "Timeout has been exceeded",
    "Cancelling nested steps due to timeout",
    "Finished: ABORTED",
])
superseded = "\n".join([
    "Superseded by #1892",
    "Click here to forcibly terminate running steps",
    "Finished: ABORTED",
])
manual_stop = "Aborted by Matthew Dowell\nFinished: ABORTED"
restart_expired = "\n".join([
    "Waiting for reconnection of elohim-edge-dev-1343-abcd before proceeding with build",
    "Timeout has been exceeded",
    "Finished: ABORTED",
])

f = scan(own_timeout)
check("own-timeout abort is a finding", f is not None)
check("category is BUDGET_EXHAUSTED", f["category"] == "BUDGET_EXHAUSTED")
check("ident is fixed, so one fingerprint per job", f["ident"] == "pipeline-budget-exhausted")
check("class joins the ci-failure lifecycle (confirm/reopen)", f["class"] == "ci-failure")
check("superseded abort is not a finding", scan(superseded) is None)
check("manual stop is not a finding", scan(manual_stop) is None)
check("restart-expired timeout stays CONTROLLER_RESTART, not budget", scan(restart_expired) is None)
check("empty tail is not a finding", scan("") is None and scan(None) is None)

fp_a = ci_harvest.fingerprint("elohim-orchestrator", f["category"], f["ident"])
fp_b = ci_harvest.fingerprint("elohim-orchestrator", f["category"], f["ident"])
fp_edge = ci_harvest.fingerprint("elohim-edge", f["category"], f["ident"])
check("fingerprint is stable across builds", fp_a == fp_b)
check("fingerprint is per job", fp_a != fp_edge)

print(f"ok — {_passed} checks")
