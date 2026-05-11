#!/usr/bin/env python3
"""
Deterministic staleness scanner for specs, plans, and memory entries.

Scans for HIGH-CONFIDENCE archive candidates (operator-approved bulk-action safe):
  - Specs/plans with status: superseded or # SUPERSEDED heading
  - Specs/plans with status: cancelled or # CANCELLED heading
  - Plans with all checkboxes complete AND age > 30 days
  - Specs with status: proposal AND age > 90 days AND not referenced anywhere

Scans for FLAG-FOR-REVIEW items (information only; no archive action):
  - Specs/plans with dangling file references
  - Memory entries with dangling file references

Output: .claude/cleanup/<YYYY-MM-DD>/proposals.md

Zero LLM judgment. Pure rule-based.
"""

from __future__ import annotations

import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from datetime import date, datetime, timedelta
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
MEMORY_ROOT = Path("/projects/.claude-config/projects/-projects-elohim/memory")

SPEC_GLOBS = [
    "genesis/docs/superpowers/specs/*.md",
    "genesis/docs/superpowers/plans/*.md",
    "genesis/docs/specs/*.md",
]
PLAN_GLOBS = [
    "genesis/docs/plans/*.md",
]
MEMORY_GLOB = "*.md"

# Repo top-level directories — paths starting with these are candidates for existence-check
REPO_PREFIXES = (
    "genesis/",
    "app/",
    "elohim/",
    "doorway/",
    "steward/",
    "mcp-servers/",
    "sophia/",
    ".claude/",
)

# Pattern: word/word/...ext — like a file path with at least one slash and an extension
PATH_PATTERN = re.compile(r"(?<![\w/])([a-zA-Z_][\w.-]*/[\w/.-]+\.[a-zA-Z]{1,8})\b")

STALE_PROPOSAL_AGE = timedelta(days=90)
COMPLETED_PLAN_AGE = timedelta(days=30)
TODAY = date.today()


@dataclass
class Proposal:
    kind: str  # "superseded" | "cancelled" | "completed-plan" | "stale-proposal"
    path: Path
    evidence: list[str] = field(default_factory=list)


@dataclass
class ReviewFlag:
    kind: str  # "spec-dangling-refs" | "memory-dangling-refs"
    path: Path
    missing: list[tuple[str, int]] = field(default_factory=list)  # (path, line_number)


def read_frontmatter(text: str) -> dict[str, str]:
    """Extract simple YAML frontmatter as a flat dict. Tolerates missing or malformed."""
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


def has_superseded_marker(text: str) -> str | None:
    """Return the marker string if a SUPERSEDED heading is present near the top."""
    head = "\n".join(text.splitlines()[:25])
    if re.search(r"^#\s+SUPERSEDED\b", head, re.MULTILINE):
        return "# SUPERSEDED heading near top"
    return None


def has_cancelled_marker(text: str) -> str | None:
    head = "\n".join(text.splitlines()[:25])
    if re.search(r"^#\s+(CANCELLED|CANCELED)\b", head, re.MULTILINE):
        return "# CANCELLED heading near top"
    return None


def plan_is_complete(text: str) -> tuple[bool, int, int]:
    """Return (is_complete, open_count, done_count) for a plan with checkboxes."""
    open_count = len(re.findall(r"^[\t ]*[-*]\s*\[ \]", text, re.MULTILINE))
    done_count = len(re.findall(r"^[\t ]*[-*]\s*\[[xX]\]", text, re.MULTILINE))
    return (open_count == 0 and done_count > 0, open_count, done_count)


