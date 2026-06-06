#!/usr/bin/env python3
"""ci-harvest — deterministic Jenkins findings harvester (findings-sentinel
pattern, instantiation B — spec: findings-sentinel-pattern-design §3.1).

Pulls completed builds since a per-job cursor from the anonymous Jenkins
JSON API, fingerprints failures into the CI findings ledger, tracks
occurrence counts (flake evidence) and green watermarks (closure-by-
disappearance evidence), and splits FRESH regressions (urgent, now-work)
from recurring signatures (backlog-work → triage dispatch directive).

Stores:
  .claude/data/ci-cursor.json    {jobs:{job:last_build}, green:{job:last_green}}
  .claude/data/ci-findings.jsonl one line per LIVE finding:
      {ts, fp, class:"ci-failure", category, job, line, status, seen,
       first_build, last_build, backlog?}
      status: open → triaged → blocked (triage owns transitions; closure =
      DELETION when the fingerprint stays gone — sweep-confirmed).

Modes:
  (default)      harvest all jobs since cursor; human-readable summary
  --hook         harvest; emit SessionStart-hook JSON (silent when nothing)
  --wait JOB     bounded poll until JOB's running build completes, then
                 harvest that job (post-push loop-closer; run in background)
  --jobs a,b     restrict harvest to listed jobs

Fail-safe: in --hook mode every error exits 0 silently — a Jenkins outage
must never break session start.
"""

import argparse
import fcntl
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone

JENKINS = "https://jenkins.ethosengine.com"
BRANCH = "dev"
JOBS = [
    "elohim",
    "elohim-edge",
    "elohim-genesis",
    "elohim-holochain",
    "elohim-orchestrator",
    "elohim-sophia",
    "elohim-steward",
    "elohim-storybook",
]
RED = {"FAILURE", "UNSTABLE"}
CONFIRM_STREAK = 3  # consecutive greens that confirm a triaged fix (disappearance)
RECENT_WINDOW = 15  # rolling per-job result window (pass/unstable/fail ratio rails)
SHALLOW_INIT = 5  # first run: look back this many builds, never full history
MAX_BUILDS_PER_JOB = 10  # per harvest run
MAX_NEW_FINDINGS = 12  # ledger appends per run (per-build console findings also capped)
MAX_CONSOLE_FINDINGS_PER_BUILD = 4
MAX_FAILED_TESTS_PER_BUILD = 8
CONSOLE_TAIL = 60_000  # bytes of console tail to classify
HTTP_TIMEOUT = 8

PROJECT = os.environ.get("CLAUDE_PROJECT_DIR") or os.path.dirname(
    os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
)
CURSOR_PATH = os.path.join(PROJECT, ".claude", "data", "ci-cursor.json")
LEDGER_PATH = os.path.join(PROJECT, ".claude", "data", "ci-findings.jsonl")
TAXONOMY_PATH = os.path.join(PROJECT, ".claude", "data", "failure-taxonomy.json")

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


def get_json(path):
    req = urllib.request.Request(JENKINS + path, headers={"Accept": "application/json"})
    with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
        return json.loads(resp.read().decode("utf-8", "replace"))


def get_console_tail(job, build):
    """Tail of the console via progressiveText (X-Text-Size two-step)."""
    base = f"/job/{job}/job/{BRANCH}/{build}/logText/progressiveText"
    probe = urllib.request.Request(JENKINS + base + "?start=2000000000")
    with urllib.request.urlopen(probe, timeout=HTTP_TIMEOUT) as resp:
        size = int(resp.headers.get("X-Text-Size", "0"))
    if size <= 0:
        return ""
    start = max(0, size - CONSOLE_TAIL)
    req = urllib.request.Request(JENKINS + base + f"?start={start}")
    with urllib.request.urlopen(req, timeout=HTTP_TIMEOUT) as resp:
        return resp.read(CONSOLE_TAIL + 4096).decode("utf-8", "replace")


