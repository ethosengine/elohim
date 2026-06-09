#!/usr/bin/env python3
"""delivery-scoreboard — the /delivery-stasis loop's Measure step, one deterministic read.

Pure-local (<2s, no Jenkins): consumes what the harvesters already persisted.
Run `python3 .claude/scripts/ci-harvest.py` FIRST when fresh CI evidence is
wanted; this script never touches the network.

Two design rules, both born from the 2026-06-09 shift:

1. FAILURE-CLASS-AWARE FLOOR. A bare pass_ratio counts SUCCESS only, so a job
   whose every remaining red is substrate-gated reads identically to one with
   live code rot — and a conveyor steering by that number keeps dispatching CI
   stations at a surface whose residual red is ABOVE the ceiling (operator's
   substrate). Here each job's window ratio is joined with its findings-ledger
   split (open = live code concern · triaged = fix landed, awaiting
   disappearance · blocked = env/substrate-gated) into a per-job VERDICT:
       code-red   — red window AND ≥1 open finding        → dispatchable work
       fix-landed — red window, no open, ≥1 triaged       → wait for streak
       env-gated  — red window, no open/triaged, blocked   → ceiling/held track
       attention  — red window, NO findings attributed     → harvest gap, look
       stable     — no red in window
2. INSTRUMENT LIVENESS. Every section is wrapped: a section that crashes emits
   `⚠ gate-error (…)` instead of vanishing. A dead instrument must never read
   as "nothing to report" (the silent scope-gate incident, 2026-06-09).

Usage:  python3 .claude/scripts/delivery-scoreboard.py [--json]
"""

import json
import os
import subprocess
import sys

PROJECT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)
AS_JSON = "--json" in sys.argv[1:]

CI_CURSOR = ".claude/data/ci-cursor.json"
CI_FINDINGS = ".claude/data/ci-findings.jsonl"
DEPRECATIONS = ".claude/data/deprecations.jsonl"
RED = ("FAILURE", "ABORTED")


def _jread(rel, default):
    with open(os.path.join(PROJECT, rel), encoding="utf-8") as fh:
        return json.load(fh) or default


def _jsonl(rel):
    out = []
    path = os.path.join(PROJECT, rel)
    if not os.path.exists(path):
        return out
    with open(path, encoding="utf-8") as fh:
        for raw in fh:
            raw = raw.strip()
            if not raw:
                continue
            try:
                out.append(json.loads(raw))
            except json.JSONDecodeError:
                continue
    return out


def section(fn):
    """Instrument-liveness wrapper: a section that dies says so, loudly, in-line."""
    def run():
        try:
            return fn()
        except Exception as e:  # noqa: BLE001 — the scoreboard must say which instrument died
            return [f"⚠ gate-error in {fn.__name__} ({type(e).__name__}: {str(e)[:80]})"]
    return run


@section
def ci_floor():
    """Per-job window ratio + findings-class split → verdict (the class-aware floor)."""
    cursor = _jread(CI_CURSOR, {})
    findings = _jsonl(CI_FINDINGS)
    by_job = {}
    for f in findings:
        j = f.get("job") or "?"
        s = f.get("status") or "open"
        by_job.setdefault(j, {"open": 0, "triaged": 0, "blocked": 0})
        if s in by_job[j]:
            by_job[j][s] += 1
    streaks = cursor.get("green_streak") or {}
    lines = []
    rows = {}
    for job, window in sorted((cursor.get("recent") or {}).items()):
        if not window:
            continue
        ratio = window.count("SUCCESS") / len(window)
        recent3 = window[-3:]
        fb = by_job.get(job, {"open": 0, "triaged": 0, "blocked": 0})
        streak = streaks.get(job, 0)
        # Verdict ladder (most-settled first). UNSTABLE counts as degraded, not
        # stable — it means "ran with failures"; only the findings split says
        # whether that's dispatchable code or ceiling-track substrate.
        if window[-1] == "SUCCESS" and streak >= 1 and (fb["open"] or fb["triaged"]):
            verdict = "clearing"      # green streak running; disappearance sweep owns closure
        elif not any(r in RED or r == "UNSTABLE" for r in recent3):
            verdict = "stable"
        elif fb["open"]:
            verdict = "code-red"
        elif fb["triaged"]:
            verdict = "fix-landed"
        elif fb["blocked"]:
            verdict = "env-gated"
        else:
            verdict = "attention"
        rows[job] = {"ratio": round(ratio, 2), "window": len(window),
                     "last": window[-1], "streak": streak, "verdict": verdict, **fb}
        mark = {"stable": "✅", "clearing": f"🟢streak {streak}/3", "fix-landed": "⏳",
                "env-gated": "⛔ceiling", "code-red": "🔴dispatch", "attention": "⚠look"}[verdict]
        lines.append(
            f"{job}: {ratio:.0%} ({len(window)}w, last {window[-1]})"
            f"  open={fb['open']} triaged={fb['triaged']} blocked={fb['blocked']}"
            f"  → {verdict} {mark}")
    # The conveyor's read: only code-red/attention rows are THIS loop's CI work.
    dispatchable = [j for j, r in rows.items() if r["verdict"] in ("code-red", "attention")]
    lines.append("dispatchable CI surface: " + (", ".join(dispatchable) if dispatchable
                 else "none — remaining red is env-gated/fix-landed (ceiling/wait track)"))
    return lines if not AS_JSON else rows


