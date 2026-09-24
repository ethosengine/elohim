#!/usr/bin/env python3
"""habits-status: pain + last evidence + cost on the top red's row (native-delivery Lane D4).

Algedonic slice-1 Task 5 joined open, concern-addressed findings to the top red's first
`@concern:` tag. Two gaps kept the delivery habit's pain off the headline:

  - a finding can be addressed to the HABIT itself (ci-harvest routes STAGE_OVER_BUDGET and
    BUDGET_EXHAUSTED to `push-delivers-within-budget`, and e22562ad0ec9 is triaged to
    `dataplane-convergence`), and push-delivers-within-budget declares no @concern tag at all;
  - the cost the habit prices (stage-wallclock@1, delivery-cost@1 — bounds whose `concern:`
    names the habit) lived only in `epr flow report`, never beside the pain it explains.

This pins: pain joins on the habit id as well as its tags; the top red's row carries
`pain · last evidence · cost` together; bound readings come through the native report (the test
substitutes READ_BOUND); an unreadable report is said, never rendered as health.

Run: python3 .claude/scripts/_lib/__tests__/habits_status_cost_test.py  (exit 0 = pass)
"""
import importlib.util
import json
import sys
import tempfile
from pathlib import Path

LIB = Path(__file__).resolve().parents[3] / "scripts" / "habits-status.py"
spec = importlib.util.spec_from_file_location("habits_status", LIB)
m = importlib.util.module_from_spec(spec)
spec.loader.exec_module(m)

fails = []


def check(name, got, want):
    if got != want:
        fails.append(f"{name}: got {got!r}, want {want!r}")


def check_true(name, cond):
    if not cond:
        fails.append(f"{name}: expected truthy")


DOC = {
    "updated": "2026-09-24",
    "habits": [
        {
            "id": "push-delivers-within-budget",
            "status": "red",
            "active": True,
            "checks": ["live series: `node genesis/orchestrator/delivery-series.mjs --window 10`"],
            "evidence": "DELTA 2026-09-24: ACTIVE for the sprint.\n\nRED written 2026-09-22.\n\nDELTA 2026-09-23a (RED preserved)",
        },
        {
            "id": "quiet-green",
            "status": "green",
            "active": False,
            "checks": ["a2o @concern:quiet"],
        },
    ],
}

REGISTRY = """
measures-version: 1
measures: []
lenses:
  - id: stage-wallclock-ceiling
    version: 1
    class: measure
    consumes: [stage-wallclock@1]
    hard: 20
    concern: push-delivers-within-budget
    status: active
  - id: delivery-cost-ceiling
    version: 1
    class: measure
    consumes: [delivery-cost@1]
    hard: 1
    concern: push-delivers-within-budget
    status: active
  - id: retired-cost-ceiling
    version: 1
    consumes: [old@1]
    hard: 1
    concern: push-delivers-within-budget
    status: superseded
  - id: unrelated-ceiling
    version: 1
    consumes: [gate-cycle-seconds@1]
    hard: 1200
    concern: dev-system-equilibrium
    status: active
"""

OUTCOMES = {
    "stage-wallclock-ceiling": {"bound": "stage-wallclock-ceiling@1", "outcome": "failed", "observed": 131.9,
                                "unit": "minutes", "watermarks": {"soft": None, "hard": 20.0},
                                "summary": "131.9 minutes is past the hard watermark 20"},
    "delivery-cost-ceiling": {"bound": "delivery-cost-ceiling@1", "outcome": "failed", "observed": 16.5,
                              "unit": "pipeline-hours", "watermarks": {"soft": None, "hard": 1.0},
                              "summary": "16.5 pipeline-hours is past the hard watermark 1"},
}
asked = []


def fake_read(bound_id):
    asked.append(bound_id)
    return OUTCOMES.get(bound_id)


