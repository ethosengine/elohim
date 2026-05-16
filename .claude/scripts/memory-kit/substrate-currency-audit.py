#!/usr/bin/env python3
"""
Substrate-currency audit — population-wide drift triage across gospel-tier surfaces.

Reads every .claude/agents/*.md, .claude/skills/*/SKILL.md, and CLAUDE.md
in the repo. For each surface, runs three drift checks:

  1. PATH-EXISTS: extract backticked path-like tokens from body; check
     against filesystem; flag fictional/wrong paths.
  2. PROCESS-STATUS: regex for temporal-status phrasing
     ("currently", "as of <date>", "Phase N closed", "gates remaining",
     "in.flight", "next milestone") — gospel-tier surfaces should describe
     stable architecture, not sprint state. Workflow primitives
     (TaskUpdate "in_progress" status strings) exempt by context.
  3. MISSING-CITATION: for each .claude/memory/*.md modified within
     last N days (default 14), check whether any surface cites it as
     [[slug]] anywhere. Memory entries written-but-not-cited are surfaced
     as candidates for the cartographer's coverage-gap lens.

Output: ranked JSON + dated Markdown report. Ranking is by drift count per
surface (highest first). The /memory-ceremony Phase 1 triage reads this
to surface the 1-3 highest-drift surfaces for four-lens rewrite.

Standalone-runnable. Read-only. No mutations.

Usage:
  python3 .claude/scripts/memory-kit/substrate-currency-audit.py
  python3 .claude/scripts/memory-kit/substrate-currency-audit.py --memory-days 30
  python3 .claude/scripts/memory-kit/substrate-currency-audit.py --json-only
"""
from __future__ import annotations

import argparse
import json
import os
import re
import sys
from collections import defaultdict
from dataclasses import dataclass, field, asdict
from datetime import date, datetime, timedelta, timezone
from pathlib import Path
from typing import Iterable

# Bootstrap _lib (matches sibling scripts)
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir() and (_here / ".git").exists():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths as _paths  # noqa: E402

REPO_ROOT = _paths.repo_root_from_file(__file__)
DEFAULT_OUT_ROOT = _paths.reports_root(REPO_ROOT)

# --- Surface discovery -----------------------------------------------------

CLAUDE_MD_EXCLUDE = ("node_modules", "target", ".cargo-target-pool", "sophia")


def discover_agents(root: Path) -> list[Path]:
    return sorted((root / ".claude" / "agents").glob("*.md"))


def discover_skills(root: Path) -> list[Path]:
    return sorted((root / ".claude" / "skills").glob("*/SKILL.md"))


def discover_claude_mds(root: Path) -> list[Path]:
    out: list[Path] = []
    for p in root.rglob("CLAUDE.md"):
        rel = p.relative_to(root).as_posix()
        if any(part in rel.split("/") for part in CLAUDE_MD_EXCLUDE):
            continue
        out.append(p)
    return sorted(out)


def discover_memory_entries(root: Path) -> list[Path]:
    return sorted((root / ".claude" / "memory").glob("*.md"))


# --- Body extraction -------------------------------------------------------

FRONTMATTER_RE = re.compile(r"^---\n.*?\n---\n", re.DOTALL)
FENCED_CODE_RE = re.compile(r"```[^\n]*\n.*?\n```", re.DOTALL)


def extract_body(text: str) -> str:
    """Strip YAML frontmatter + fenced code blocks. Leaves prose + inline backticks."""
    body = FRONTMATTER_RE.sub("", text, count=1)
    body = FENCED_CODE_RE.sub("", body)
    return body


# --- Check 1: Path-existence -----------------------------------------------

# Backticked tokens that look path-like:
#   contain a `/` or `.` and consist of safe filename chars
# Excludes: URLs (http*), wildcards (* in middle), inline shell args (--flag)
PATH_TOKEN_RE = re.compile(r"`([A-Za-z0-9_./-]+)`")

# Path-like predicate: has a slash OR has a known code extension OR is a
# directory-looking token ending in /
KNOWN_EXTS = (
    ".rs", ".ts", ".tsx", ".js", ".py", ".md", ".json", ".toml", ".yaml",
    ".yml", ".sh", ".html", ".css", ".feature", ".sql",
)