def normalize(line):
    """Build-invariant error-line normalization: same failure across builds
    must produce one fingerprint."""
    line = ANSI.sub("", line)
    line = re.sub(r"\s+", " ", line).strip()
    line = re.sub(r"\b\d{1,3}(?:\.\d{1,3}){3}\b", "#", line)  # IPv4 (worker churn)
    line = re.sub(r":\d+(?::\d+)?", ":#", line)  # file:line(:col) refs / ports
    line = re.sub(r"\b[0-9a-f]{7,64}\b", "#", line)  # hashes / build ids
    line = re.sub(r"\b\d+(?:\.\d+)?(?:ms|s|m|h)\b", "#", line)  # durations
    line = re.sub(r"\b20\d{2}-\d{2}-\d{2}[T ][\d:.]+\S*", "#", line)  # timestamps
    return line[:300]


def fingerprint(job, category, identifier):
    # Identifier stays case-exact (test names are case-sensitive identifiers);
    # job/category are developer-controlled vocabulary, normalized lower.
    norm = f"{job.lower()}|{category.lower()}|{identifier}"
    return hashlib.sha256(norm.encode()).hexdigest()[:12]


def load_taxonomy():
    try:
        with open(TAXONOMY_PATH, encoding="utf-8") as fh:
            cats = json.load(fh).get("categories", {})
        out = []
        for name, spec in cats.items():
            try:
                out.append((name, spec.get("pipelines", []), re.compile(spec["search"]),
                            spec.get("max", 5)))
            except (KeyError, re.error):
                continue
        return out
    except (OSError, json.JSONDecodeError):
        return []


def load_jsonl(path):
    entries = []
    if os.path.exists(path):
        with open(path, encoding="utf-8", errors="replace") as fh:
            for raw in fh:
                try:
                    entries.append(json.loads(raw))
                except json.JSONDecodeError:
                    continue
    return entries


def write_jsonl(path, entries):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        for e in entries:
            fh.write(json.dumps(e, ensure_ascii=False) + "\n")
    os.replace(tmp, path)


def load_cursor():
    try:
        with open(CURSOR_PATH, encoding="utf-8") as fh:
            c = json.load(fh)
            c.setdefault("jobs", {})
            c.setdefault("green", {})
            return c
    except (OSError, json.JSONDecodeError):
        return {"jobs": {}, "green": {}}


def collect_build_findings(job, build, taxonomy):
    """Findings for one red build: failed tests first, console classification
    as the fallback/supplement. Returns [(category, identifier, display)]."""
    findings = []
    # 1. Failed test cases (UNSTABLE builds usually have a testReport).
    try:
        tr = get_json(
            f"/job/{job}/job/{BRANCH}/{build}/testReport/api/json"
            "?tree=suites[cases[className,name,status]]"
        )
        for suite in tr.get("suites", []):
            for case in suite.get("cases", []):
                if case.get("status") in ("FAILED", "REGRESSION"):
                    ident = f'{case.get("className", "?")}.{case.get("name", "?")}'
                    findings.append(("TEST_FAILURE", ident, ident))
                    if len(findings) >= MAX_FAILED_TESTS_PER_BUILD:
                        return findings
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, OSError):
        pass  # no test report — fall through to console classification

    # 2. Console-tail classification via the existing failure taxonomy.
    if not findings:
        try:
            tail = get_console_tail(job, build)
            cats = [t for t in taxonomy if any(p in job for p in t[1])] or taxonomy
            seen_lines = set()
            for name, _pipes, rx, _mx in cats:
                for line in tail.splitlines():
                    if rx.search(line):
                        norm = normalize(line)
                        if norm and norm not in seen_lines:
                            seen_lines.add(norm)
                            findings.append((name, norm, norm))
                            if len(findings) >= MAX_CONSOLE_FINDINGS_PER_BUILD:
                                return findings
        except (urllib.error.URLError, urllib.error.HTTPError, OSError):
            pass

    # 3. Nothing classified: record the red at stage granularity if possible.
    if not findings:
        stage = None
        try:
            wf = get_json(f"/job/{job}/job/{BRANCH}/{build}/wfapi/describe")
            for st in wf.get("stages", []):
                if st.get("status") in ("FAILED", "UNSTABLE"):
                    stage = st.get("name")
                    break
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, OSError):
            pass
        ident = f"stage:{stage}" if stage else "unclassified"
        findings.append(("UNCLASSIFIED", ident, f"red build, {ident}"))
    return findings


