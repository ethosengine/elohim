"""ci-harvest NO_MEASURE finding class + @concern: attachment (algedonic slice-1, Task 2).

The `run-dataplane-validation.sh` gate-skip banner is an ALGEDONIC signal — the
absence of a measure, not a failure. It must mint a finding with a FIXED
identifier (repeat no-measures dedupe to ONE open finding — pain is a held
state, not a stream) and route to the active habit's @concern: address via
the Task-1 helper (`_lib/concern_routes.py`).

Mirrors ci_harvest_echo_test.py's entry point: importlib-loads the hyphenated
ci-harvest.py, then exercises the pure console-scan helper directly (no
network — Jenkins fetches are the caller's job, not this helper's).

Run: python3 .claude/scripts/_lib/__tests__/ci_harvest_no_measure_test.py  (exit 0 = pass)
"""
import importlib.util
import re
import tempfile
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


# Same fixture text Task 1's concern_routes_test.py uses — one active habit,
# notary-authority, carrying the @concern:notary-authority tag.
HABITS_FIXTURE = """
habits:
  - id: notary-authority
    status: red
    active: true
    checks:
      - "a2o @concern:notary-authority (genesis/a2o/features/dataplane/notary-authority.feature)"
  - id: other-habit
    status: green
    active: false
    checks:
      - "a2o @concern:saga-06-heads-converge"
"""

BANNER = "=== Dataplane Validation: DID NOT MEASURE ==="

TAIL_NO_MEASURE = f"""
+ bash run-dataplane-validation.sh
[gate] quiesce predicate not satisfied — skipping measure
{BANNER}
Stage skipped: dataplane validation gate not open
"""

TAIL_NO_MEASURE_2 = f"{BANNER}\nunrelated trailing noise on a different build\n"


def with_habits_fixture(fn):
    with tempfile.TemporaryDirectory() as td:
        habits_path = Path(td) / "habits.yaml"
        habits_path.write_text(HABITS_FIXTURE, encoding="utf-8")
        orig = ci_harvest.HABITS_PATH
        ci_harvest.HABITS_PATH = str(habits_path)
        try:
            return fn()
        finally:
            ci_harvest.HABITS_PATH = orig


check("has HABITS_PATH", hasattr(ci_harvest, "HABITS_PATH"))
check("has _scan_console helper", hasattr(ci_harvest, "_scan_console"))

findings = with_habits_fixture(lambda: ci_harvest._scan_console(TAIL_NO_MEASURE, [], "elohim-edge"))
check("exactly one finding", len(findings) == 1)
f = findings[0]
check("class == ci-no-measure", f.get("class") == "ci-no-measure")
check("category == NO_MEASURE", f.get("category") == "NO_MEASURE")
check("concern == notary-authority", f.get("concern") == "notary-authority")

expected_fp = ci_harvest.fingerprint(
    "elohim-edge", "NO_MEASURE", "dataplane-validation-did-not-measure"
)
got_fp = ci_harvest.fingerprint("elohim-edge", f["category"], f["ident"])
check("fingerprint == fixed identifier", got_fp == expected_fp)

# Repeat no-measures (different surrounding console noise, i.e. a later
# build) still produce the SAME fixed fingerprint — dedupe to ONE finding.
findings2 = with_habits_fixture(lambda: ci_harvest._scan_console(TAIL_NO_MEASURE_2, [], "elohim-edge"))
check("second no-measure also one finding", len(findings2) == 1)
got_fp2 = ci_harvest.fingerprint("elohim-edge", findings2[0]["category"], findings2[0]["ident"])
check("fingerprint stable across repeat no-measures", got_fp2 == expected_fp)

# No habits.yaml at all (FileNotFoundError) → concern omitted, never guessed.
with tempfile.TemporaryDirectory() as td:
    missing = Path(td) / "does-not-exist.yaml"
    orig = ci_harvest.HABITS_PATH
    ci_harvest.HABITS_PATH = str(missing)
    try:
        findings3 = ci_harvest._scan_console(TAIL_NO_MEASURE, [], "elohim-edge")
    finally:
        ci_harvest.HABITS_PATH = orig
check("missing habits.yaml → no finding-breaking crash", len(findings3) == 1)
check("missing habits.yaml → concern honestly absent", "concern" not in findings3[0] or findings3[0]["concern"] is None)

# ---------------------------------------------------------------- ordinary ci-failure @concern: attachment
_taxonomy = [("BUILD_FAILURE", [], re.compile(r"ERROR:"), 5)]
LINE_WITH_CONCERN = "ERROR: doorway boot failed @concern:saga-04-doorway-serves"
LINE_WITHOUT_CONCERN = "ERROR: unrelated boot failure with no tag"

ord_findings = with_habits_fixture(
    lambda: ci_harvest._scan_console(f"{LINE_WITH_CONCERN}\n{LINE_WITHOUT_CONCERN}\n", _taxonomy, "elohim")
)
check("two ordinary findings", len(ord_findings) == 2)
tagged = next(x for x in ord_findings if "saga-04-doorway-serves" in x["ident"])
untagged = next(x for x in ord_findings if x is not tagged)
check("tagged line class stays ci-failure", tagged.get("class") == "ci-failure")
check("tagged line gets concern", tagged.get("concern") == "saga-04-doorway-serves")
check("untagged line has NO concern key (honest absence)", "concern" not in untagged)

print(f"ci_harvest_no_measure_test: {_passed} checks passed")
