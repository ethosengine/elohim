#!/usr/bin/env python3
"""habits-status.py — emit the delivery-habits headline from genesis/manifests/habits.yaml.

The habits register is the inversion-of-control seam for sessions: each habit is a
runnable contract (an interface); sessions implement "make the top red green". This
script turns it into the one line every session opens with, so session start is
SELECTION (top red) instead of SYNTHESIS (read 240 specs).

WHY "HABITS" (2026-08-06, operator directive). The manifesto already quotes James Clear:
"You don't rise to the level of your goals, you fall to the level of your systems." This
file is that level — not what we intend (intentions live in specs and plans) but what the
system is observed to do, with evidence. A habit is proven by repetition, never by
declaration, which is the discipline this register exists to enforce. Earlier names failed
on shape and namespace: "spine" implied a sequence this register does not have (the ordered
structure is the resiliency-saga), and "charter" collided with the shipped qahal collective
`charter` wire field.

NOT MEASURED — THE HONESTY LEG (2026-08-22, Law I of the verification guidestar,
genesis/docs/superpowers/specs/2026-08-22-verification-as-memoized-derivation-guidestar.md).
A declared habit whose evidence lane measured NOTHING must not read green. This script
reads the newest a2o sprint report, derives what the lane actually observed, and RENDERS
THE DISAGREEMENT beside the declared status — `observed_status:` and `last_measured:`
BESIDE the human-owned `status:`, never instead of it.

**THIS SCRIPT NEVER WRITES `status:` IN habits.yaml — IT NEVER WRITES habits.yaml AT ALL.**
That is Law I, not an implementation detail: a measured absence is rendered, and the flip
is the operator's (covenant rule 4). There is no code path here that opens the register
for writing; `_lib/__tests__/habits_status_not_measured_test.py` asserts its bytes are
unchanged across a full render.

The denominator is declared OUTSIDE the run: a habit's concerns come from its own
`checks:` strings here, NOT from the set of concerns a run happened to touch. A
tag-scoped or path-filtered run therefore cannot make missing coverage vanish — the
unexercised declared concerns still render NOT MEASURED. Absence of any report at all is
itself NOT MEASURED, never green.

Usage:
  habits-status.py --headline   one line for the SessionStart context block
  habits-status.py --full       table: every habit, status, checks, first moves

Deterministic, read-only, no network. Statuses are DECLARED in habits.yaml (covenant rule
4: flips require evidence); this script renders, never infers.
"""
import json
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

try:
    import yaml
except ImportError:
    sys.exit(0)  # never break session start over a dependency

_ROOT = Path(__file__).resolve().parents[2]
HABITS = _ROOT / "genesis" / "manifests" / "habits.yaml"

# The three concern-addressed findings ledgers a habit's pain can live in:
# CI (ci-harvest), runtime self-heal-exhaustion (runtime-harvest), and the
# local devspace/architecture plane (epr-meta measure mint). Tests override
# this list wholesale (`m.LEDGERS = [fixture_path]`) rather than patching
# individual entries.
LEDGERS = [
    _ROOT / ".claude" / "data" / "ci-findings.jsonl",
    _ROOT / ".claude" / "data" / "runtime-findings.jsonl",
    _ROOT / ".claude" / "data" / "architecture-findings.jsonl",
]

# Inline copy of concern_routes.py's tag regex — habits-status is a
# session-start surface and stays import-free of other _lib modules.
_CONCERN_TAG = re.compile(r"@concern:([a-z0-9][a-z0-9-]*)")