@section
def ledgers():
    ci = _jsonl(CI_FINDINGS)
    dep = _jsonl(DEPRECATIONS)
    ci_c = {"open": 0, "triaged": 0, "blocked": 0}
    for f in ci:
        s = f.get("status") or "open"
        if s in ci_c:
            ci_c[s] += 1
    dep_live = sum(1 for d in dep if d.get("status") != "fixed")
    line = (f"ci-findings: open={ci_c['open']} triaged={ci_c['triaged']} "
            f"blocked={ci_c['blocked']}   |   dep/sec live: {dep_live}")
    return [line] if not AS_JSON else {"ci": ci_c, "dep_live": dep_live}


@section
def doc_budget():
    """The placement headline verbatim — its gate lines now self-report liveness."""
    out = subprocess.run(
        [sys.executable, os.path.join(PROJECT, ".claude/scripts/memory-kit/placement-audit.py"),
         "--headline"],
        capture_output=True, text=True, timeout=30, cwd=PROJECT)
    if out.returncode != 0:
        err = (out.stderr or "").strip().splitlines()
        return [f"⚠ gate-error (placement-audit --headline: {err[-1][:90] if err else out.returncode})"]
    return [ln for ln in out.stdout.splitlines() if ln.strip()]


@section
def delivery_distribution():
    script = os.path.join(PROJECT, ".claude/scripts/memory-kit/delivery-status-distribution.py")
    if not os.path.exists(script):
        return ["delivery-status-distribution.py absent (skip)"]
    out = subprocess.run([sys.executable, script], capture_output=True, text=True,
                         timeout=30, cwd=PROJECT)
    if out.returncode != 0:
        err = (out.stderr or "").strip().splitlines()
        return [f"⚠ gate-error (delivery-status-distribution: {err[-1][:90] if err else out.returncode})"]
    store = os.path.join(PROJECT, ".claude/memory-kit/delivery-status-distribution.json")
    try:
        with open(store, encoding="utf-8") as fh:
            d = json.load(fh)
        keep = {k: v for k, v in d.items() if k in ("summary", "counts", "anomalies")} or d
        return [json.dumps(keep, default=str)[:400]]
    except (OSError, ValueError):
        return [(out.stdout or "").strip()[:400] or "(no distribution output)"]


def main():
    sections = [("CI FLOOR (class-aware)", ci_floor),
                ("LEDGERS", ledgers),
                ("DOC BUDGET (placement headline)", doc_budget),
                ("DELIVERY DISTRIBUTION", delivery_distribution)]
    if AS_JSON:
        print(json.dumps({name: fn() for name, fn in sections}, indent=1, default=str))
        return 0
    print("DELIVERY SCOREBOARD  (pure-local; run ci-harvest.py first for fresh CI evidence)")
    for name, fn in sections:
        print(f"── {name}")
        body = fn()
        for ln in (body if isinstance(body, list) else [str(body)]):
            print(f"   {ln}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