def harvest_job(job, cursor, taxonomy):
    """Returns (job_result dict) — never raises."""
    out = {"job": job, "new": [], "green": None, "last": None, "urgent": None, "builds_seen": []}
    try:
        data = get_json(
            f"/job/{job}/job/{BRANCH}/api/json"
            f"?tree=builds[number,result]{{0,{MAX_BUILDS_PER_JOB + 5}}}"
        )
    except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, OSError):
        return out  # job missing / offline — silent
    completed = [b for b in data.get("builds", []) if b.get("result")]
    if not completed:
        return out
    completed.sort(key=lambda b: b["number"])  # oldest → newest
    last_cursor = cursor["jobs"].get(job)
    if last_cursor is None:
        fresh = completed[-SHALLOW_INIT:]
    else:
        fresh = [b for b in completed if b["number"] > last_cursor][-MAX_BUILDS_PER_JOB:]
    out["last"] = completed[-1]["number"]

    # Urgent split: newest completed is red AND its closest completed
    # predecessor was green AND the red is fresh (not yet harvested).
    # Reverse-search rather than [-2]: aborted/NOT_BUILT builds are filtered
    # out above, so adjacency in the list isn't adjacency in build numbers.
    newest = completed[-1]
    prev = next(
        (b for b in reversed(completed[:-1]) if b["number"] < newest["number"]), None
    )
    if (
        newest["result"] in RED
        and prev is not None
        and prev["result"] == "SUCCESS"
        and (last_cursor is None or newest["number"] > last_cursor)
    ):
        out["urgent"] = {"build": newest["number"], "result": newest["result"]}

    for b in fresh:
        out["builds_seen"].append(b["number"])
        out.setdefault("sequence", []).append((b["number"], b["result"]))
        if b["result"] == "SUCCESS":
            out["green"] = max(out["green"] or 0, b["number"])
        elif b["result"] in RED:
            for category, ident, display in collect_build_findings(job, b["number"], taxonomy):
                out["new"].append(
                    {"build": b["number"], "category": category, "ident": ident, "display": display}
                )
    return out


