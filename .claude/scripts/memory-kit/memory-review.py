#!/usr/bin/env python3
"""
Memory-side health diagnostic — read-only scan over the project's auto-memory
directory (the .claude-config-managed complement to Claude's native memory).

Five sections:
  A. MEMORY.md size vs the 200-line budget (the per-Pawel article ceiling).
  B. Topic-file count grouped by type prefix (feedback_/project_/reference_/user_).
  C. Growth — entries added in last 7d / 30d (filesystem ctime, not git).
  D. Stale candidates — topic files untouched > 90 days.
  E. Index-vs-files drift — entries in MEMORY.md missing backing files;
     backing files missing from MEMORY.md.

This is the explicit "is memory healthy?" surface the original kit lacked.
Read-only. No mutations. No LLM. No network.

Output: .claude/memory-kit/<YYYY-MM-DD>/memory-review.md
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta, timezone
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OUT_ROOT = REPO_ROOT / ".claude" / "memory-kit"
TODAY = date.today()
NOW = datetime.now(timezone.utc)

# Memory storage — two-tier model (2026-05-13):
#   Primary: <repo>/.claude/memory/ (team-shareable, git-tracked, PVC-recoverable)
#   Fallback: /projects/.claude-config/projects/<slug>/memory/ (personal, harness slot)
# The personal slot is typically a symlink to the primary; both paths resolve
# to the same files. We check the primary first to be explicit about intent.
MEMORY_BUDGET_LINES = 200
STALE_DAYS = 90
RECENT_7D = 7
RECENT_30D = 30

TYPE_PREFIXES = ("feedback", "project", "reference", "user")


def resolve_memory_dir(cli_override: str | None) -> Path:
    if cli_override:
        return Path(cli_override).expanduser().resolve()
    env_dir = os.environ.get("CLAUDE_MEMORY_DIR")
    if env_dir:
        return Path(env_dir).expanduser().resolve()
    primary = REPO_ROOT / ".claude" / "memory"
    if primary.is_dir():
        return primary.resolve()
    slug = "-" + "-".join(REPO_ROOT.parts[1:])
    return Path(f"/projects/.claude-config/projects/{slug}/memory").resolve()


@dataclass
class TypeStats:
    count: int = 0
    total_bytes: int = 0
    oldest_mtime: datetime | None = None
    newest_mtime: datetime | None = None


@dataclass
class Report:
    memory_dir: Path
    index_path: Path
    index_exists: bool
    index_lines: int = 0
    index_over_budget: bool = False
    type_stats: dict[str, TypeStats] = field(default_factory=dict)
    untyped_files: list[str] = field(default_factory=list)
    recent_7d: list[tuple[str, datetime]] = field(default_factory=list)
    recent_30d: list[tuple[str, datetime]] = field(default_factory=list)
    stale_candidates: list[tuple[str, datetime]] = field(default_factory=list)
    index_entries_no_file: list[str] = field(default_factory=list)
    files_not_in_index: list[str] = field(default_factory=list)


# Line entries look like: - [Title](filename.md) — note
INDEX_LINE_RE = re.compile(r"^\s*-\s*\[[^\]]+\]\(([^)]+\.md)\)")


def parse_index_entries(index_path: Path) -> tuple[int, set[str]]:
    if not index_path.is_file():
        return 0, set()
    text = index_path.read_text(encoding="utf-8", errors="replace")
    lines = text.splitlines()
    referenced: set[str] = set()
    for line in lines:
        m = INDEX_LINE_RE.match(line)
        if m:
            referenced.add(m.group(1).strip())
    return len(lines), referenced


def file_type(name: str) -> str | None:
    for prefix in TYPE_PREFIXES:
        if name.startswith(prefix + "_"):
            return prefix
    return None


def scan(memory_dir: Path) -> Report:
    report = Report(
        memory_dir=memory_dir,
        index_path=memory_dir / "MEMORY.md",
        index_exists=(memory_dir / "MEMORY.md").is_file(),
    )

    if not memory_dir.is_dir():
        return report

    report.index_lines, indexed_files = parse_index_entries(report.index_path)
    report.index_over_budget = report.index_lines > MEMORY_BUDGET_LINES

    actual_files: set[str] = set()
    type_stats: dict[str, TypeStats] = {p: TypeStats() for p in TYPE_PREFIXES}

    for entry in sorted(memory_dir.iterdir()):
        if not entry.is_file() or entry.name == "MEMORY.md" or not entry.name.endswith(".md"):
            continue
        actual_files.add(entry.name)
        stat = entry.stat()
        mtime = datetime.fromtimestamp(stat.st_mtime, tz=timezone.utc)
        age_days = (NOW - mtime).days

        t = file_type(entry.name)
        if t is None:
            report.untyped_files.append(entry.name)
        else:
            s = type_stats[t]
            s.count += 1
            s.total_bytes += stat.st_size
            if s.oldest_mtime is None or mtime < s.oldest_mtime:
                s.oldest_mtime = mtime
            if s.newest_mtime is None or mtime > s.newest_mtime:
                s.newest_mtime = mtime

        if age_days <= RECENT_7D:
            report.recent_7d.append((entry.name, mtime))
        if age_days <= RECENT_30D:
            report.recent_30d.append((entry.name, mtime))
        if age_days >= STALE_DAYS:
            report.stale_candidates.append((entry.name, mtime))

    report.type_stats = type_stats
    report.index_entries_no_file = sorted(indexed_files - actual_files)
    report.files_not_in_index = sorted(actual_files - indexed_files)
    report.recent_7d.sort(key=lambda p: p[1], reverse=True)
    report.recent_30d.sort(key=lambda p: p[1], reverse=True)
    report.stale_candidates.sort(key=lambda p: p[1])
    return report


def render_report(report: Report) -> str:
    out: list[str] = []
    out.append(f"# Memory Review — {TODAY.isoformat()}\n")
    out.append(
        "Read-only diagnostic on the project's complement to Claude's native auto-memory. "
        "Generated by `.claude/scripts/memory-kit/memory-review.py`.\n"
    )
    out.append(f"**Memory dir**: `{report.memory_dir}`\n")

    if not report.memory_dir.is_dir():
        out.append("\n**Memory directory does not exist** — nothing to review.\n")
        return "\n".join(out)

    # A. Index size
    out.append("\n## A. MEMORY.md size\n")
    status = "OVER BUDGET" if report.index_over_budget else "within budget"
    if not report.index_exists:
        out.append("- MEMORY.md does not exist in the memory directory.\n")
    else:
        out.append(
            f"- {report.index_lines} lines vs {MEMORY_BUDGET_LINES}-line budget — **{status}**\n"
        )
        if report.index_over_budget:
            out.append(
                f"- Index has overflowed by {report.index_lines - MEMORY_BUDGET_LINES} lines. "
                "Claude only loads the first ~200 lines of MEMORY.md into context — entries past "
                "that point are invisible. Prune or consolidate.\n"
            )

    # B. Type distribution
    out.append("\n## B. Topic files by type\n")
    total_files = sum(s.count for s in report.type_stats.values())
    out.append(f"- Total typed files: {total_files}\n")
    if report.untyped_files:
        out.append(f"- Untyped files (no prefix match): {len(report.untyped_files)}\n")
    out.append("\n| Type | Count | Total KB | Newest | Oldest |\n")
    out.append("|---|---:|---:|---|---|\n")
    for t in TYPE_PREFIXES:
        s = report.type_stats.get(t)
        if not s or s.count == 0:
            out.append(f"| {t} | 0 | — | — | — |\n")
            continue
        newest = s.newest_mtime.date().isoformat() if s.newest_mtime else "—"
        oldest = s.oldest_mtime.date().isoformat() if s.oldest_mtime else "—"
        kb = round(s.total_bytes / 1024, 1)
        out.append(f"| {t} | {s.count} | {kb} | {newest} | {oldest} |\n")
    if report.untyped_files:
        out.append("\n**Untyped files** (consider renaming with a type prefix):\n")
        for name in report.untyped_files[:10]:
            out.append(f"- `{name}`\n")

    # C. Growth
    out.append("\n## C. Growth (filesystem mtime, not git)\n")
    out.append(f"- Last {RECENT_7D}d: {len(report.recent_7d)} entries touched\n")
    out.append(f"- Last {RECENT_30D}d: {len(report.recent_30d)} entries touched\n")
    if report.recent_7d:
        out.append("\nRecent (7d):\n")
        for name, mtime in report.recent_7d[:10]:
            out.append(f"- {mtime.date().isoformat()} — `{name}`\n")

    # D. Stale candidates
    out.append("\n## D. Stale candidates (untouched > {} days)\n".format(STALE_DAYS))
    if not report.stale_candidates:
        out.append("- None — all entries touched within the staleness window.\n")
    else:
        out.append(f"- {len(report.stale_candidates)} entries untouched > {STALE_DAYS}d\n\n")
        for name, mtime in report.stale_candidates[:20]:
            days = (NOW - mtime).days
            out.append(f"- {days}d — `{name}` (mtime {mtime.date().isoformat()})\n")
        if len(report.stale_candidates) > 20:
            out.append(f"- ...and {len(report.stale_candidates) - 20} more.\n")
        out.append(
            "\nNote: staleness is a candidate signal, not a verdict — load-bearing principles "
            "stay valid regardless of recency. Use `cleanup-scan.py` for archive judgment.\n"
        )

    # E. Index drift
    out.append("\n## E. Index ↔ files drift\n")
    if not report.index_entries_no_file and not report.files_not_in_index:
        out.append("- No drift detected. MEMORY.md and the topic-file set agree.\n")
    else:
        if report.index_entries_no_file:
            out.append(
                f"\n**Index entries with no backing file** ({len(report.index_entries_no_file)}):\n"
            )
            for name in report.index_entries_no_file[:20]:
                out.append(f"- `{name}`\n")
        if report.files_not_in_index:
            out.append(
                f"\n**Backing files missing from MEMORY.md** ({len(report.files_not_in_index)}):\n"
            )
            for name in report.files_not_in_index[:20]:
                out.append(f"- `{name}`\n")

    out.append("\n---\n")
    out.append(
        "_Read-only diagnostic. To act on findings: edit MEMORY.md to fix drift; run "
        "`cleanup-scan.py` for stale-archive judgment; rename untyped files with a "
        "`feedback_/project_/reference_/user_` prefix._\n"
    )
    return "".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--memory-dir",
        help="Override memory directory (default: auto-resolve via $CLAUDE_MEMORY_DIR or slug).",
    )
    parser.add_argument(
        "--output-dir",
        help=f"Override output directory (default: {DEFAULT_OUT_ROOT}/<YYYY-MM-DD>/).",
    )
    args = parser.parse_args()

    memory_dir = resolve_memory_dir(args.memory_dir)
    report = scan(memory_dir)
    text = render_report(report)

    out_dir = Path(args.output_dir) if args.output_dir else DEFAULT_OUT_ROOT / TODAY.isoformat()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "memory-review.md"
    out_path.write_text(text, encoding="utf-8")

    print(f"memory dir: {memory_dir}")
    print(f"index lines: {report.index_lines} (budget {MEMORY_BUDGET_LINES})")
    typed_total = sum(s.count for s in report.type_stats.values())
    print(f"typed files: {typed_total}; untyped: {len(report.untyped_files)}")
    print(f"recent 7d: {len(report.recent_7d)}; stale (>{STALE_DAYS}d): {len(report.stale_candidates)}")
    print(
        f"drift: {len(report.index_entries_no_file)} index→missing, "
        f"{len(report.files_not_in_index)} file→unindexed"
    )
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