def open_pain() -> dict:
    """concern -> [fps] over OPEN, concern-addressed entries across LEDGERS.

    Entries without a truthy `concern` are skipped silently (never counted,
    never a crash). A missing, unreadable, or malformed ledger — or an
    individual malformed line — contributes nothing; this must never raise
    at session start.
    """
    pain: dict = {}
    for ledger in LEDGERS:
        try:
            ledger = Path(ledger)
            if not ledger.exists():
                continue
            with ledger.open(encoding="utf-8") as fh:
                for raw in fh:
                    raw = raw.strip()
                    if not raw:
                        continue
                    try:
                        entry = json.loads(raw)
                    except json.JSONDecodeError:
                        continue
                    if not isinstance(entry, dict):
                        continue
                    if entry.get("status") != "open":
                        continue
                    concern = entry.get("concern")
                    # Honest absence extends to malformed addresses: a
                    # non-string concern (e.g. a list) is unhashable and
                    # cannot be a dict key — skip rather than crash.
                    if not isinstance(concern, str) or not concern:
                        continue
                    fp = entry.get("fp")
                    # A non-string fp (e.g. an int) is valid JSON but would
                    # crash ", ".join(...) in full() — coerce to the same
                    # honest placeholder used for a missing fp.
                    if not isinstance(fp, str) or not fp:
                        fp = "?"
                    pain.setdefault(concern, []).append(fp)
        except (OSError, ValueError, TypeError):
            # OSError: missing/unreadable ledger. ValueError covers
            # UnicodeDecodeError (raised mid-iteration on a non-UTF-8 file,
            # not caught by OSError alone). TypeError is a defensive
            # catch-all for any other hostile-shape surprise. Any of these
            # degrade this ledger to an empty contribution — never a crash
            # at session start.
            continue
    return pain


def _concerns(checks) -> list:
    """EVERY @concern: tag across a habit's check strings, in order, deduped.

    Law I's denominator, read from the register and not from a run: a habit that
    declares six concerns is measured only when all six are. `_first_concern` (the
    findings-ledger pain join, which addresses one concern) reads the first of these.
    """
    out = []
    for c in checks or []:
        for name in _CONCERN_TAG.findall(str(c)):
            if name not in out:
                out.append(name)
    return out


def _first_concern(checks):
    """First @concern: tag among a habit's check strings, or None."""
    got = _concerns(checks)
    return got[0] if got else None


# ----------------------------------------------------------------- NOT MEASURED (Law I)
# The a2o sprint reports are the evidence lane a habit's `checks:` name. Both lanes match:
# run-identified names (`sprint-report-<lane>-<runId>.json`) and the legacy single mutable
# slot (`sprint-report-<lane>.json`). Bare `sprint-report.json` (no lane) does not — it is
# an April stub, not a lane. Tests override REPORTS_DIR wholesale.
REPORTS_DIR = _ROOT / "genesis" / "a2o" / "reports"
REPORT_GLOB = "sprint-report-*-*.json"
# The legacy single-slot name the fleet lane still writes today
# (scripts/ci/run-dataplane-validation.sh). Kept as a SECOND glob rather than widening the
# first, so the run-identified shape is the one the code is written around.
REPORT_GLOB_LEGACY = "sprint-report-*.json"
MAX_REPORTS = 60  # newest-first cap — session start stays cheap, never scans a decade

# DOCUMENTED DEFAULT. A habit that declares no `max_evidence_age:` is held to 14d: two
# weeks with no verdict at all is stale in every lane we run (the fleet lane runs on
# every edge build, the household lane several times an evening). Declare a shorter age
# on a habit whose lane runs nightly; declare a longer one on a seasonal lane; declare
# `max_evidence_age: none` to opt a habit out of the clock entirely. The default exists
# so this leg works on all 12 habits with ZERO annotation — an honesty feature that costs
# the operator an evening of YAML before it fires is not an honesty feature.
DEFAULT_MAX_EVIDENCE_AGE = "14d"

NOT_MEASURED = "not-measured"
_AGE_UNITS = {"h": 3600.0, "d": 86400.0, "w": 604800.0}
_AGE_RE = re.compile(r"^(\d+(?:\.\d+)?)\s*([hdw])?$")
# jenkins-elohim-<job>-<branch>-<n>  ->  "<job> #<n>"  (jenkins-elohim-edge-dev-1376)
_RUN_ID_RE = re.compile(r"^jenkins-elohim-([a-z0-9]+)-.*?-(\d+)$")


def _parse_age(value):
    """'14d' | '36h' | '2w' | 14 (bare number = days) -> seconds. None = no clock.

    An unparseable or `none`/`never` value disables the clock rather than inventing one:
    a malformed annotation must not silently demote a habit nobody meant to demote.
    """
    if value is None or isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value) * 86400.0 if value > 0 else None
    m = _AGE_RE.match(str(value).strip().lower())
    if not m:
        return None
    return float(m.group(1)) * _AGE_UNITS.get(m.group(2) or "d", 86400.0)