def reconcile(results, cursor):
    """Apply harvest results to ledger + cursor. Returns (new_entries, bumped).

    Concurrency: SessionStart-hook and --wait invocations can overlap, and
    both do a load→modify→write cycle — serialized via an advisory flock.
    Crash-safety: ledger is written BEFORE the cursor deliberately. A crash
    between the writes re-harvests the same builds next run, which is
    idempotent (fp dedupe + the last_build guard prevent duplicate entries
    and double seen-bumps); the inverted order would permanently skip
    findings instead.
    """
    os.makedirs(os.path.dirname(LEDGER_PATH), exist_ok=True)
    lock = open(LEDGER_PATH + ".lock", "w")  # noqa: SIM115 — held for fn lifetime
    fcntl.flock(lock, fcntl.LOCK_EX)
    entries = load_jsonl(LEDGER_PATH)
    by_fp = {e["fp"]: e for e in entries}
    new_entries, bumped = [], []
    now = datetime.now(timezone.utc).isoformat(timespec="seconds")
    for r in results:
        for f in r["new"]:
            fp = fingerprint(r["job"], f["category"], f["ident"])
            entry = by_fp.get(fp)
            if entry is None:
                if len(new_entries) >= MAX_NEW_FINDINGS:
                    continue
                entry = {
                    "ts": now,
                    "fp": fp,
                    "class": "ci-failure",
                    "category": f["category"],
                    "job": r["job"],
                    "line": f["display"][:300],
                    "status": "open",
                    "seen": 1,
                    "first_build": f["build"],
                    "last_build": f["build"],
                }
                by_fp[fp] = entry
                entries.append(entry)
                new_entries.append(entry)
            elif f["build"] > entry.get("last_build", 0):
                entry["seen"] = entry.get("seen", 1) + 1
                entry["last_build"] = f["build"]
                if entry not in new_entries:
                    bumped.append(entry)
        if r["builds_seen"]:
            cursor["jobs"][r["job"]] = max(
                cursor["jobs"].get(r["job"], 0), max(r["builds_seen"])
            )
        if r["green"]:
            cursor["green"][r["job"]] = max(cursor["green"].get(r["job"], 0), r["green"])
        # Consecutive-green streak per job (the COMPUTABLE disappearance
        # evidence — build-number arithmetic lies across aborted gaps) and
        # the rolling result window (pass/unstable/fail ratio = the
        # agentic-developer loop's FLOOR rail).
        streaks = cursor.setdefault("green_streak", {})
        recent = cursor.setdefault("recent", {})
        for _n, res in r.get("sequence", []):
            if res == "SUCCESS":
                streaks[r["job"]] = streaks.get(r["job"], 0) + 1
            else:
                streaks[r["job"]] = 0
            window = recent.setdefault(r["job"], [])
            window.append(res)
            del window[:-RECENT_WINDOW]

    # Deterministic lifecycle — no agent, no ceremony (spec §3.3):
    # confirmation-by-disappearance and recurrence-reopen are computable.
    confirmed, reopened, kept = [], [], []
    for e in entries:
        if e.get("class") == "ci-failure" and e.get("status") == "triaged":
            tab = e.get("triaged_at_build")
            if tab is not None and e.get("last_build", 0) > tab:
                e["status"] = "open"  # recurred after the fix — it didn't take
                reopened.append(e)
            elif (
                tab is not None
                and cursor.get("green_streak", {}).get(e.get("job", ""), 0) >= CONFIRM_STREAK
            ):
                confirmed.append(e)  # decomposed — ledger line not kept
                # Judgment was made ONCE at triage time: decompose_on_confirm
                # means no museum-worthy lesson — the backlog entry deletes
                # deterministically too. Otherwise the entry is reported for
                # graduate-then-decompose.
                bl = e.get("backlog")
                if e.get("decompose_on_confirm") and bl:
                    try:
                        os.remove(os.path.join(PROJECT, bl))
                    except OSError:
                        pass
                continue
        kept.append(e)
    entries = kept
    write_jsonl(LEDGER_PATH, entries)
    os.makedirs(os.path.dirname(CURSOR_PATH), exist_ok=True)
    tmp = CURSOR_PATH + ".tmp"
    with open(tmp, "w", encoding="utf-8") as fh:
        json.dump(cursor, fh, indent=1)
    os.replace(tmp, CURSOR_PATH)
    fcntl.flock(lock, fcntl.LOCK_UN)
    lock.close()
    return new_entries, bumped, confirmed, reopened


