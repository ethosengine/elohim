#!/usr/bin/env python3
"""
Plan-status dashboard for the Elohim Protocol dev memory.

Read-only scan of plans under:
  - genesis/docs/plans/*.md
  - genesis/docs/superpowers/plans/*.md

For each plan, classifies into:
  - active                — recently modified (<= 14 days) AND has open checkboxes
  - in-progress-cooling   — has open checkboxes BUT last-modified > 14 days
  - stalled               — open checkboxes AND date > 60 days AND last-modified > 30 days
  - complete              — all checkboxes done AND open count is 0
  - no-checkboxes         — plan has no checkbox structure

Output: .claude/memory-kit/<YYYY-MM-DD>/plan-status.md

Sibling tools live alongside in `.claude/scripts/memory-kit/`. This is the
read-only dashboard companion to /cleanup (which archives). It surfaces what
NOT to archive (active work) and what to refresh (stale-but-unfinished).

Spec reference: genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
PLAN_GLOBS = [
    "genesis/docs/plans/*.md",
    "genesis/docs/superpowers/plans/*.md",
]

# Classification thresholds (days)
ACTIVE_MTIME_DAYS = 14            # mtime within this window = "active"
COOLING_MTIME_DAYS = 14           # mtime older than this with open boxes = "cooling"
STALLED_DATE_DAYS = 60            # plan date older than this AND ...
STALLED_MTIME_DAYS = 30           # ... mtime older than this = "stalled"

TODAY = date.today()

CHECKBOX_OPEN_RE = re.compile(r"^([\t ]*[-*]\s*\[ \])\s*(.*)$", re.MULTILINE)
CHECKBOX_DONE_RE = re.compile(r"^([\t ]*[-*]\s*\[[xX]\])\s*(.*)$", re.MULTILINE)


@dataclass
class PlanRecord:
    path: Path
    classification: str
    mtime: date
    file_date: date | None
    fm_status: str | None
    open_count: int
    done_count: int
    open_items: list[str] = field(default_factory=list)  # text following "- [ ]"


def read_frontmatter(text: str) -> dict[str, str]:
    """Extract simple YAML frontmatter as a flat dict. Tolerates malformed."""
    if not text.startswith("---"):
        return {}
    end = text.find("\n---", 3)
    if end < 0:
        return {}
    body = text[3:end].strip()
    out = {}
    for line in body.splitlines():
        if ":" in line:
            k, _, v = line.partition(":")
            out[k.strip().lower()] = v.strip().strip("\"'")
    return out


def date_from_filename(filename: str) -> date | None:
    """Extract a YYYY-MM-DD prefix from a filename if present."""
    m = re.match(r"^(\d{4})-(\d{2})-(\d{2})", filename)
    if not m:
        return None
    try:
        return date(int(m.group(1)), int(m.group(2)), int(m.group(3)))
    except ValueError:
        return None


def date_from_frontmatter(fm: dict[str, str]) -> date | None:
    raw = fm.get("date", "")
    # Accept YYYY-MM-DD at start of value
    m = re.match(r"^(\d{4})-(\d{2})-(\d{2})", raw)
    if not m:
        return None
    try:
        return date(int(m.group(1)), int(m.group(2)), int(m.group(3)))
    except ValueError:
        return None


def collect_open_items(text: str, limit: int = 25) -> list[str]:
    """Return text following each `- [ ]` checkbox, up to limit entries."""
    items = []
    for m in CHECKBOX_OPEN_RE.finditer(text):
        body = m.group(2).strip()
        # Compress whitespace and trim long lines
        body = re.sub(r"\s+", " ", body)
        if len(body) > 200:
            body = body[:197] + "..."
        items.append(body if body else "(empty checkbox)")
        if len(items) >= limit:
            break
    return items


def classify(
    open_count: int,
    done_count: int,
    file_date: date | None,
    mtime: date,
) -> str:
    if open_count == 0 and done_count == 0:
        return "no-checkboxes"
    if open_count == 0 and done_count > 0:
        return "complete"

    # open_count > 0 here
    mtime_age = (TODAY - mtime).days
    file_age = (TODAY - file_date).days if file_date else None

    # Stalled: very old plan, very cold mtime, with open work
    if (
        file_age is not None
        and file_age > STALLED_DATE_DAYS
        and mtime_age > STALLED_MTIME_DAYS
    ):
        return "stalled"

    if mtime_age > COOLING_MTIME_DAYS:
        return "in-progress-cooling"

    return "active"


def scan_plan(path: Path) -> PlanRecord | None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None

    fm = read_frontmatter(text)
    fm_status = fm.get("status") or None

    file_date = date_from_filename(path.name) or date_from_frontmatter(fm)

    try:
        mtime_ts = path.stat().st_mtime
    except OSError:
        return None
    mtime = datetime.fromtimestamp(mtime_ts).date()

    open_count = len(CHECKBOX_OPEN_RE.findall(text))
    done_count = len(CHECKBOX_DONE_RE.findall(text))

    classification = classify(open_count, done_count, file_date, mtime)

    open_items: list[str] = []
    if classification in ("in-progress-cooling", "stalled"):
        open_items = collect_open_items(text)

    return PlanRecord(
        path=path,
        classification=classification,
        mtime=mtime,
        file_date=file_date,
        fm_status=fm_status,
        open_count=open_count,
        done_count=done_count,
        open_items=open_items,
    )


def gather_plans() -> list[Path]:
    plans: list[Path] = []
    for pattern in PLAN_GLOBS:
        plans.extend(sorted(REPO_ROOT.glob(pattern)))
    return plans


def fmt_age(d: date | None) -> str:
    if d is None:
        return "unknown"
    days = (TODAY - d).days
    if days == 0:
        return "today"
    if days == 1:
        return "1 day ago"
    return f"{days} days ago"


def render_record(rec: PlanRecord) -> list[str]:
    rel = (
        rec.path.relative_to(REPO_ROOT)
        if rec.path.is_relative_to(REPO_ROOT)
        else rec.path
    )
    lines: list[str] = []
    lines.append(f"### `{rel}`")
    lines.append("")
    parts = []
    parts.append(f"**Plan date**: {rec.file_date.isoformat() if rec.file_date else 'unknown'} ({fmt_age(rec.file_date)})")
    parts.append(f"**Last modified**: {rec.mtime.isoformat()} ({fmt_age(rec.mtime)})")
    if rec.fm_status:
        parts.append(f"**Frontmatter status**: `{rec.fm_status}`")
    parts.append(f"**Checkboxes**: {rec.done_count} done / {rec.open_count} open")
    lines.append(" · ".join(parts))
    lines.append("")

    if rec.classification == "complete":
        lines.append("**Suggested action**: consider archive via `/cleanup` (this tool will not archive).")
        lines.append("")
    elif rec.classification == "stalled":
        lines.append("**Suggested action**: revisit, decide — finish, reframe, or close-interval (mark superseded).")
        lines.append("")
    elif rec.classification == "in-progress-cooling":
        lines.append("**Suggested action**: pull back into the active sprint, or accept it's cooling and let it stall.")
        lines.append("")
    elif rec.classification == "active":
        lines.append("**Suggested action**: keep going — this is current work.")
        lines.append("")
    elif rec.classification == "no-checkboxes":
        lines.append("**Suggested action**: narrative-style plan; no automated progress signal. Manual review only.")
        lines.append("")

    if rec.open_items:
        lines.append("**Open items** (unfinished work — this is the gold):")
        lines.append("")
        for item in rec.open_items:
            lines.append(f"- {item}")
        if rec.open_count > len(rec.open_items):
            lines.append(f"- ... and {rec.open_count - len(rec.open_items)} more")
        lines.append("")

    lines.append("---")
    lines.append("")
    return lines


def render_report(records: list[PlanRecord], include_complete: bool) -> str:
    # Group
    by_class: dict[str, list[PlanRecord]] = {
        "active": [],
        "in-progress-cooling": [],
        "stalled": [],
        "complete": [],
        "no-checkboxes": [],
    }
    for rec in records:
        by_class[rec.classification].append(rec)

    # Sort: active = most recent mtime first; cooling = oldest mtime first;
    # stalled = oldest mtime first; complete = most recent mtime first;
    # no-checkboxes = most recent mtime first.
    by_class["active"].sort(key=lambda r: r.mtime, reverse=True)
    by_class["in-progress-cooling"].sort(key=lambda r: r.mtime)
    by_class["stalled"].sort(key=lambda r: r.mtime)
    by_class["complete"].sort(key=lambda r: r.mtime, reverse=True)
    by_class["no-checkboxes"].sort(key=lambda r: r.mtime, reverse=True)

    out: list[str] = []
    out.append(f"# Plan Status — {TODAY.isoformat()}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/plan-status.py`. Read-only "
        "dashboard of plans under `genesis/docs/plans/` and "
        "`genesis/docs/superpowers/plans/`."
    )
    out.append("")
    out.append(
        f"**Total plans scanned**: {len(records)} — "
        f"active: {len(by_class['active'])}, "
        f"in-progress-cooling: {len(by_class['in-progress-cooling'])}, "
        f"stalled: {len(by_class['stalled'])}, "
        f"complete: {len(by_class['complete'])}, "
        f"no-checkboxes: {len(by_class['no-checkboxes'])}"
    )
    out.append("")
    out.append(
        "Classification rules: **active** = mtime within last "
        f"{ACTIVE_MTIME_DAYS} days AND open checkboxes; "
        f"**in-progress-cooling** = open checkboxes AND mtime > {COOLING_MTIME_DAYS} days; "
        f"**stalled** = open checkboxes AND plan date > {STALLED_DATE_DAYS} days AND mtime > {STALLED_MTIME_DAYS} days; "
        "**complete** = all checkboxes done; **no-checkboxes** = narrative-style plan with no checkbox structure."
    )
    out.append("")
    out.append("---")
    out.append("")

    sections: list[tuple[str, str, str]] = [
        (
            "active",
            "Active",
            "Currently being worked on. Modified within the last "
            f"{ACTIVE_MTIME_DAYS} days and still has open items.",
        ),
        (
            "in-progress-cooling",
            "In-Progress Cooling",
            "Has open checkboxes but hasn't been touched in a while. "
            "Decide: re-engage, or let it stall.",
        ),
        (
            "stalled",
            "Stalled",
            f"Old plan ({STALLED_DATE_DAYS}+ days) that's gone cold "
            f"({STALLED_MTIME_DAYS}+ days untouched) with open work remaining. "
            "Most-stalled first — these are the worst-staleness candidates.",
        ),
        (
            "complete",
            "Complete",
            "All checkboxes are done. Consider archiving via `/cleanup` "
            "(this tool will not archive).",
        ),
        (
            "no-checkboxes",
            "No Checkboxes",
            "Narrative-style plans without checkbox progress signal. "
            "Listed for completeness; needs manual review to assess.",
        ),
    ]

    for key, title, blurb in sections:
        if key == "complete" and not include_complete:
            continue
        recs = by_class[key]
        out.append(f"## {title} ({len(recs)})")
        out.append("")
        out.append(blurb)
        out.append("")
        if not recs:
            out.append("_(none)_")
            out.append("")
        else:
            for rec in recs:
                out.extend(render_record(rec))

    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Plan status dashboard — read-only scan of dev plans.",
    )
    parser.add_argument(
        "--include-complete",
        dest="include_complete",
        action="store_true",
        default=True,
        help="Include completed plans in the report (default: true).",
    )
    parser.add_argument(
        "--no-include-complete",
        dest="include_complete",
        action="store_false",
        help="Hide completed plans (TO-DO-only view).",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=None,
        help="Override output directory (default: .claude/memory-kit/<YYYY-MM-DD>/).",
    )
    args = parser.parse_args()

    out_dir = args.output_dir or (
        REPO_ROOT / ".claude" / "memory-kit" / TODAY.isoformat()
    )
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "plan-status.md"

    plan_paths = gather_plans()
    records: list[PlanRecord] = []
    for path in plan_paths:
        rec = scan_plan(path)
        if rec:
            records.append(rec)

    report = render_report(records, include_complete=args.include_complete)
    out_path.write_text(report, encoding="utf-8")

    counts: dict[str, int] = {
        "active": 0,
        "in-progress-cooling": 0,
        "stalled": 0,
        "complete": 0,
        "no-checkboxes": 0,
    }
    for r in records:
        counts[r.classification] += 1

    print(
        f"scanned: {len(records)} plans across {len(PLAN_GLOBS)} directories"
    )
    print(
        f"  active: {counts['active']}, "
        f"in-progress-cooling: {counts['in-progress-cooling']}, "
        f"stalled: {counts['stalled']}, "
        f"complete: {counts['complete']}, "
        f"no-checkboxes: {counts['no-checkboxes']}"
    )
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