def looks_like_path(tok: str) -> bool:
    if tok.startswith(("http://", "https://", "//")):
        return False
    if "*" in tok:
        return False
    if tok.startswith("--") or tok.startswith("-"):
        return False
    if tok.startswith("/api/") or tok.startswith("/v1/"):  # HTTP routes, not files
        return False
    if "::" in tok:  # Rust paths like `crate::module`
        return False
    if "/" in tok:
        return True
    if tok.endswith(KNOWN_EXTS):
        return True
    if tok.endswith("/"):
        return True
    return False


def path_exists_in_repo(root: Path, claim: str) -> bool:
    """Check if claimed path exists, treating leading slash + bare names tolerantly."""
    # Strip leading/trailing slash for filesystem check
    clean = claim.strip("/")
    if not clean:
        return False
    p = root / clean
    if p.exists():
        return True
    # Some prompts cite paths relative to a subdir (e.g., `src/foo.rs` inside a crate).
    # We can't disambiguate without context, so accept bare filename matches anywhere.
    if "/" not in clean and "." in clean:
        # Bare filename — see if it exists anywhere in repo
        try:
            for _ in root.rglob(clean):
                return True
        except OSError:
            pass
        return False
    return False


def check_path_existence(root: Path, body: str) -> list[dict]:
    """Return list of {claim, exists, line_no?} entries for path-like tokens."""
    findings: list[dict] = []
    seen: set[str] = set()
    for m in PATH_TOKEN_RE.finditer(body):
        tok = m.group(1)
        if not looks_like_path(tok):
            continue
        if tok in seen:
            continue
        seen.add(tok)
        if path_exists_in_repo(root, tok):
            continue
        # Compute line-number approximation
        line_no = body.count("\n", 0, m.start()) + 1
        findings.append({"claim": tok, "exists": False, "line_no_approx": line_no})
    return findings


# --- Check 2: Process-status phrasing -------------------------------------

# Phrases that indicate sprint/phase state rather than stable architecture.
# Compiled with case-insensitive match; word boundaries where applicable.
PROCESS_STATUS_PATTERNS = [
    (re.compile(r"\bcurrently\b", re.IGNORECASE), "currently"),
    (re.compile(r"\bin[- ]flight\b", re.IGNORECASE), "in-flight"),
    (re.compile(r"\bas of \d{4}-\d{2}-\d{2}\b", re.IGNORECASE), "as of <date>"),
    (re.compile(r"\bPhase \d+\b.*?\b(closed|complete|done|landed)\b", re.IGNORECASE), "Phase N closed/complete"),
    (re.compile(r"\b(gates?|prereq(s|uisite)?s?)\b.*?\b(remaining|outstanding|left)\b", re.IGNORECASE), "gates remaining"),
    (re.compile(r"\bnext milestone\b", re.IGNORECASE), "next milestone"),
    (re.compile(r"\bproduction today\b", re.IGNORECASE), "production today"),
    (re.compile(r"\bcutover[- ](gate|window)\b", re.IGNORECASE), "cutover gate/window"),
]

# Workflow-primitive exemptions (substrings that, when found near a match,
# indicate it's a tool/code reference rather than narrative status).
EXEMPT_SUBSTR_WINDOW = 80  # chars on either side
EXEMPT_SUBSTRINGS = ("TaskUpdate", "status: \"in_progress\"", "status='in_progress'", "in_progress\"")


def check_process_status(body: str) -> list[dict]:
    findings: list[dict] = []
    for pattern, label in PROCESS_STATUS_PATTERNS:
        for m in pattern.finditer(body):
            start = max(0, m.start() - EXEMPT_SUBSTR_WINDOW)
            end = min(len(body), m.end() + EXEMPT_SUBSTR_WINDOW)
            window = body[start:end]
            if any(s in window for s in EXEMPT_SUBSTRINGS):
                continue
            line_no = body.count("\n", 0, m.start()) + 1
            findings.append({
                "phrase": label,
                "match": m.group(0),
                "line_no_approx": line_no,
            })
    return findings


# --- Check 3: Missing citation (memory entries written-but-not-cited) -----

# A surface "cites" a memory slug via [[slug]] or [[slug.md]] anywhere in the file.
WIKILINK_RE = re.compile(r"\[\[([A-Za-z0-9_.-]+?)(?:\.md)?\]\]")


def collect_cited_slugs(surface_texts: dict[Path, str]) -> set[str]:
    out: set[str] = set()
    for _, text in surface_texts.items():
        for m in WIKILINK_RE.finditer(text):
            out.add(m.group(1))
    return out