def render(results, new_entries, bumped, confirmed, reopened, as_hook):
    urgent = [r for r in results if r["urgent"]]
    parts, sys_parts = [], []
    if urgent:
        reds = "; ".join(
            f'{r["job"]}/dev #{r["urgent"]["build"]} {r["urgent"]["result"]} (was green)'
            for r in urgent
        )
        parts.append(
            f"[ci-harvest] FRESH REGRESSION on {reds} — this is now-work for the "
            f"session/operator (a recent push likely broke it), not a background-"
            f"triage item. Findings are ledgered for trail; investigate directly."
        )
        sys_parts.append(f"FRESH RED: {reds}")
    if new_entries:
        fps = ", ".join(e["fp"] for e in new_entries)
        lines = " | ".join(
            f'{e["fp"]} [{e["category"]}] {e["job"]}#{e["last_build"]}: "{e["line"][:90]}"'
            for e in new_entries[:5]
        )
        more = f" (+{len(new_entries) - 5} more)" if len(new_entries) > 5 else ""
        parts.append(
            f"[ci-harvest] {len(new_entries)} NEW CI finding(s) captured to "
            f".claude/data/ci-findings.jsonl — {lines}{more}. "
            f"DISPATCH (do not derail the current task): launch the "
            f"`ci-failure-triage` agent via the Agent tool with "
            f"run_in_background: true and the prompt 'Triage CI ledger "
            f"fingerprint(s) {fps} per your agent definition "
            f"(.claude/agents/ci-failure-triage.md). Your goal is the largest "
            f"genuine step toward stasis this run supports — canonicalize by "
            f"concern, land what is bounded, document live trajectories for "
            f"the rest.' Fall back to general-purpose with the same prompt if "
            f"the type is unavailable."
        )
        sys_parts.append(f"+{len(new_entries)} new finding(s) → ci-failure-triage dispatch")
    if bumped:
        sys_parts.append(f"{len(bumped)} known finding(s) recurred (flake evidence)")
    if reopened:
        fps = ", ".join(e["fp"] for e in reopened)
        parts.append(
            f"[ci-harvest] {len(reopened)} triaged fix(es) RECURRED — reopened: {fps}. "
            f"The fix didn't take; re-dispatch ci-failure-triage for these."
        )
        sys_parts.append(f"{len(reopened)} triaged finding(s) recurred → reopened")
    if confirmed:
        graduate = [e for e in confirmed if not e.get("decompose_on_confirm") and e.get("backlog")]
        sys_parts.append(
            f"{len(confirmed)} fix(es) CONFIRMED by disappearance (green streak) → decomposed"
        )
        if graduate:
            paths = ", ".join(e["backlog"] for e in graduate)
            parts.append(
                f"[ci-harvest] confirmed-fixed backlog entr(ies) awaiting graduation "
                f"judgment before decompose: {paths} — graduate the lesson to the "
                f"anti-patterns museum if worthy, then delete the entry."
            )
    if not parts and not sys_parts:
        return None
    if as_hook:
        return json.dumps(
            {
                "systemMessage": "ci-harvest: " + "; ".join(sys_parts),
                "hookSpecificOutput": {
                    "hookEventName": "SessionStart",
                    "additionalContext": " ".join(parts) if parts else "; ".join(sys_parts),
                },
            }
        )
    out = []
    if sys_parts:
        out.append("; ".join(sys_parts))
    out.extend(parts)
    return "\n".join(out)


def run_harvest(jobs, as_hook):
    cursor = load_cursor()
    taxonomy = load_taxonomy()
    with ThreadPoolExecutor(max_workers=6) as ex:
        results = list(ex.map(lambda j: harvest_job(j, cursor, taxonomy), jobs))
    new_entries, bumped, confirmed, reopened = reconcile(results, cursor)
    rendered = render(results, new_entries, bumped, confirmed, reopened, as_hook)
    if rendered:
        print(rendered)
    elif not as_hook:
        watermark = ", ".join(f'{r["job"]}@{r["last"]}' for r in results if r["last"])
        print(f"ci-harvest: nothing new ({watermark})")


def wait_mode(job, timeout_mins):
    """Post-push loop-closer: poll until the job's current build completes."""
    deadline = time.time() + timeout_mins * 60
    while time.time() < deadline:
        try:
            data = get_json(f"/job/{job}/job/{BRANCH}/lastBuild/api/json?tree=number,result,building")
            if not data.get("building") and data.get("result"):
                print(f'ci-harvest --wait: {job}/dev #{data["number"]} completed '
                      f'{data["result"]}; harvesting.')
                run_harvest([job], as_hook=False)
                return
        except (urllib.error.URLError, urllib.error.HTTPError, json.JSONDecodeError, OSError):
            pass
        time.sleep(60)
    print(f"ci-harvest --wait: timed out after {timeout_mins}m waiting on {job}/dev")


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--hook", action="store_true", help="SessionStart-hook JSON output")
    ap.add_argument("--wait", metavar="JOB", help="poll JOB until its build completes, then harvest")
    ap.add_argument("--timeout-mins", type=int, default=30)
    ap.add_argument("--jobs", help="comma-separated job subset")
    args = ap.parse_args()
    jobs = [j.strip() for j in args.jobs.split(",")] if args.jobs else JOBS
    if args.wait:
        wait_mode(args.wait, args.timeout_mins)
    else:
        run_harvest(jobs, as_hook=args.hook)


if __name__ == "__main__":
    if "--hook" in sys.argv:
        try:
            main()
        except BaseException:  # noqa: BLE001 — never break session start
            # (BaseException: argparse SystemExit included)
            pass
        sys.exit(0)
    main()