def is_referenced_elsewhere(filename: str, exclude_path: Path) -> bool:
    """Use ripgrep to check if a filename appears anywhere else in the repo."""
    try:
        result = subprocess.run(
            [
                "rg",
                "-l",
                "--hidden",
                "--no-ignore-vcs",
                "-g",
                "!.git",
                "-g",
                "!.claude/cleanup",
                "-g",
                "!.claude/archive",
                "-F",
                filename,
                str(REPO_ROOT),
            ],
            capture_output=True,
            text=True,
            timeout=30,
        )
        hits = {Path(line).resolve() for line in result.stdout.strip().splitlines() if line}
        hits.discard(exclude_path.resolve())
        return len(hits) > 0
    except (subprocess.SubprocessError, OSError):
        return True  # On error, assume referenced (safer — don't archive)


def find_dangling_refs(text: str) -> list[tuple[str, int]]:
    """Return [(path_string, line_number)] for repo-shaped paths that don't exist."""
    out = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        for m in PATH_PATTERN.finditer(line):
            candidate = m.group(1)
            if not candidate.startswith(REPO_PREFIXES):
                continue
            if (REPO_ROOT / candidate).exists():
                continue
            out.append((candidate, lineno))
    # Deduplicate by path keeping first occurrence
    seen = set()
    deduped = []
    for path, lineno in out:
        if path not in seen:
            seen.add(path)
            deduped.append((path, lineno))
    return deduped


def scan_doc(path: Path, is_plan: bool) -> tuple[Proposal | None, ReviewFlag | None]:
    """Scan a single spec or plan file."""
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None, None

    fm = read_frontmatter(text)
    status = fm.get("status", "").lower()
    file_date = date_from_filename(path.name) or (
        datetime.strptime(fm["date"], "%Y-%m-%d").date()
        if fm.get("date", "").count("-") == 2 and len(fm.get("date", "")) >= 10
        else None
    )

    proposal: Proposal | None = None

    if status == "superseded":
        proposal = Proposal("superseded", path, ["frontmatter declares status: superseded"])
    elif status == "cancelled" or status == "canceled":
        proposal = Proposal("cancelled", path, ["frontmatter declares status: cancelled"])
    elif (marker := has_superseded_marker(text)):
        proposal = Proposal("superseded", path, [marker])
    elif (marker := has_cancelled_marker(text)):
        proposal = Proposal("cancelled", path, [marker])
    elif is_plan:
        complete, open_n, done_n = plan_is_complete(text)
        if complete and file_date and (TODAY - file_date) > COMPLETED_PLAN_AGE:
            age_days = (TODAY - file_date).days
            proposal = Proposal(
                "completed-plan",
                path,
                [f"all {done_n} checkboxes complete, 0 open; age {age_days} days"],
            )
    else:
        if status == "proposal" and file_date and (TODAY - file_date) > STALE_PROPOSAL_AGE:
            if not is_referenced_elsewhere(path.name, path):
                age_days = (TODAY - file_date).days
                proposal = Proposal(
                    "stale-proposal",
                    path,
                    [
                        f"status: proposal; age {age_days} days; no other doc references {path.name}",
                    ],
                )

    flag: ReviewFlag | None = None
    if proposal is None:  # only flag refs for docs not already archive-candidates
        missing = find_dangling_refs(text)
        if missing:
            flag = ReviewFlag("spec-dangling-refs" if not is_plan else "plan-dangling-refs", path, missing)

    return proposal, flag


def scan_memory(path: Path) -> ReviewFlag | None:
    try:
        text = path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return None
    missing = find_dangling_refs(text)
    if missing:
        return ReviewFlag("memory-dangling-refs", path, missing)
    return None


def gather_paths() -> tuple[list[Path], list[Path], list[Path]]:
    specs = []
    for pattern in SPEC_GLOBS:
        specs.extend(sorted(REPO_ROOT.glob(pattern)))
    plans = []
    for pattern in PLAN_GLOBS:
        plans.extend(sorted(REPO_ROOT.glob(pattern)))
    memories = sorted(MEMORY_ROOT.glob(MEMORY_GLOB)) if MEMORY_ROOT.exists() else []
    # MEMORY.md itself is the index — skip
    memories = [p for p in memories if p.name != "MEMORY.md"]
    return specs, plans, memories


