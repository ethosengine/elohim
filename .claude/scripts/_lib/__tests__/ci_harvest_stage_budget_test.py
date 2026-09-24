"""ci-harvest STAGE_OVER_BUDGET + durationMillis — the stage clock the ledger never kept.

Grounding (evidence-ladder spec §8): app builds #1718–#1726 each spent 122–132 minutes inside
"Publish and Verify App Delivery" waiting out the edge fleet's post-roll window, and the ledger
held ONE open fingerprint for all of them (e22562ad0ec9, "red build, stage:Publish and Verify App
Delivery", seen 10) with no duration anywhere: durations are scrubbed from fingerprints on purpose
(a duration in a fingerprint mints a new finding per build), and nothing else carried them.

This pins the repair:
  - a stage over its DECLARED budget (measures.yaml stage-wallclock-ceiling@1) is a finding of its
    own class, addressed to the habit that prices it via concern_routes;
  - its duration rides as a FIELD (`durationMillis`), never as a fingerprint input;
  - the unclassified-stage fallback carries the stage's duration too;
  - the harvest folds stage-wallclock@1 / delivery-cost@1 from the delivery series, and an
    unbounded cost (nothing delivered) is never folded as 0.

Run: python3 .claude/scripts/_lib/__tests__/ci_harvest_stage_budget_test.py  (exit 0 = pass)
"""
import importlib.util
import json
import os
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

spec = importlib.util.spec_from_file_location("ci_harvest", root / ".claude" / "scripts" / "ci-harvest.py")
ci_harvest = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ci_harvest)

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1


# ---------------------------------------------------------------- the declared budget
minutes, concern = ci_harvest.declared_stage_budget()
check("budget is read from measures.yaml stage-wallclock-ceiling (20 min)", minutes == 20)
check("concern is the habit the lens names", concern == "push-delivers-within-budget")
check("missing registry falls back, never raises",
      ci_harvest.declared_stage_budget(Path("/nonexistent/measures.yaml")) == (None, None))

# ---------------------------------------------------------------- STAGE_OVER_BUDGET
describe_1725 = {"stages": [
    {"name": "Unit Test", "status": "SUCCESS", "durationMillis": 120_000},
    {"name": "Publish and Verify App Delivery", "status": "FAILED", "durationMillis": 7_350_000},
    {"name": "Build Image", "status": "FAILED", "durationMillis": 180},
]}
describe_fast = {"stages": [
    {"name": "Publish and Verify App Delivery", "status": "SUCCESS", "durationMillis": 8 * 60_000},
]}
describe_skipped = {"stages": [
    {"name": "Unit Test", "status": "FAILED", "durationMillis": 90_000},
    {"name": "Publish and Verify App Delivery", "status": "FAILED", "durationMillis": 160},
]}

scan = ci_harvest._scan_stages_over_budget
got = scan("elohim", describe_1725, budget_minutes=20, concern="push-delivers-within-budget")
check("one over-budget stage in #1725", len(got) == 1)
f = got[0]
check("category STAGE_OVER_BUDGET", f["category"] == "STAGE_OVER_BUDGET")
check("ident names the stage and nothing else", f["ident"] == "stage:Publish and Verify App Delivery")
check("durationMillis rides as a field", f["durationMillis"] == 7_350_000)
check("display carries the minutes for a human", "122.5" in f["display"] and "20" in f["display"])
check("concern routed to the pricing habit", f["concern"] == "push-delivers-within-budget")
check("joins the ci-failure lifecycle (confirm/reopen)", f["class"] == "ci-failure")
check("a stage within budget is not a finding", scan("elohim", describe_fast, 20, "x") == [])
check("a stage failed only by an earlier failure has no clock", scan("elohim", describe_skipped, 20, "x") == [])
check("a job with no declared stage budget is never scanned", scan("elohim-edge", describe_1725, 20, "x") == [])
check("no declared budget, no finding (honest absence)", scan("elohim", describe_1725, None, None) == [])
check("absent describe body is not a finding", scan("elohim", None, 20, "x") == [])

fp_1725 = ci_harvest.fingerprint("elohim", f["category"], f["ident"])
slower = scan("elohim", {"stages": [dict(describe_1725["stages"][1], durationMillis=7_900_000)]}, 20, "x")[0]
check("fingerprint is stable across builds with different durations",
      fp_1725 == ci_harvest.fingerprint("elohim", slower["category"], slower["ident"]))

