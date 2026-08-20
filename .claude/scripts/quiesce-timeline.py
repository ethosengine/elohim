#!/usr/bin/env python3
"""quiesce-timeline.py — turn the fleet-quiesce gate's own log lines into a series.

The gate at scripts/ci/fleet-quiesce-gate.sh already prints, once per poll, every
number needed to understand why the fleet did or did not quiesce. That output then
dies in a Jenkins console log. This reads it back and derives the numbers nobody
has had:

  * time-to-quiesce — gate start to A-QUIESCED (or to the deadline)
  * sustained — the winning window's length vs the required sustain
  * best-window — the LONGEST window achieved on a run that did NOT measure.
    On edge #1371 that was 303s against a 330s requirement: a 27-second miss,
    invisible in a pass/fail verdict, and the difference between "the fleet is
    broken" and "the gate is 8% too strict".
  * reset ledger — how many times the sustain window opened and reset, and WHICH
    leg reset it. This is the gate's composition residual: a run blocked by
    A-content-503 is a doorway availability story; one blocked by A-not-quiesced
    is a notary-plane story. Same verdict, opposite cures.

A READER, not a register (see 2026-08-20-latency-valueflow-chain-design.md §4).
Jenkins holds the raw text; this derives. The JSONL it appends is a local series
cache alongside genesis/a2o/reports/, not a source of truth.

    quiesce-timeline.py --build 1374
    quiesce-timeline.py --builds 1366-1374        # the series
    quiesce-timeline.py --build 1374 --record     # append to the local series
    quiesce-timeline.py --series                  # read back what's recorded

Jenkins reads are anonymous (OIDC-protected instance allows GET on console text).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from datetime import datetime
from pathlib import Path

JENKINS = "https://jenkins.ethosengine.com/job/elohim-edge/job/dev"
SERIES = Path(__file__).resolve().parents[2] / ".claude" / "data" / "quiesce-timeline.jsonl"

# fleet-quiesce[2026-08-20T16:35:30Z]: PASS A-caughtUp=True ... — sustained 363s, ...
LINE = re.compile(
    r"fleet-quiesce\[(?P<ts>[0-9T:\-]+Z)\]:\s+(?P<verdict>PASS|FAIL)\s+(?P<body>.*)$"
)
SUSTAINED = re.compile(r"sustained (?P<s>\d+)s")
ELAPSED = re.compile(r"elapsed (?P<s>\d+)s")
REASONS = re.compile(r"—\s*(?:resetting sustain window\s*)?\((?P<r>[^)]*)\)")


def fetch(build: int) -> str | None:
    url = f"{JENKINS}/{build}/consoleText"
    try:
        with urllib.request.urlopen(url, timeout=120) as r:
            return r.read().decode("utf-8", "replace")
    except Exception as e:
        print(f"  build {build}: fetch failed ({e})", file=sys.stderr)
        return None


def parse_ts(s: str) -> datetime:
    return datetime.strptime(s, "%Y-%m-%dT%H:%M:%SZ")


def analyse(build: int, text: str) -> dict | None:
    polls = []
    for line in text.splitlines():
        m = LINE.search(line)
        if m:
            polls.append((parse_ts(m.group("ts")), m.group("verdict"), m.group("body"), line))
    if not polls:
        return None

    measured = "A-QUIESCED" in text
    no_measure = "DID NOT MEASURE" in text
    start, end = polls[0][0], polls[-1][0]

    # Reset ledger + best window. A window opens on the first PASS after a FAIL
    # and closes when a FAIL resets it; its length is the elapsed the gate itself
    # reports, which is authoritative over anything we'd recompute from stamps.
    resets, best, cur = [], 0, 0
    for ts, verdict, body, raw in polls:
        if verdict == "PASS":
            e = ELAPSED.search(body)
            s = SUSTAINED.search(body)
            if s:
                cur = int(s.group("s"))
            elif e:
                cur = int(e.group("s"))
            else:
                cur = 0
            best = max(best, cur)
        else:
            if cur > 0:
                # a window was open and this FAIL ended it
                r = REASONS.search(body)
                resets.append({"at": ts.isoformat() + "Z", "after_s": cur,
                               "reason": (r.group("r").strip() if r else "unknown")})
            cur = 0

    # Which legs blocked, counted across every failing poll. This is the number
    # that says whether a run was a doorway story or a notary story.
    legs: dict[str, int] = {}
    for _, verdict, body, _ in polls:
        if verdict != "FAIL":
            continue
        r = REASONS.search(body)
        if not r:
            continue
        for leg in r.group("r").split():
            key = re.sub(r"\(.*", "", leg)
            legs[key] = legs.get(key, 0) + 1

    return {
        "build": build,
        "measured": measured,
        "no_measure": no_measure,
        "polls": len(polls),
        "gate_start": start.isoformat() + "Z",
        "gate_end": end.isoformat() + "Z",
        "time_to_verdict_s": int((end - start).total_seconds()),
        "best_window_s": best,
        "resets": len(resets),
        "reset_detail": resets,
        "blocking_legs": dict(sorted(legs.items(), key=lambda kv: -kv[1])),
    }


def render(rs: list[dict]):
    hdr = (f"{'build':>7}{'verdict':>14}{'polls':>7}{'to-verdict':>12}"
           f"{'best-window':>13}{'resets':>8}  blocking legs (count)")
    print(hdr)
    print("-" * (len(hdr) + 18))
    for r in rs:
        v = "MEASURED" if r["measured"] else ("no-measure" if r["no_measure"] else "?")
        legs = ", ".join(f"{k}×{n}" for k, n in list(r["blocking_legs"].items())[:3]) or "—"
        print(f"{r['build']:>7}{v:>14}{r['polls']:>7}"
              f"{r['time_to_verdict_s']:>11}s{r['best_window_s']:>12}s{r['resets']:>8}  {legs}")

    measured = [r for r in rs if r["measured"]]
    missed = [r for r in rs if not r["measured"]]
    print(f"\nmeasured {len(measured)}/{len(rs)} runs"
          + (f" — time-to-quiesce: "
             f"min {min(r['time_to_verdict_s'] for r in measured)}s / "
             f"max {max(r['time_to_verdict_s'] for r in measured)}s" if measured else ""))
    if missed:
        near = [r for r in missed if r["best_window_s"] > 0]
        if near:
            worst = max(near, key=lambda r: r["best_window_s"])
            print(f"NEAR MISS: build {worst['build']} reached {worst['best_window_s']}s of sustain "
                  f"and reset {worst['resets']}×. A pass/fail verdict cannot show this — it is the")
            print("  difference between 'the fleet is broken' and 'the gate is a few percent too strict'.")
        agg: dict[str, int] = {}
        for r in missed:
            for k, n in r["blocking_legs"].items():
                agg[k] = agg.get(k, 0) + n
        if agg:
            print("  legs blocking the no-measure runs: "
                  + ", ".join(f"{k}×{n}" for k, n in sorted(agg.items(), key=lambda kv: -kv[1])))
            print("  (a doorway-content leg is an AVAILABILITY story; A-not-quiesced/caughtUp is a")
            print("   NOTARY-plane story. Same verdict, opposite cures — never read the verdict alone.)")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--build", type=int)
    ap.add_argument("--builds", help="inclusive range, e.g. 1366-1374")
    ap.add_argument("--record", action="store_true", help="append results to the local series")
    ap.add_argument("--series", action="store_true", help="read back the recorded series")
    args = ap.parse_args()

    if args.series:
        if not SERIES.exists():
            print("no series recorded yet — run with --record", file=sys.stderr)
            return 1
        rows = [json.loads(l) for l in SERIES.read_text().splitlines() if l.strip()]
        seen, uniq = set(), []
        for r in sorted(rows, key=lambda r: r["build"]):
            if r["build"] in seen:
                continue
            seen.add(r["build"])
            uniq.append(r)
        render(uniq)
        return 0

    builds = []
    if args.builds:
        a, _, b = args.builds.partition("-")
        builds = list(range(int(a), int(b) + 1))
    elif args.build:
        builds = [args.build]
    else:
        ap.error("need --build, --builds or --series")

    results = []
    for b in builds:
        text = fetch(b)
        if text is None:
            continue
        r = analyse(b, text)
        if r is None:
            print(f"  build {b}: no fleet-quiesce lines "
                  "(stage skipped, or the build never reached it)", file=sys.stderr)
            continue
        results.append(r)

    if not results:
        print("nothing parsed — ABSENT is not zero; the gate may not have run.", file=sys.stderr)
        return 2

    render(results)
    if args.record:
        SERIES.parent.mkdir(parents=True, exist_ok=True)
        with SERIES.open("a") as fh:
            for r in results:
                fh.write(json.dumps(r) + "\n")
        print(f"\nrecorded {len(results)} run(s) → {SERIES}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