def _report_time(doc: dict, path):
    """The report's own `generatedAt`, else the file mtime, else None (unknown)."""
    raw = doc.get("generatedAt")
    if isinstance(raw, str) and raw:
        try:
            when = datetime.fromisoformat(raw.replace("Z", "+00:00"))
            return when if when.tzinfo else when.replace(tzinfo=timezone.utc)
        except ValueError:
            pass
    try:
        return datetime.fromtimestamp(Path(path).stat().st_mtime, tz=timezone.utc)
    except (OSError, ValueError, OverflowError):
        return None


def _mtime(path) -> float:
    try:
        return path.stat().st_mtime
    except OSError:
        return 0.0


def load_reports() -> list:
    """Newest-first a2o sprint reports, as {path, when, runId, profile, byConcern, scenarios}.

    Never raises: a missing directory, an unreadable file, a non-dict body or a malformed
    `summary` each contribute nothing. AN EMPTY RESULT IS ITSELF NOT MEASURED — callers
    render that, and must never read it as green.
    """
    try:
        base = Path(REPORTS_DIR)
        found = set(base.glob(REPORT_GLOB)) | set(base.glob(REPORT_GLOB_LEGACY))
        # Pre-rank by mtime, NOT alphabetically: run-identified names accumulate, and an
        # alphabetical truncation would drop the newest run once the directory outgrows
        # the cap. mtime only orders the candidate set — `generatedAt` still decides.
        paths = sorted(found, key=lambda q: _mtime(q), reverse=True)[:MAX_REPORTS * 4]
    except (OSError, ValueError):
        return []
    out = []
    for path in paths:
        try:
            doc = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, ValueError, TypeError):
            continue  # unreadable / not JSON / not UTF-8 — contributes nothing
        if not isinstance(doc, dict):
            continue
        summary = doc.get("summary")
        summary = summary if isinstance(summary, dict) else {}
        by = summary.get("byConcern")
        scenarios = summary.get("scenarios")
        out.append({
            "path": path,
            "when": _report_time(doc, path),
            "runId": doc.get("runId") if isinstance(doc.get("runId"), str) else path.stem,
            "profile": doc.get("profile") if isinstance(doc.get("profile"), str) else "?",
            "byConcern": by if isinstance(by, dict) else {},
            "scenarios": scenarios if isinstance(scenarios, dict) else {},
        })
    _floor = datetime.min.replace(tzinfo=timezone.utc)
    out.sort(key=lambda r: (r["when"] is not None, r["when"] or _floor), reverse=True)
    return out[:MAX_REPORTS]


def observe_concern(rollup) -> str:
    """green | red | not-measured for ONE concern in ONE report.

    any failed -> red. passed>0 with no failure -> green. passed==0 — whether the
    scenarios were pending/skipped, or the concern is absent from `byConcern` entirely
    (rollup is None) — -> NOT MEASURED. An all-zero rollup is an apparatus that ran
    nothing, never a pass: that conflation is what let 76-of-80 skips read as green.
    """
    if not isinstance(rollup, dict):
        return NOT_MEASURED

    def n(key):
        v = rollup.get(key)
        return v if isinstance(v, int) and not isinstance(v, bool) else 0

    if n("failed") > 0:
        return "red"
    if n("passed") > 0:
        return "green"
    return NOT_MEASURED


def observe_habit(habit: dict, report) -> tuple:
    """(observed, unwitnessed_concerns) for one habit against ONE report.

    observed is None when the habit declares no @concern-tagged check — this lane cannot
    witness it, and claiming otherwise would be the same over-claim in reverse.
    A red concern outranks an unwitnessed one: a failure is a measurement.
    """
    concerns = _concerns(habit.get("checks"))
    if not concerns:
        return (None, [])
    by = (report or {}).get("byConcern") or {}
    seen = [(c, observe_concern(by.get(c))) for c in concerns]
    missing = [c for c, o in seen if o == NOT_MEASURED]
    if any(o == "red" for _, o in seen):
        return ("red", missing)
    if missing:
        return (NOT_MEASURED, missing)
    return ("green", [])


def _age(when, now) -> str:
    if when is None:
        return "never"
    secs = (now - when).total_seconds()
    if secs < 0:
        return "just now"
    if secs >= 86400:
        return f"{int(secs // 86400)}d ago"
    if secs >= 3600:
        return f"{int(secs // 3600)}h ago"
    return f"{int(secs // 60)}m ago"