with tempfile.TemporaryDirectory() as td:
    ledger = Path(td) / "ci-findings.jsonl"
    ledger.write_text("\n".join(json.dumps(e) for e in [
        {"fp": "be4a89a4a87d", "status": "open", "concern": "push-delivers-within-budget"},
        {"fp": "e22562ad0ec9", "status": "triaged", "concern": "dataplane-convergence"},
    ]) + "\n", encoding="utf-8")
    registry = Path(td) / "measures.yaml"
    registry.write_text(REGISTRY, encoding="utf-8")

    orig = (m.LEDGERS, m.COST_REGISTRIES, m.READ_BOUND)
    m.LEDGERS, m.COST_REGISTRIES, m.READ_BOUND = [ledger], [registry], fake_read
    try:
        check("cost bounds keyed by concern, active rows only",
              m.cost_bounds(), {"push-delivers-within-budget": ["stage-wallclock-ceiling", "delivery-cost-ceiling"],
                                "dev-system-equilibrium": ["unrelated-ceiling"]})

        head = m.headline(DOC)
        row = next((ln for ln in head.splitlines() if "top red: push-delivers-within-budget" in ln), "")
        check_true("top red row exists", row)
        check_true("pain joins on the habit id (no @concern tag needed)",
                   "· pain: 1 open @push-delivers-within-budget" in row)
        check_true("last evidence is the newest DELTA date", "· last evidence: delta 2026-09-24" in row)
        check_true("cost on the same row: stage wall clock",
                   "cost: stage-wallclock 131.9 minutes (hard 20) ⚠" in row)
        check_true("cost on the same row: delivery cost", "delivery-cost 16.5 pipeline-hours (hard 1) ⚠" in row)
        check("only the top red's bounds are read", sorted(asked), ["delivery-cost-ceiling", "stage-wallclock-ceiling"])

        full = m.full(DOC)
        check_true("full: cost line per habit", "    cost: stage-wallclock 131.9 minutes (hard 20) ⚠" in full)
        check_true("full: last evidence line", "    last evidence: delta 2026-09-24" in full)
        check_true("full: pain joins on habit id", "pain: 1 open (be4a89a4a87d)" in full)

        # a passing and an unfolded bound
        OUTCOMES["stage-wallclock-ceiling"] = {"bound": "stage-wallclock-ceiling@1", "outcome": "passed",
                                               "observed": 9.5, "unit": "minutes",
                                               "watermarks": {"soft": None, "hard": 20.0}, "summary": "ok"}
        OUTCOMES["delivery-cost-ceiling"] = {"bound": "delivery-cost-ceiling@1", "outcome": "skipped",
                                             "summary": "no fold for delivery-cost@1", "watermarks": {"hard": 1.0}}
        row = next(ln for ln in m.headline(DOC).splitlines() if "top red:" in ln)
        check_true("passed bound renders within its watermark", "stage-wallclock 9.5 minutes (hard 20) ✅" in row)
        check_true("unfolded bound says skipped, never 0", "delivery-cost skipped (no fold for delivery-cost@1)" in row)

        # the native report unreadable → said, never health
        m.READ_BOUND = lambda _bound: None
        row = next(ln for ln in m.headline(DOC).splitlines() if "top red:" in ln)
        check_true("unreadable report is named", "cost: not read (epr flow report unavailable)" in row)
        check_true("unreadable report never renders a check mark", "✅" not in row)

        # two active reds (the WIP fence): the second gets its own row with pain · evidence · cost
        m.READ_BOUND = fake_read
        two = {"habits": [
            {"id": "dataplane-convergence", "status": "red", "active": True, "checks": ["a2o @concern:federation-deploy"],
             "evidence": "DELTA 2026-09-20: x"},
            DOC["habits"][0],
        ]}
        head = m.headline(two)
        top_row = next(ln for ln in head.splitlines() if "top red:" in ln)
        also_row = next((ln for ln in head.splitlines() if ln.startswith("  also active: ")), "")
        check_true("top red stays the first active red", "top red: dataplane-convergence" in top_row)
        check_true("top red without a pricing bound carries no cost clause", "cost:" not in top_row)
        check_true("second active red has its own row",
                   also_row.startswith("  also active: push-delivers-within-budget · pain: 1 open"))
        check_true("second row carries evidence and cost", "last evidence: delta 2026-09-24" in also_row
                   and "delivery-cost skipped (no fold for delivery-cost@1)" in also_row)
        one = m.headline(DOC)
        check_true("one active red → no also-active row", "also active:" not in one)

        # a habit with no cost bound and no pain: no cost clause, no crash
        bare = {"habits": [{"id": "bare-red", "status": "red", "active": True, "checks": ["plain"]}]}
        row = next(ln for ln in m.headline(bare).splitlines() if "top red:" in ln)
        check_true("no bound, no cost clause", "cost:" not in row and "pain:" not in row)
        check_true("no dated evidence says so", "· last evidence: none recorded" in row)
    finally:
        m.LEDGERS, m.COST_REGISTRIES, m.READ_BOUND = orig

check_true("default COST_REGISTRIES are the two declared bound homes",
           sorted(p.name for p in m.COST_REGISTRIES) == ["measures.yaml", "policies.yaml"])

if fails:
    print("FAIL")
    for f in fails:
        print("  -", f)
    sys.exit(1)
print("ok — habits-status pain + last evidence + cost")