def find_uncited_recent_memory(
    memory_entries: list[Path],
    cited_slugs: set[str],
    cutoff: datetime,
) -> list[dict]:
    findings: list[dict] = []
    for mem in memory_entries:
        try:
            mtime = datetime.fromtimestamp(mem.stat().st_mtime, tz=timezone.utc)
        except OSError:
            continue
        if mtime < cutoff:
            continue
        slug = mem.stem
        if slug in cited_slugs:
            continue
        # Bypass the index itself + any aggregator that wouldn't normally be cited
        if slug in ("MEMORY", "README"):
            continue
        findings.append({
            "slug": slug,
            "path": str(mem.relative_to(REPO_ROOT)),
            "mtime": mtime.date().isoformat(),
        })
    return findings


# --- Surface aggregation ---------------------------------------------------

@dataclass
class SurfaceReport:
    path: str
    family: str  # "agent" | "skill" | "claude-md"
    line_count: int
    drift_path: list[dict] = field(default_factory=list)
    drift_process_status: list[dict] = field(default_factory=list)

    @property
    def drift_count(self) -> int:
        return len(self.drift_path) + len(self.drift_process_status)


def audit_surface(path: Path, root: Path, family: str) -> tuple[SurfaceReport, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    body = extract_body(text)
    line_count = text.count("\n") + 1
    rep = SurfaceReport(
        path=str(path.relative_to(root)),
        family=family,
        line_count=line_count,
        drift_path=check_path_existence(root, body),
        drift_process_status=check_process_status(body),
    )
    return rep, text


# --- Report writers --------------------------------------------------------

def write_json_report(out_path: Path, surfaces: list[SurfaceReport], uncited: list[dict]) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "counts": {
            "surfaces_total": len(surfaces),
            "surfaces_with_drift": sum(1 for s in surfaces if s.drift_count > 0),
            "drift_path_total": sum(len(s.drift_path) for s in surfaces),
            "drift_process_status_total": sum(len(s.drift_process_status) for s in surfaces),
            "uncited_recent_memory": len(uncited),
        },
        "by_family": {
            "agent": sum(1 for s in surfaces if s.family == "agent"),
            "skill": sum(1 for s in surfaces if s.family == "skill"),
            "claude-md": sum(1 for s in surfaces if s.family == "claude-md"),
        },
        "surfaces": [asdict(s) for s in surfaces],
        "uncited_recent_memory": uncited,
    }
    out_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")


def write_markdown_report(out_path: Path, surfaces: list[SurfaceReport], uncited: list[dict], memory_days: int) -> None:
    out_path.parent.mkdir(parents=True, exist_ok=True)
    ranked = sorted(surfaces, key=lambda s: s.drift_count, reverse=True)
    drift_surfaces = [s for s in ranked if s.drift_count > 0]

    lines: list[str] = []
    lines.append("# Substrate-currency audit")
    lines.append("")
    lines.append(f"Generated: {datetime.now(timezone.utc).isoformat()}")
    lines.append("")
    lines.append("## Headline counts")
    lines.append("")
    lines.append(f"- surfaces total: **{len(surfaces)}** (agents {sum(1 for s in surfaces if s.family == 'agent')} / skills {sum(1 for s in surfaces if s.family == 'skill')} / CLAUDE.md {sum(1 for s in surfaces if s.family == 'claude-md')})")
    lines.append(f"- surfaces with any drift: **{len(drift_surfaces)}**")
    lines.append(f"- path-existence findings total: **{sum(len(s.drift_path) for s in surfaces)}**")
    lines.append(f"- process-status findings total: **{sum(len(s.drift_process_status) for s in surfaces)}**")
    lines.append(f"- uncited memory entries (last {memory_days}d): **{len(uncited)}**")
    lines.append("")

    if drift_surfaces:
        lines.append(f"## Ranked drift (top {min(len(drift_surfaces), 15)} surfaces)")
        lines.append("")
        lines.append("| # | Surface | Family | Drift count | Path | Process-status |")
        lines.append("|---|---|---|---:|---:|---:|")
        for i, s in enumerate(drift_surfaces[:15], start=1):
            lines.append(f"| {i} | `{s.path}` | {s.family} | {s.drift_count} | {len(s.drift_path)} | {len(s.drift_process_status)} |")
        lines.append("")

    # Per-surface detail for top-N
    detail_n = min(len(drift_surfaces), 5)
    if detail_n:
        lines.append(f"## Top {detail_n} surface details")
        lines.append("")
        for s in drift_surfaces[:detail_n]:
            lines.append(f"### `{s.path}` ({s.family}, {s.line_count} lines, {s.drift_count} drift findings)")
            lines.append("")
            if s.drift_path:
                lines.append("**Path-existence findings**:")
                for f in s.drift_path[:10]:
                    lines.append(f"- L{f['line_no_approx']}: `{f['claim']}` does not resolve in repo")
                if len(s.drift_path) > 10:
                    lines.append(f"- … +{len(s.drift_path) - 10} more")
                lines.append("")
            if s.drift_process_status:
                lines.append("**Process-status phrasing** (gospel-tier should describe stable architecture):")
                for f in s.drift_process_status[:10]:
                    lines.append(f"- L{f['line_no_approx']}: `{f['phrase']}` — \"{f['match']}\"")
                if len(s.drift_process_status) > 10:
                    lines.append(f"- … +{len(s.drift_process_status) - 10} more")
                lines.append("")

    if uncited:
        lines.append(f"## Uncited recent memory entries (last {memory_days}d)")
        lines.append("")
        lines.append("These memory entries were written/touched recently but no agent / skill / CLAUDE.md cites them as `[[slug]]`. Cartographer's coverage-gap lens should evaluate which gospel surfaces SHOULD cite them.")
        lines.append("")
        for u in uncited[:25]:
            lines.append(f"- `{u['slug']}` (mtime {u['mtime']}) → `{u['path']}`")
        if len(uncited) > 25:
            lines.append(f"- … +{len(uncited) - 25} more")
        lines.append("")

    lines.append("## Methodology notes")
    lines.append("")
    lines.append("- **Path existence**: backticked tokens that look path-like (contain `/`, end in a known extension, or end in `/`). URLs and `*`-wildcards excluded. False-positive class: paths cited relative to a subdir without context — bare filename matches anywhere in repo are accepted.")
    lines.append("- **Process-status phrasing**: regex sweep on body (frontmatter + fenced code blocks stripped). Workflow primitives like `TaskUpdate ... status: \"in_progress\"` exempt within an 80-char window.")
    lines.append(f"- **Uncited recent memory**: memory entry mtime within last {memory_days}d; slug not found as `[[slug]]` in any gospel surface. Cartographer's lens decides which surfaces SHOULD cite each entry.")
    lines.append("")

    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")


