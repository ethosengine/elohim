#!/usr/bin/env python3
"""delivery-gate — SessionStart LIGHT advisory (delivery-stasis-loop spec §4.3).

A one-line planning feed for the session: delivery-scoreboard counts plus the
single highest-leverage opportunity that could be caught in flight. It is
LEGIBILITY, NOT A TRIGGER — the injected context explicitly instructs the
session to keep the pilot's direction and only raise an item if it BLOCKS the
pilot's subject. No network, local-file reads only, <1s, silent at stasis,
fail-silent always.
"""

import json
import os
import re
import subprocess
import sys
import time

PROJECT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)


def headline_text():
    """The placement-audit budget headline. Reuse the cache load-project-context.py wrote earlier
    THIS SessionStart (<120s old) so the heavy audit runs once, not once per consumer; fall back to
    our own subprocess otherwise. Both paths fail-open (return '')."""
    slug = re.sub(r"[^A-Za-z0-9]+", "-", PROJECT).strip("-")
    cache = f"/tmp/claude-headline-{slug}.txt"
    try:
        if os.path.getmtime(cache) > time.time() - 120:
            with open(cache, encoding="utf-8") as fh:
                txt = fh.read()
            if txt.strip():
                return txt
    except OSError:
        pass  # no cache / stale / unreadable → recompute below
    try:
        return subprocess.run(
            ["python3", os.path.join(PROJECT, ".claude/scripts/memory-kit/placement-audit.py"),
             "--headline"],
            capture_output=True, text=True, timeout=10, cwd=PROJECT,
        ).stdout
    except Exception:  # noqa: BLE001 — headline fetch is best-effort
        return ""


def jread(rel, default):
    try:
        with open(os.path.join(PROJECT, rel), encoding="utf-8") as fh:
            return json.load(fh)
    except (OSError, json.JSONDecodeError):
        return default


def jsonl_count(rel, pred=lambda e: True):
    path = os.path.join(PROJECT, rel)
    n = 0
    try:
        with open(path, encoding="utf-8") as fh:
            for raw in fh:
                try:
                    if pred(json.loads(raw)):
                        n += 1
                except json.JSONDecodeError:
                    continue
    except OSError:
        pass
    return n


def main():
    # Doc-budget numbers come from placement-audit's FILTERED judgment (the
    # session headline) — never recount raw gap-items here (raw totals
    # include terminal/historical specs and read as thousands of noise).
    claimed = 0
    try:
        head = headline_text()
        m = re.search(r"(\d+)\s+claimed-unverified", head)
        if m:
            claimed = int(m.group(1))
    except Exception:  # noqa: BLE001 — headline parse is best-effort
        pass

    ci_open = jsonl_count(".claude/data/ci-findings.jsonl", lambda e: e.get("status") == "open")
    dep_open = jsonl_count(".claude/data/deprecations.jsonl", lambda e: e.get("status") != "fixed")

    # FAILURE-CLASS-AWARE floor (mirrors delivery-scoreboard.py's verdict ladder):
    # floor pressure = a degraded window WITH open (live code) findings attributed.
    # Red whose findings are all blocked/triaged is the ceiling/wait track and must
    # not steer dispatch. (A bare worst-pass_ratio read "0%" at substrate-gated
    # jobs all day on 2026-06-09 and pointed the conveyor wrong.)
    open_by_job = {}
    path = os.path.join(PROJECT, ".claude/data/ci-findings.jsonl")
    try:
        with open(path, encoding="utf-8") as fh:
            for raw in fh:
                try:
                    e = json.loads(raw)
                except json.JSONDecodeError:
                    continue
                if e.get("status") == "open":
                    j = e.get("job") or "?"
                    open_by_job[j] = open_by_job.get(j, 0) + 1
    except OSError:
        pass
    code_red = sorted(
        job for job, window in (jread(".claude/data/ci-cursor.json", {}).get("recent", {}) or {}).items()
        if len(window) >= 4 and open_by_job.get(job)
        and any(r in ("FAILURE", "ABORTED", "UNSTABLE") for r in window[-3:]))

    pressures = []
    if claimed:
        pressures.append((claimed, f"{claimed} CLAIMED-unverified (→ /deliver)"))
    if ci_open:
        pressures.append((ci_open, f"{ci_open} open CI findings (→ ci-failure-triage / shift rails)"))
    if dep_open:
        pressures.append((dep_open, f"{dep_open} live dep/sec findings (→ /deprecation-stasis)"))
    if code_red:
        pressures.append((10, f"code-red CI floor: {', '.join(code_red)} (degraded window + open findings;"
                              f" env-gated red excluded)"))

    if not pressures:
        return  # stasis — say nothing

    pressures.sort(reverse=True)
    top = pressures[0][1]
    summary = " · ".join(p[1].split(" (")[0] for p in pressures[:4])

    context = (
        f"DELIVERY GATE (advisory, planning-feed only): {summary}. "
        f"Highest-leverage if the session has room: {top}. "
        f"Full loop: /delivery-stasis. RULE: this line informs session "
        f"direction and surfaces what could be caught in flight — it must NOT "
        f"disrupt or change the pilot's direction unless one of these items "
        f"is a BLOCKER to the subject the pilot is directing."
    )
    print(
        json.dumps(
            {
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": context,
                }
            }
        )
    )


if __name__ == "__main__":
    try:
        main()
    except BaseException:  # noqa: BLE001 — never break session start
        pass
    sys.exit(0)