def _run_label(report) -> str:
    """Name the run that produced (or failed to produce) the evidence."""
    if report is None:
        return "no sprint report on disk — the lane left no durable trace"
    rid = str(report.get("runId") or "?")
    m = _RUN_ID_RE.match(rid)
    # Law II: the lane is part of the key. A 1-scenario household smoke run and an
    # 80-scenario fleet run are different environments; naming the profile keeps the
    # reader from reading one as the other.
    label = f"{report.get('profile') or '?'} · " + (f"{m.group(1)} #{m.group(2)}" if m else rid)
    sc = report.get("scenarios") or {}

    def n(key):
        v = sc.get(key)
        return v if isinstance(v, int) and not isinstance(v, bool) else 0

    total, unrun = n("total"), n("skipped") + n("pending")
    if total and unrun:
        return f"{label}: {unrun} of {total} scenarios never ran"
    return label


def not_measured_view(doc: dict, now=None, reports=None) -> dict:
    """Render-ready observation of every habit against the newest sprint report.

    PURE OF THE REGISTER: reads habits.yaml's parsed doc and the report files, writes
    nothing. `last_measured` is the newest report that produced a REAL verdict for the
    habit (green or red) — not merely one that mentioned it — because that is the clock
    `max_evidence_age` must be measured against.
    """
    reports = load_reports() if reports is None else list(reports)
    newest = reports[0] if reports else None
    now = now or datetime.now(timezone.utc)

    rows, no_concern = [], []
    for habit in habits_of(doc):
        concerns = _concerns(habit.get("checks"))
        if not concerns:
            no_concern.append(habit.get("id"))
            continue
        observed, missing = observe_habit(habit, newest)

        last = None
        for report in reports:
            verdict, _ = observe_habit(habit, report)
            if verdict in ("green", "red"):
                last = report
                break

        declared_age = habit.get("max_evidence_age", DEFAULT_MAX_EVIDENCE_AGE)
        limit = _parse_age(declared_age)
        stale, undated = False, False
        if observed in ("green", "red") and limit is not None:
            when = last.get("when") if last else None
            if when is None:
                # A verdict exists but cannot be DATED (no parseable `generatedAt`, no
                # readable mtime). An undateable witness cannot satisfy an age clock —
                # demote and say so, rather than crediting evidence of unknown vintage.
                stale, undated, observed = True, True, NOT_MEASURED
            elif (now - when).total_seconds() > limit:
                stale, observed = True, NOT_MEASURED

        declared = habit.get("status")
        rows.append({
            "id": habit.get("id"),
            "declared": declared if isinstance(declared, str) else "?",
            "observed": observed,
            "concerns": concerns,
            "missing": missing,
            "stale": stale,
            "undated": undated,
            "max_evidence_age": str(declared_age),
            "last_measured": last.get("when") if last else None,
            "age": _age(last.get("when") if last else None, now),
            # The disagreement worth a session's attention: the register claims MORE than
            # the evidence (green with no witness / green with a failure), or LESS (a red
            # the lane now passes — a flip candidate, still the operator's to make).
            "disagrees": (declared == "green" and observed != "green")
            or (declared in ("red", "unwired") and observed == "green"),
        })

    not_measured = [r for r in rows if r["observed"] == NOT_MEASURED]
    return {
        "rows": rows,
        "not_measured": not_measured,
        "disagreements": [r for r in rows if r["disagrees"]],
        "concern_count": sum(len(r["missing"]) for r in not_measured),
        "no_concern": no_concern,
        "newest": newest,
        "run_label": _run_label(newest),
        "reports": reports,
    }


def not_measured_lines(view: dict) -> list:
    """The headline block: the count, the run that produced it, then the disagreeing habits.

    Rendered ABOVE the next-move line, because a green habit with no witness invalidates
    the selection the next-move line is about to make.
    """
    if not view["not_measured"]:
        return []
    shown = view["disagreements"] or []
    head = (f"  NOT MEASURED  {len(view['not_measured'])} habits · "
            f"{view['concern_count']} declared concerns ({view['run_label']})")
    if shown:
        head += "  ← read this first"
    lines = [head]
    for r in shown:
        lines.append(
            f"    {r['id']:<34} status: {r['declared']:<7} "
            f"observed_status: {r['observed']:<13} last_measured: {r['age']}")
    rest = len(view["not_measured"]) - len(shown)
    if rest:
        lines.append(f"    +{rest} more not measured, already declared red/unwired "
                     f"(habits-status.py --full)")
    if shown:
        lines.append("  → the register and the evidence disagree. Covenant rule 4: the "
                     "status flip is YOURS, this tool never writes it.")
    return lines