# ---------------------------------------------------------------- unclassified fallback keeps the clock
u = ci_harvest._unclassified_from_describe(describe_1725)
check("unclassified names the first failed stage", u["ident"] == "stage:Publish and Verify App Delivery")
check("unclassified carries that stage's durationMillis", u["durationMillis"] == 7_350_000)
check("unclassified fingerprint input unchanged (e22562ad0ec9 keeps its fp)",
      ci_harvest.fingerprint("elohim", "UNCLASSIFIED", u["ident"]) == "e22562ad0ec9")
check("no failed stage → plain unclassified, no duration",
      ci_harvest._unclassified_from_describe(describe_fast) == {
          "category": "UNCLASSIFIED", "ident": "unclassified", "display": "red build, unclassified",
          "class": "ci-failure"})

# ---------------------------------------------------------------- BUDGET_EXHAUSTED is addressed too
own_timeout = "Timeout has been exceeded\nFinished: ABORTED"
check("BUDGET_EXHAUSTED carries the pricing habit's concern",
      ci_harvest._scan_for_budget_timeout(own_timeout).get("concern") == "push-delivers-within-budget")

# ---------------------------------------------------------------- reconcile persists the field
with tempfile.TemporaryDirectory() as tmp:
    ci_harvest.LEDGER_PATH = os.path.join(tmp, "ci-findings.jsonl")
    ci_harvest.CURSOR_PATH = os.path.join(tmp, "ci-cursor.json")
    first = dict(f, build=1725)
    result = {"job": "elohim", "new": [first], "green": None, "builds_seen": [1725], "sequence": [(1725, "FAILURE")]}
    new, bumped, _, _ = ci_harvest.reconcile([result], ci_harvest.load_cursor())
    check("new entry filed", len(new) == 1)
    entry = new[0]
    check("entry carries durationMillis", entry.get("durationMillis") == 7_350_000)
    check("entry carries concern", entry.get("concern") == "push-delivers-within-budget")
    check("entry fp excludes the duration", entry["fp"] == fp_1725)
    again = dict(slower, build=1726, concern="push-delivers-within-budget")
    result2 = {"job": "elohim", "new": [again], "green": None, "builds_seen": [1726], "sequence": [(1726, "FAILURE")]}
    new2, bumped2, _, _ = ci_harvest.reconcile([result2], ci_harvest.load_cursor())
    check("recurrence bumps, never mints", new2 == [] and len(bumped2) == 1)
    check("bump refreshes durationMillis to the latest build", bumped2[0]["durationMillis"] == 7_900_000)
    rows = [json.loads(x) for x in open(ci_harvest.LEDGER_PATH, encoding="utf-8")]
    check("ledger holds one line for the stage", len(rows) == 1 and rows[0]["seen"] == 2)

# ---------------------------------------------------------------- delivery folds
series_incident = {"pipelines": {"elohim": {
    "considered": 10, "delivered": 0, "pipelineHours": 16.5, "costPerDeliveredHours": None,
    "publishVerify": {"n": 8, "p50Min": 122.5, "p90Min": 129.6}}}}
folds = ci_harvest.delivery_folds(series_incident)
by = {m: (v, r) for m, v, r in folds}
check("stage-wallclock@1 folds the publish+verify p90", by["stage-wallclock@1"][0] == 129.6)
check("zero delivered folds the hours spent as a LOWER BOUND, never 0",
      by["delivery-cost@1"][0] == 16.5 and "lower bound" in by["delivery-cost@1"][1])
series_ok = {"pipelines": {"elohim": {
    "considered": 10, "delivered": 8, "pipelineHours": 4.0, "costPerDeliveredHours": 0.5,
    "publishVerify": {"n": 8, "p50Min": 6, "p90Min": 9.5}}}}
by_ok = {m: v for m, v, _ in ci_harvest.delivery_folds(series_ok)}
check("a delivering series folds cost per bundle", by_ok["delivery-cost@1"] == 0.5)
series_unmeasured = {"pipelines": {"elohim": {
    "considered": 10, "delivered": 0, "pipelineHours": 1.0, "costPerDeliveredHours": None,
    "publishVerify": {"n": 0, "p50Min": None, "p90Min": None}}}}
check("no publish+verify reading → no stage fold (skipped, never 0)",
      [m for m, _, _ in ci_harvest.delivery_folds(series_unmeasured)] == ["delivery-cost@1"])
check("unreadable series → no folds", ci_harvest.delivery_folds(None) == []
      and ci_harvest.delivery_folds({"pipelines": {}}) == [])

print(f"ok — {_passed} checks")
