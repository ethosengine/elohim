#!/usr/bin/env python3
"""quiesce-timeline.py — turn the fleet-quiesce gate's own log lines into a series.

AGENT-AGNOSTIC BY PLACEMENT. This lives in `genesis/scripts/` — not under
`.claude/` — because it is tooling for reading OUR OWN SIMULACRA, and any agent
(Claude, Codex, Gemini, a human at a terminal) should be able to reach for it.
Sibling of `genesis/scripts/jenkins-sync.sh`, which fetches the CI artifact this
one derives from the same builds. `.claude/scripts/ci-harvest.py` imports
`parse_quiesce` from here rather than carrying its own copy — one parser, one
place to fix when the gate's log format moves.

The gate at scripts/ci/fleet-quiesce-gate.sh already prints, once per poll, every
number needed to understand why the fleet did or did not quiesce. That output then
dies in a Jenkins console log. This reads it back and derives the numbers nobody
has had:

  * time-to-quiesce — gate start to A-QUIESCED (or to the deadline)
  * sustained — the winning window's length vs the required sustain
  * best-window — the LONGEST window achieved on a run that did NOT measure.
    Invisible in a pass/fail verdict, and the difference between "the fleet is
    broken" and "the fleet was one poll short".

    THE WINDOW IS QUANTIZED, and the series is what shows it. Sustain is the
    separation between two PASSing observations, and observations happen only
    every QUIESCE_POLL_SECS (60), so the only reachable separations are ~60s
    multiples. Measured across builds 1366-1374: 242, 303, 362, 363, 423 — every
    one an integer poll count plus a couple of seconds of poll-execution drift.

    That makes QUIESCE_SUSTAIN_SECS=330 unreachable as written: nothing can land
    between 300s and 360s, so the threshold means "360s" exactly, and any value
    in (300, 360] would behave identically. #1371's 303s is therefore not a
    27-second miss to be tuned away — it is a ONE-POLL miss. Reading it as "8%
    too strict" invites shaving 8% off the threshold, which changes nothing:
    the next reachable value down is 300s, a full poll cheaper, and it would
    admit the 5-interval runs the 300s sweep cadence is meant to exclude.
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
    quiesce-timeline.py --local gate.log --label scenario=smoke
    quiesce-timeline.py --series                  # read back what's recorded

Jenkins reads are anonymous (OIDC-protected instance allows GET on console text).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

JOB = "elohim-edge"  # matches ci-harvest.py's QUIESCE_JOB — same series, same label
JENKINS = f"https://jenkins.ethosengine.com/job/{JOB}/job/dev"
SERIES = Path(__file__).resolve().parents[2] / ".claude" / "data" / "quiesce-timeline.jsonl"

# fleet-quiesce[2026-08-20T16:35:30Z]: PASS A-caughtUp=True ... — sustained 363s, ...
LINE = re.compile(
    r"fleet-quiesce\[(?P<ts>[0-9T:\-]+Z)\]:\s+(?P<verdict>PASS|FAIL)\s+(?P<body>.*)$"
)
SUSTAINED = re.compile(r"sustained (?P<s>\d+)s")
ELAPSED = re.compile(r"elapsed (?P<s>\d+)s")
REASONS = re.compile(r"—\s*(?:resetting sustain window\s*)?\((?P<r>[^)]*)\)")
ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")
# Jenkins' own last word. Line-anchored — see result_from_console().
FINISHED = re.compile(r"^Finished:\s*(?P<r>[A-Z_]+)\s*$", re.M)

# The three outcomes, rendered. Kept beside the parser that mints them so a new
# outcome cannot be added without a label — an unlabelled outcome renders "?"
# and silently reads as "nothing happened".
OUTCOME_LABEL = {
    "measured": "MEASURED",
    "no_measure": "no-measure",
    "interrupted": "INTERRUPTED",
    "unknown": "?",
}


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


def age_phrase(stamp: str) -> str | None:
    """How stale an observation is, in words. Returns None if unparseable —
    an unreadable stamp must not print a confident-looking age."""
    try:
        delta = datetime.now(timezone.utc) - parse_ts(stamp).replace(tzinfo=timezone.utc)
    except Exception:
        return None
    mins = int(delta.total_seconds() // 60)
    if mins < 0:
        return None  # clock skew between writer and reader; say nothing
    if mins < 60:
        return f"{mins}m ago"
    if mins < 60 * 48:
        return f"{mins // 60}h ago"
    return f"{mins // 1440}d ago"


def parse_quiesce(
    build: int | None,
    text: str,
    build_result: str | None = None,
    *,
    source: str = "fleet",
    labels: dict | None = None,
) -> dict | None:
    """THE shared parser. One console body -> one honest record, or None.

    Importable: `.claude/scripts/ci-harvest.py` calls this so the gate's log
    format has exactly one reader to update. Any other agent's tooling should
    call it too rather than re-deriving the regexes.

    `build_result` distinguishes the three outcomes. Collapsing them is how a
    coin-flip gets read as a green:

      measured      the gate reached A-QUIESCED; time_to_verdict_s is real
      no_measure    the gate ran its deadline and gave up; best_window_s says how
                    close — in POLLS, not seconds (see the quantization note in
                    the module docstring: #1371's 303s vs 330s is one poll short,
                    not 27 seconds short)
      interrupted   ABORTED/superseded MID-GATE. NOT a quiesce verdict. #1373
                    produced 2 polls over 70s before abort-previous killed it;
                    recorded naively that reads as "the fleet settled in 70s".

    READING CAVEAT carried in every record via `observed_at`: a quiesce result is
    POINT-IN-TIME AND DECAYS. "The build is green" does not mean "the p2p
    dataplanes are settled now" — the gate passed at an instant, validation then
    ran for many minutes, and the fleet kept moving. Weigh `observed_at` against
    now before treating a recorded PASS as current state. Evidence about a
    moment, never a live status.
    """
    polls = []
    for line in text.splitlines():
        m = LINE.search(ANSI.sub("", line))
        if m:
            polls.append((m.group("ts"), m.group("verdict"), m.group("body")))
    if not polls:
        return None  # gate never ran (stage skipped, or the build died before it)

    interrupted = build_result == "ABORTED"
    measured = (not interrupted) and ("A-QUIESCED" in text)
    no_measure = (not interrupted) and ("DID NOT MEASURE" in text)

    best, cur, resets = 0, 0, 0
    legs: dict[str, int] = {}
    for _ts, verdict, body in polls:
        if verdict == "PASS":
            m = SUSTAINED.search(body) or ELAPSED.search(body)
            cur = int(m.group("s")) if m else 0
            best = max(best, cur)
        else:
            if cur > 0:
                resets += 1
            cur = 0
            r = REASONS.search(body)
            if r:
                for leg in r.group("r").split():
                    key = re.sub(r"\(.*", "", leg)
                    legs[key] = legs.get(key, 0) + 1

    try:
        span = int((parse_ts(polls[-1][0]) - parse_ts(polls[0][0])).total_seconds())
    except Exception:
        span = None

    return {
        "build": build,
        "source": source,
        "labels": dict(labels or {}),
        "outcome": "interrupted" if interrupted else
                   ("measured" if measured else ("no_measure" if no_measure else "unknown")),
        "build_result": build_result,
        "observed_at": polls[-1][0],
        "polls": len(polls),
        "time_to_verdict_s": span,
        "best_window_s": best,
        "resets": resets,
        "blocking_legs": dict(sorted(legs.items(), key=lambda kv: -kv[1])),
        "verdict_trustworthy": not interrupted,
    }


def result_from_console(text: str) -> str | None:
    """The build's own verdict, taken from the console we already fetched.

    `/api/json` is OIDC-protected on this instance (it 403s to an anonymous
    reader); `consoleText` is not, and Jenkins writes its final verdict there as
    the last line. So the result costs no extra request and no credential.

    ANCHORED TO LINE START on purpose. `elohim-storage` has tests named
    `lookup_excludes_superseded_binding` and `rejects_superseded_approval`; a
    loose search for "superseded"/"aborted" anywhere in the body classifies a
    green build as interrupted because its unit tests have honest names.

    None means no verdict line — the build is still running, or the fetch was
    truncated. `parse_quiesce` then reports outcome "unknown" rather than
    guessing, which is the right answer for a gate that has not finished.
    """
    found = FINISHED.findall(text)
    return found[-1] if found else None


def render(rs: list[dict]):
    """Render records in `parse_quiesce`'s shape — the ONLY shape written.

    Reads `outcome`, never a `measured`/`no_measure` boolean pair: this renderer
    and `ci-harvest.py` consume one file, so a second record shape here means
    whichever writer ran last decides whether the reader crashes.
    """
    hdr = (f"{'build':>7}{'outcome':>14}{'polls':>7}{'to-verdict':>12}"
           f"{'best-window':>13}{'resets':>8}  blocking legs (count)")
    print(hdr)
    print("-" * (len(hdr) + 18))
    for r in rs:
        v = OUTCOME_LABEL.get(r.get("outcome"), "?")
        legs = ", ".join(f"{k}×{n}" for k, n in list(r["blocking_legs"].items())[:3]) or "—"
        span = r.get("time_to_verdict_s")
        span_s = f"{span:>11}s" if span is not None else f"{'—':>12}"
        record_id = r.get("build") or r.get("source")
        label_suffix = ""
        if r.get("labels"):
            label_suffix = " " + " ".join(f"{k}={v}" for k, v in r["labels"].items())
        print(f"{record_id:>7}{v:>14}{r['polls']:>7}"
              f"{span_s}{r['best_window_s']:>12}s{r['resets']:>8}  {legs}{label_suffix}")

    measured = [r for r in rs if r.get("outcome") == "measured"]
    missed = [r for r in rs if r.get("outcome") == "no_measure"]
    untrusted = [r for r in rs if not r.get("verdict_trustworthy", True)]

    times = [r["time_to_verdict_s"] for r in measured if r.get("time_to_verdict_s") is not None]
    print(f"\nmeasured {len(measured)}/{len(rs)} runs"
          + (f" — time-to-quiesce: min {min(times)}s / max {max(times)}s" if times else ""))

    # Interrupted runs are excluded from EVERY statistic above and below. A build
    # killed mid-gate by abort-previous has a real elapsed and real poll count,
    # and both are meaningless as quiesce measurements: #1373 died 2 polls in and
    # would otherwise report "the fleet settled in 70s" — faster than any run
    # that actually settled, and pure artefact of when the axe fell.
    if untrusted:
        print(f"  excluded {len(untrusted)} interrupted run(s) from the stats: "
              + ", ".join(f"#{r['build']}" for r in untrusted)
              + " — killed mid-gate, so their timings measure the abort, not the fleet.")

    if missed:
        near = [r for r in missed if r["best_window_s"] > 0]
        if near:
            worst = max(near, key=lambda r: r["best_window_s"])
            worst_id = worst.get("build") or worst.get("source")
            print(f"NEAR MISS: build {worst_id} reached {worst['best_window_s']}s of sustain "
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

    # Decay stamp. A recorded PASS is evidence about a moment; the fleet kept
    # moving after it. Print the age so nobody reads the newest row as "now".
    stamps = [r.get("observed_at") for r in rs if r.get("observed_at")]
    if stamps:
        newest = max(stamps)
        print(f"\nnewest observation {newest}"
              + (f" ({age_phrase(newest)})" if age_phrase(newest) else "")
              + " — point-in-time, not live status.")


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__,
                                 formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--build", type=int)
    ap.add_argument("--builds", help="inclusive range, e.g. 1366-1374")
    ap.add_argument(
        "--local",
        help=("parse a local mesh gate log (hc-mesh-quiesce.sh writes "
              "$MESH_DIR/quiesce-gate/<start>.log)"),
    )
    ap.add_argument(
        "--label",
        action="append",
        default=[],
        help="k=v label attached to a --local record (scenario=, shape=, run=)",
    )
    ap.add_argument("--record", action="store_true", help="append results to the local series")
    ap.add_argument("--series", action="store_true", help="read back the recorded series")
    args = ap.parse_args(argv)

    if args.local:
        labels = dict(kv.split("=", 1) for kv in args.label if "=" in kv)
        with open(args.local, encoding="utf-8", errors="replace") as fh:
            rec = parse_quiesce(None, fh.read(), None, source="local", labels=labels)
        if rec is None:
            print(f"{args.local}: no fleet-quiesce lines found", file=sys.stderr)
            return 2
        render([rec])
        if args.record:
            SERIES.parent.mkdir(parents=True, exist_ok=True)
            with SERIES.open("a") as out:
                out.write(json.dumps(rec) + "\n")
        return 0

    if args.series:
        if not SERIES.exists():
            print("no series recorded yet — run with --record", file=sys.stderr)
            return 1
        rows = [json.loads(l) for l in SERIES.read_text().splitlines() if l.strip()]
        # Newest append per build wins. A re-harvest exists to CORRECT an earlier
        # record — a build read while still running, or one re-read after its
        # result landed — so keeping the first occurrence would pin the series to
        # the least-informed reading of every build.
        fleet_latest = {r["build"]: r for r in rows if r.get("build") is not None}
        local = [r for r in rows if r.get("build") is None]
        render([fleet_latest[b] for b in sorted(fleet_latest)] + local)
        return 0

    builds = []
    if args.builds:
        a, _, b = args.builds.partition("-")
        builds = list(range(int(a), int(b) + 1))
    elif args.build:
        builds = [args.build]
    else:
        ap.error("need --build, --builds, --local or --series")

    results = []
    for b in builds:
        text = fetch(b)
        if text is None:
            continue
        # Same parser, same record shape, same interrupted-classification as the
        # ci-harvest leg — this CLI is a second CALLER, never a second parser.
        r = parse_quiesce(b, text, build_result=result_from_console(text))
        if r is None:
            print(f"  build {b}: no fleet-quiesce lines "
                  "(stage skipped, or the build never reached it)", file=sys.stderr)
            continue
        r["job"] = JOB
        r["harvested_at"] = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")
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
