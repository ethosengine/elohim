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
check("has _scan_for_banner helper", hasattr(ci_harvest, "_scan_for_banner"))
check("has _scan_console helper", hasattr(ci_harvest, "_scan_console"))
check("has collect_build_findings helper", hasattr(ci_harvest, "collect_build_findings"))

# Banner detection now lives in _scan_for_banner (a single dict or None),
# separated from _scan_console's taxonomy-only scan — see FIX 3, final
# whole-arc review 2026-08-10: a red build with pre-existing findings
# (failed tests) must not swallow the banner.
f = with_habits_fixture(lambda: ci_harvest._scan_for_banner(TAIL_NO_MEASURE))
check("banner detected", f is not None)
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
f2 = with_habits_fixture(lambda: ci_harvest._scan_for_banner(TAIL_NO_MEASURE_2))
check("second no-measure also detected", f2 is not None)
got_fp2 = ci_harvest.fingerprint("elohim-edge", f2["category"], f2["ident"])
check("fingerprint stable across repeat no-measures", got_fp2 == expected_fp)

# No habits.yaml at all (FileNotFoundError) → concern omitted, never guessed.
with tempfile.TemporaryDirectory() as td:
    missing = Path(td) / "does-not-exist.yaml"
    orig = ci_harvest.HABITS_PATH
    ci_harvest.HABITS_PATH = str(missing)
    try:
        f3 = ci_harvest._scan_for_banner(TAIL_NO_MEASURE)
    finally:
        ci_harvest.HABITS_PATH = orig
check("missing habits.yaml → no finding-breaking crash", f3 is not None)
check("missing habits.yaml → concern honestly absent (None, not omitted at this layer)", f3.get("concern") is None)

# No banner in tail → None, not a list.
check("no banner → None", ci_harvest._scan_for_banner("nothing interesting here\n") is None)

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

# ---------------------------------------------------------------- FIX 3 (final whole-arc review 2026-08-10):
# a red build that already yielded failed-test findings must NOT swallow the
# NO_MEASURE banner. collect_build_findings does its own network I/O
# (get_json for the testReport, get_console_tail for the console) — stub
# both so this stays offline and deterministic.
_TEST_REPORT = {
    "suites": [
        {"cases": [{"className": "SomeSuite", "name": "test_thing", "status": "FAILED"}]}
    ]
}


def _fake_get_json(path):
    if "testReport" in path:
        return _TEST_REPORT
    raise AssertionError(f"unexpected get_json call in this test: {path}")


def _fake_get_console_tail(job, build):
    return TAIL_NO_MEASURE


def with_stubbed_network(fn):
    orig_get_json = ci_harvest.get_json
    orig_get_console_tail = ci_harvest.get_console_tail
    ci_harvest.get_json = _fake_get_json
    ci_harvest.get_console_tail = _fake_get_console_tail
    try:
        return fn()
    finally:
        ci_harvest.get_json = orig_get_json
        ci_harvest.get_console_tail = orig_get_console_tail


combined = with_habits_fixture(
    lambda: with_stubbed_network(lambda: ci_harvest.collect_build_findings("elohim-edge", 123, []))
)
check(
    "red build + non-empty test findings + banner in tail → BOTH findings present",
    len(combined) == 2,
)
categories = {f["category"] for f in combined}
check("TEST_FAILURE still present", "TEST_FAILURE" in categories)
check("NO_MEASURE finding still minted (not swallowed by pre-existing findings)", "NO_MEASURE" in categories)
no_measure_finding = next(f for f in combined if f["category"] == "NO_MEASURE")
check("no-measure finding carries the fixed identifier", no_measure_finding["ident"] == "dataplane-validation-did-not-measure")
check("no-measure finding carries the routed concern", no_measure_finding.get("concern") == "notary-authority")

print(f"ci_harvest_no_measure_test: {_passed} checks passed")
