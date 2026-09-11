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


def _observation_module():
    """The hooks' shared emitter (binary resolution + the kit->bounds producer bridge)."""
    sys.path.insert(0, os.path.join(PROJECT, ".claude", "hooks"))
    try:
        import _observation
        return _observation
    except Exception:  # noqa: BLE001 — resolution is best-effort
        return None


def headline_text():
    """The budget headline. Reuse the cache load-project-context.py wrote earlier THIS
    SessionStart (<120s old) so it is computed once, not once per consumer; otherwise ask
    `epr flow report --headline`, which is the sole OWNER. The last-resort kit re-run went at
    station six round (a) and the producer bridge at round (b) (2026-09-11), once every
    headline slot had a native producer — bridging a derived value would double it.
    Fails open (returns '')."""
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
    obs = _observation_module()
    if obs:
        try:
            binary = obs.resolve_bin()
        except Exception:  # noqa: BLE001
            binary = None
        if binary:
            try:
                r = subprocess.run(
                    [binary, "flow", "report", "--headline", "--root", PROJECT],
                    capture_output=True, text=True, timeout=10, cwd=PROJECT,
                )
                if r.returncode == 0 and r.stdout.strip():
                    return r.stdout
            except Exception:  # noqa: BLE001 — native headline fetch is best-effort
                pass
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


def claimed_unverified():
    """The over-claim count: docs in an ACTIVE home whose status claims done with no verification.

    Read from `epr flow report placement --json` — the native owner of document placement since
    station two of the memory-kit replacement. The ACTIVE-home filter is the load-bearing half and
    is kept exactly: CANONICAL and HISTORY are the SETTLED destinations, so an `accepted`/`landed`
    status there is the correct steady state rather than debt awaiting CI, and counting them would
    point the conveyor at documents that are already where they belong.

    This deliberately does NOT enumerate the gap-item store (`.eprfs/status/gap-items/`) — raw station totals run to
    thousands across terminal and historical specs and read as noise. It never did; it read a number
    out of the kit's printed headline instead. It now reads the same judgment from a JSON field.

    Falls back to scraping the headline text while the native binary is absent. Returns 0 on every
    failure path — a planning-feed advisory that cannot compute stays silent rather than guessing.
    """
    obs = _observation_module()
    binary = None
    if obs:
        try:
            binary = obs.resolve_bin()
        except Exception:  # noqa: BLE001 — resolution is best-effort
            binary = None
    if binary:
        try:
            r = subprocess.run(
                [binary, "flow", "report", "placement", "--json", "--root", PROJECT],
                capture_output=True, text=True, timeout=30, cwd=PROJECT,
            )
            if r.returncode == 0 and r.stdout.strip():
                rows = json.loads(r.stdout).get("rows") or []
                return sum(1 for row in rows
                           if row.get("state") == "CLAIMED-ONLY"
                           and str(row.get("position", "")).startswith("ACTIVE:"))
        except Exception:  # noqa: BLE001 — native placement read is best-effort
            pass
    try:
        m = re.search(r"(\d+)\s+claimed-unverified", headline_text())
        return int(m.group(1)) if m else 0
    except Exception:  # noqa: BLE001 — headline parse is best-effort
        return 0


def main():
    claimed = claimed_unverified()

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