def render_proposals(
    proposals: list[Proposal],
    flags: list[ReviewFlag],
    archive_root: Path,
) -> str:
    out = []
    out.append(f"# Cleanup Proposals — {TODAY.isoformat()}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/cleanup-scan.py`. "
        "Mark `- [x] Accept` on archive candidates, then run "
        f"`.claude/scripts/memory-kit/cleanup-apply.py .claude/memory-kit/{archive_root.name}/cleanup-proposals.md`."
    )
    out.append("")
    out.append(f"- Archive candidates: **{len(proposals)}**")
    out.append(f"- Flagged for review: **{len(flags)}**")
    out.append("")
    out.append("---")
    out.append("")

    if proposals:
        out.append("## Archive Candidates")
        out.append("")
        out.append(
            "Each entry below is HIGH-CONFIDENCE stale per deterministic rules. "
            "Accepting moves the file to the dated archive directory "
            "(preserves trajectory; not deletion)."
        )
        out.append("")
        for i, p in enumerate(proposals, start=1):
            rel = p.path.relative_to(REPO_ROOT) if p.path.is_relative_to(REPO_ROOT) else p.path
            dest = archive_root / rel
            out.append(f"### {i}. `{p.kind}` — `{rel}`")
            out.append("")
            for line in p.evidence:
                out.append(f"**Evidence**: {line}")
            out.append("")
            out.append(f"**Proposed**: move to `{dest.relative_to(REPO_ROOT) if dest.is_relative_to(REPO_ROOT) else dest}`")
            out.append("")
            out.append(f"- [ ] Accept  <!-- id: {rel} -->")
            out.append("")
            out.append("---")
            out.append("")

    if flags:
        out.append("## Flagged for Review")
        out.append("")
        out.append(
            "These have references to paths that don't exist in the repo. "
            "**No automatic action.** Manually update the references, archive "
            "the document, or ignore — your call."
        )
        out.append("")
        for i, f in enumerate(flags, start=1):
            rel = f.path.relative_to(REPO_ROOT) if f.path.is_relative_to(REPO_ROOT) else f.path
            out.append(f"### {i}. `{f.kind}` — `{rel}`")
            out.append("")
            out.append("**Missing references**:")
            for path, lineno in f.missing[:10]:
                out.append(f"- `{path}` (line {lineno})")
            if len(f.missing) > 10:
                out.append(f"- ...and {len(f.missing) - 10} more")
            out.append("")
            out.append("---")
            out.append("")

    if not proposals and not flags:
        out.append("_Nothing to propose. Repo is clean per these rules._")
        out.append("")

    return "\n".join(out)


def main() -> int:
    out_dir = REPO_ROOT / ".claude" / "memory-kit" / TODAY.isoformat()
    out_dir.mkdir(parents=True, exist_ok=True)
    archive_root = REPO_ROOT / ".claude" / "archive" / TODAY.isoformat()

    specs, plans, memories = gather_paths()

    proposals: list[Proposal] = []
    flags: list[ReviewFlag] = []

    for path in specs:
        p, f = scan_doc(path, is_plan=False)
        if p:
            proposals.append(p)
        if f:
            flags.append(f)

    for path in plans:
        p, f = scan_doc(path, is_plan=True)
        if p:
            proposals.append(p)
        if f:
            flags.append(f)

    for path in memories:
        f = scan_memory(path)
        if f:
            flags.append(f)

    output = render_proposals(proposals, flags, archive_root)
    out_path = out_dir / "cleanup-proposals.md"
    out_path.write_text(output, encoding="utf-8")

    print(f"scanned: {len(specs)} specs, {len(plans)} plans, {len(memories)} memory entries")
    print(f"archive candidates: {len(proposals)}")
    print(f"review flags: {len(flags)}")
    print(f"proposals: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