def load():
    if not HABITS.exists():
        return None
    try:
        return yaml.safe_load(HABITS.read_text(encoding="utf-8"))
    except yaml.YAMLError as e:
        print(f"HABITS  ⚠ unparseable ({e}) — fix {HABITS}", file=sys.stdout)
        return None


def habits_of(doc: dict) -> list:
    """The declared habits."""
    return doc.get("habits") or []


def headline(habits: dict) -> str:
    arts = habits_of(habits)
    greens = [n for n in arts if n.get("status") == "green"]
    reds = [n for n in arts if n.get("status") == "red"]
    unwired = [n for n in arts if n.get("status") == "unwired"]
    active = [n for n in arts if n.get("active")]

    # Top red: an active red beats an inactive red beats an active unwired.
    top = next((n for n in reds if n.get("active")), None) \
        or (reds[0] if reds else None) \
        or next((n for n in unwired if n.get("active")), None)
    if top is None:
        nxt = "all green — pull the next habit from the covenant queue"
    elif top.get("status") == "red":
        check = (top.get("checks") or ["(check missing)"])[0]
        nxt = f"top red: {top['id']} → {check}"
        concern = _first_concern(top.get("checks"))
        if concern:
            fps = open_pain().get(concern) or []
            if fps:
                nxt += f" · pain: {len(fps)} open @{concern}"
    else:
        nxt = f"top move: {top['id']} is unwired → write its red ({(top.get('first_move') or '').strip().split(':')[0]}…)"

    counts = f"{len(greens)} green · {len(reds)} red · {len(unwired)} unwired"
    fence = ",".join(n["id"] for n in active) or "none"
    # Law I: the honesty leg renders ABOVE the next-move line. A green habit whose lane
    # measured nothing invalidates the selection that line is about to make, so it cannot
    # sit below it. Guarded: this must never break session start (a hostile report body,
    # an unreadable reports dir), and a failure degrades to printing no block at all —
    # which is the pre-2026-08-22 behaviour, not a false green.
    try:
        nm_lines = not_measured_lines(not_measured_view(habits))
    except Exception:
        nm_lines = []
    body = ("\n".join(nm_lines) + "\n") if nm_lines else ""
    return (
        f"HABITS ({HABITS.name})  {counts} · active: {fence}\n"
        f"{body}"
        f"  {nxt}\n"
        f"  RULE: sessions serve the habits — move reds green (with evidence), file new reds runnable, "
        f"one-line delta. A plan citing no habit belongs in held/."
    )


def full(habits: dict) -> str:
    lines = [f"Delivery habits — {HABITS} (updated {habits.get('updated')})", ""]
    pain = open_pain()
    try:
        view = not_measured_view(habits)
    except Exception:
        view = None
    if view:
        nm_lines = not_measured_lines(view)
        if nm_lines:
            lines.extend(nm_lines)
        else:
            lines.append(f"  every declared concern was measured ({view['run_label']})")
        lines.append("")
    observed = {r["id"]: r for r in (view["rows"] if view else [])}
    for n in habits_of(habits):
        mark = {"green": "✅", "red": "🔴", "unwired": "▫️"}.get(n.get("status"), "?")
        act = "  [ACTIVE]" if n.get("active") else ""
        lines.append(f"{mark} {n['id']}{act}")
        lines.append(f"    {' '.join(str(n.get('invariant', '')).split())}")
        for c in n.get("checks", []) or []:
            lines.append(f"    check: {c}")
        if n.get("first_move"):
            lines.append(f"    first move: {' '.join(str(n['first_move']).split())}")
        if n.get("evidence"):
            lines.append(f"    evidence: {' '.join(str(n['evidence']).split())}")
        concern = _first_concern(n.get("checks"))
        fps = pain.get(concern) if concern else None
        if fps:
            shown = ", ".join(fps[:3]) + ("…" if len(fps) > 3 else "")
            lines.append(f"    pain: {len(fps)} open ({shown})")
        row = observed.get(n.get("id"))
        if row:
            obs = (f"    observed_status: {row['observed']}"
                   f"  ·  last_measured: {row['age']}"
                   f"  ·  max_evidence_age: {row['max_evidence_age']}")
            if row["undated"]:
                obs += "  ·  UNDATED (a verdict exists but cannot be dated — no clock)"
            elif row["stale"]:
                obs += "  ·  STALE (a verdict exists, but older than the declared age)"
            lines.append(obs)
            if row["missing"]:
                lines.append(f"    no witness for: {', '.join(row['missing'])}")
            if row["disagrees"]:
                lines.append("    ⚠ the register and the evidence DISAGREE — covenant rule 4: "
                             "the flip is the operator's, this tool never writes it")
        elif view is not None:
            lines.append("    observed_status: — (no @concern-tagged check; the a2o lane "
                         "cannot witness this habit)")
        lines.append("")
    return "\n".join(lines)