# --- Main ------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0] if __doc__ else "")
    parser.add_argument("--memory-days", type=int, default=14, help="Lookback window for uncited-recent-memory check (default: 14)")
    parser.add_argument("--json-only", action="store_true", help="Emit only JSON (no Markdown)")
    parser.add_argument("--output-dir", type=Path, default=None, help="Override output directory")
    args = parser.parse_args()

    out_dir = args.output_dir or _paths.reports_dir_for_today(REPO_ROOT)

    agents = discover_agents(REPO_ROOT)
    skills = discover_skills(REPO_ROOT)
    claude_mds = discover_claude_mds(REPO_ROOT)
    memory_entries = discover_memory_entries(REPO_ROOT)

    surfaces: list[SurfaceReport] = []
    surface_texts: dict[Path, str] = {}
    for path in agents:
        rep, text = audit_surface(path, REPO_ROOT, "agent")
        surfaces.append(rep)
        surface_texts[path] = text
    for path in skills:
        rep, text = audit_surface(path, REPO_ROOT, "skill")
        surfaces.append(rep)
        surface_texts[path] = text
    for path in claude_mds:
        rep, text = audit_surface(path, REPO_ROOT, "claude-md")
        surfaces.append(rep)
        surface_texts[path] = text

    # Check 3: uncited recent memory
    cited = collect_cited_slugs(surface_texts)
    cutoff = datetime.now(timezone.utc) - timedelta(days=args.memory_days)
    uncited = find_uncited_recent_memory(memory_entries, cited, cutoff)

    json_out = out_dir / "substrate-currency-audit.json"
    write_json_report(json_out, surfaces, uncited)
    print(f"audited: {len(agents)} agents, {len(skills)} skills, {len(claude_mds)} CLAUDE.md files")
    print(f"  surfaces with drift: {sum(1 for s in surfaces if s.drift_count > 0)}")
    print(f"  path findings: {sum(len(s.drift_path) for s in surfaces)}")
    print(f"  process-status findings: {sum(len(s.drift_process_status) for s in surfaces)}")
    print(f"  uncited recent memory (last {args.memory_days}d): {len(uncited)}")
    print(f"report: {json_out}")

    if not args.json_only:
        md_out = out_dir / "substrate-currency-audit.md"
        write_markdown_report(md_out, surfaces, uncited, args.memory_days)
        print(f"        {md_out}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