def main() -> int:
    try:
        return _render()
    except BrokenPipeError:
        # `habits-status.py --full | head -40` is the documented way to read this table,
        # and `head` closes the pipe mid-write. Python turns that into an exception and a
        # traceback on stderr — noise on a surface whose whole job is a clean render.
        # (Pre-existing: the --full body has exceeded the 64KB pipe buffer since well
        # before the NOT MEASURED block; it is fixed here because this file owns it.)
        # Redirect stdout to /dev/null so nothing can re-raise, then leave via os._exit
        # so the interpreter never runs its own shutdown flush of the half-written
        # buffer (that flush is what turns this into exit 120 even after the redirect).
        # A consumer that stopped reading is a normal, successful end: exit 0.
        try:
            os.dup2(os.open(os.devnull, os.O_WRONLY), sys.stdout.fileno())
        except OSError:
            pass
        os._exit(0)


def _render() -> int:
    habits = load()
    if not habits:
        return 0
    if "--full" in sys.argv:
        print(full(habits))
    else:
        print(headline(habits))
        # Guarded import (operator resolution, 2026-08-11): habits-status is a
        # session-start surface and must NEVER break on this — doc_dynamics
        # shells out to git, and git can fail, be absent, hang, or return
        # garbage. Import, computation, AND print all live inside one
        # try/except Exception; any failure degrades to printing no ratio
        # line at all, never a traceback at session start.
        try:
            from _lib.doc_dynamics import corpus_dynamics
            # Slice 2 (C): the WINDOW is part of the measure, not an argument default — in the
            # first live run the 28d default ALONE decided whether this line read `unknown` or
            # `3.2`. Absorption here is bursty, so several windows are reported and the shortest
            # BOUNDED one carries the headline: an unbounded shortest window is honest absence,
            # not a finding, and letting it be the whole line hides a real overshoot behind it.
            windows = corpus_dynamics()
            for line in _dynamics_lines(windows):
                print(line)
        except Exception:
            pass
    return 0


def _dynamics_lines(windows: list) -> list:
    """Render the doc-corpus stock: the index at the shortest bounded window, its turnover, and
    which windows were unbounded. Pure — takes the measured stocks, returns strings."""
    def flag(iv):
        # lo > 1.0 means overshoot at EVERY plausible absorption rate, not just the central one.
        return "⚠" if iv["lo"] > 1.0 else ("~" if iv["hi"] > 1.0 else "✅")

    def unknown(iv):
        return iv["lo"] == float("-inf") and iv["hi"] == float("inf")

    unbounded = [s for s in windows if unknown(s["emission_absorption"]["confidence"]["interval"])]
    bounded = [s for s in windows
               if not unknown(s["emission_absorption"]["confidence"]["interval"])]
    out = []
    if bounded:
        s = bounded[0]
        idx, iv = s["emission_absorption"], s["emission_absorption"]["confidence"]["interval"]
        c, turn = s["counts"], s["turnover_time"]
        out.append(
            f"  doc dynamics: emission/absorption {idx['value']:.2f} "
            f"[{iv['lo']:.2f}–{iv['hi']:.2f}] {flag(iv)} "
            f"({s['window_days']}d: {c['generated']} authored / {c['absorbed']} absorbed, "
            f"level {s['level']['value']:.0f})")
        if turn["value"] is not None:
            ti = turn["confidence"]["interval"]
            out.append(
                f"    turnover: {turn['value']:.0f}wk [{ti['lo']:.0f}–{ti['hi']:.0f}] — "
                f"the measure that separates dynamic equilibrium from silting")
    if unbounded:
        spans = ", ".join(f"{s['window_days']}d" for s in unbounded)
        out.append(f"    unbounded at {spans}: zero absorption counted — honest absence, "
                   f"not a zero")
    return out


if __name__ == "__main__":
    sys.exit(main())
