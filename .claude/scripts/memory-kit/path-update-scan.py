#!/usr/bin/env python3
"""
Deterministic scanner for path-update memory hygiene.

Detects file renames (git-tracked AND inferred) and proposes mechanical
text replacements in spec/plan/skill/memory documents that still cite
the OLD path.

Two evidence classes:
  1. git-rename: `git log --diff-filter=R` shows old → new (high confidence)
  2. inferred-rename: old path missing AND a unique candidate new path
     exists whose basename matches a known suffix-drop or sibling pattern
     (medium confidence; surfaced as Accept candidates only when unique)

Outputs:
  .claude/memory-kit/<YYYY-MM-DD>/path-update-proposals.md

Boundary:
  - Read-only. Does NOT modify files.
  - Does NOT archive. Does NOT merge entries. Does NOT change content
    meaning — only tracks path strings for downstream apply.

Pure stdlib + subprocess (git, ripgrep).
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import date, timedelta
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MEMORY_ROOT = Path("/projects/.claude-config/projects/-projects-elohim/memory")

# Doc locations to scan for stale path citations.
DOC_GLOBS = [
    "genesis/docs/**/*.md",
    ".claude/skills/**/*.md",
    ".claude/agents/**/*.md",
    ".claude/commands/**/*.md",
    "*.md",  # root-level markdown (CLAUDE.md, README.md)
]

# How far back to walk git history for renames.
DEFAULT_RENAME_WINDOW_DAYS = 365

# Repo top-level prefixes — mirror cleanup-scan.py's notion of "repo-shaped paths".
REPO_PREFIXES = (
    "genesis/",
    "app/",
    "elohim/",
    "doorway/",
    "steward/",
    "mcp-servers/",
    "sophia/",
    ".claude/",
    ".husky/",
)

# Pattern: word/word/...ext — file path with at least one slash and an extension.
# Same shape as cleanup-scan.py for consistency.
PATH_PATTERN = re.compile(r"(?<![\w/])([a-zA-Z_][\w.-]*/[\w/.-]+\.[a-zA-Z]{1,8})\b")

TODAY = date.today()


@dataclass(frozen=True)
class Rename:
    """A single file rename: old → new."""
    old_path: str
    new_path: str
    commit: str  # short hash, or "inferred" for heuristic matches
    when: str  # ISO date, or "" for inferred
    confidence: str  # "git" | "inferred"

    @property
    def key(self) -> str:
        """Stable identifier for the apply step's checkbox."""
        return f"{self.confidence}:{self.old_path}->{self.new_path}"


@dataclass
class Hit:
    """One occurrence of an old path inside a doc file."""
    doc_path: Path
    line_number: int
    line_text: str


@dataclass
class Proposal:
    """An accept-candidate: rename plus all the doc hits to update."""
    rename: Rename
    hits: list[Hit] = field(default_factory=list)


@dataclass
class ReviewItem:
    """A finding that needs operator review (no Accept checkbox)."""
    reason: str
    detail: str


# ----------------------------------------------------------------------------
# Git rename detection
# ----------------------------------------------------------------------------

def detect_git_renames(window_days: int) -> list[Rename]:
    """Walk git log for file renames within the window."""
    since = (date.today() - timedelta(days=window_days)).isoformat()
    try:
        result = subprocess.run(
            [
                "git",
                "log",
                "--all",
                "--diff-filter=R",
                "--name-status",
                "--format=COMMIT:%h:%ai",
                f"--since={since}",
            ],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=120,
        )
    except (subprocess.SubprocessError, OSError) as exc:
        print(f"warning: git log failed: {exc}", file=sys.stderr)
        return []

    renames: list[Rename] = []
    current_commit = ""
    current_when = ""
    seen_pairs: set[tuple[str, str]] = set()

    for line in result.stdout.splitlines():
        if line.startswith("COMMIT:"):
            parts = line.split(":", 2)
            if len(parts) >= 3:
                current_commit = parts[1]
                # ISO timestamp; keep just the date portion.
                current_when = parts[2].split(" ", 1)[0] if parts[2] else ""
            continue
        if not line or not line[0] == "R":
            continue
        # Format: "R<score>\t<old>\t<new>"
        fields = line.split("\t")
        if len(fields) < 3:
            continue
        old_path, new_path = fields[1], fields[2]
        # Collapse identical rename pairs (multi-commit chains use most-recent).
        pair = (old_path, new_path)
        if pair in seen_pairs:
            continue
        seen_pairs.add(pair)
        renames.append(
            Rename(
                old_path=old_path,
                new_path=new_path,
                commit=current_commit,
                when=current_when,
                confidence="git",
            )
        )
    return renames


# ----------------------------------------------------------------------------
# Inferred rename detection (filename suffix drops, etc.)
# ----------------------------------------------------------------------------

# Known suffix-drop patterns observed in the codebase. Each entry is
# (old-suffix-before-extension, new-suffix-before-extension) — we strip
# the old and look for the new.
SUFFIX_REWRITES: list[tuple[str, str]] = [
    ("-view.schema", ".schema"),  # *-view.schema.json → *.schema.json
    ("-view.json", ".json"),
]


def infer_rename_from_missing_path(
    missing_path: str,
    existing_paths: set[str],
) -> Rename | None:
    """Try suffix-drop heuristics; return a Rename only if exactly one candidate matches."""
    candidates: list[str] = []
    for old_marker, new_marker in SUFFIX_REWRITES:
        if old_marker in missing_path:
            candidate = missing_path.replace(old_marker, new_marker, 1)
            if candidate in existing_paths and candidate != missing_path:
                candidates.append(candidate)
    # Only confidently propose when exactly one heuristic resolves.
    if len(candidates) == 1:
        return Rename(
            old_path=missing_path,
            new_path=candidates[0],
            commit="inferred",
            when="",
            confidence="inferred",
        )
    return None


# ----------------------------------------------------------------------------
# Doc scanning
# ----------------------------------------------------------------------------

def gather_doc_paths() -> list[Path]:
    """All files we're willing to update."""
    docs: set[Path] = set()
    for pattern in DOC_GLOBS:
        for p in REPO_ROOT.glob(pattern):
            if p.is_file():
                docs.add(p)
    if MEMORY_ROOT.exists():
        for p in MEMORY_ROOT.glob("*.md"):
            if p.is_file():
                docs.add(p)
    # Skip our own outputs to avoid self-reference loops.
    docs = {p for p in docs if ".claude/memory-kit" not in str(p)}
    docs = {p for p in docs if ".claude/cleanup" not in str(p)}
    docs = {p for p in docs if ".claude/archive" not in str(p)}
    return sorted(docs)


def find_path_hits_in_doc(doc_path: Path, target_paths: set[str]) -> list[Hit]:
    """Return hits where any target_path appears in the doc."""
    try:
        text = doc_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return []
    hits: list[Hit] = []
    for lineno, line in enumerate(text.splitlines(), start=1):
        for m in PATH_PATTERN.finditer(line):
            candidate = m.group(1)
            if candidate in target_paths:
                hits.append(Hit(doc_path, lineno, line.rstrip()))
    return hits


def collect_existing_repo_paths() -> set[str]:
    """All current repo-relative paths under known prefixes (used for inferred-rename matching)."""
    existing: set[str] = set()
    try:
        result = subprocess.run(
            ["git", "ls-files"],
            cwd=REPO_ROOT,
            capture_output=True,
            text=True,
            timeout=60,
        )
        for line in result.stdout.splitlines():
            if line.startswith(REPO_PREFIXES):
                existing.add(line)
    except (subprocess.SubprocessError, OSError) as exc:
        print(f"warning: git ls-files failed: {exc}", file=sys.stderr)
    return existing


def collect_dangling_paths_from_docs(doc_paths: list[Path], existing: set[str]) -> set[str]:
    """Walk all docs, return repo-shaped paths that are referenced but don't exist."""
    dangling: set[str] = set()
    for doc in doc_paths:
        try:
            text = doc.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for line in text.splitlines():
            for m in PATH_PATTERN.finditer(line):
                candidate = m.group(1)
                if not candidate.startswith(REPO_PREFIXES):
                    continue
                if candidate in existing:
                    continue
                if (REPO_ROOT / candidate).exists():
                    # Untracked file that exists on disk — not dangling.
                    continue
                dangling.add(candidate)
    return dangling


# ----------------------------------------------------------------------------
# Filtering & ambiguity guards
# ----------------------------------------------------------------------------

def filter_renames(
    renames: list[Rename],
    existing: set[str],
) -> tuple[list[Rename], list[ReviewItem]]:
    """
    Drop renames where:
      - new_path no longer exists (was renamed again or deleted)
      - old_path was re-created (now exists; updating refs would break them)
    Surface ambiguous cases (same old_path → multiple new_paths) as ReviewItems.
    """
    by_old: dict[str, list[Rename]] = defaultdict(list)
    for r in renames:
        by_old[r.old_path].append(r)

    kept: list[Rename] = []
    review: list[ReviewItem] = []

    for old_path, group in by_old.items():
        # If old_path now exists on disk, do NOT propose updates — the original
        # name is live again (e.g. the file was restored or the rename was undone).
        if old_path in existing:
            continue
        # Drop entries whose target no longer exists (chained rename or delete).
        live = [r for r in group if r.new_path in existing]
        if not live:
            continue
        # Multiple distinct surviving destinations → ambiguous. Flag for review.
        unique_destinations = {r.new_path for r in live}
        if len(unique_destinations) > 1:
            review.append(
                ReviewItem(
                    reason="ambiguous-rename",
                    detail=(
                        f"`{old_path}` has multiple surviving destinations: "
                        + ", ".join(f"`{d}`" for d in sorted(unique_destinations))
                        + " — operator must choose"
                    ),
                )
            )
            continue
        # Pick the most recent rename (last commit wins; renames list is in
        # log order which is reverse-chronological).
        kept.append(live[0])

    # Stable order for deterministic output.
    kept.sort(key=lambda r: (r.old_path, r.new_path))
    return kept, review


def collapse_chained_renames(renames: list[Rename]) -> list[Rename]:
    """
    Follow rename chains. If A→B and B→C both exist, emit A→C with the
    later commit's evidence. Preserves the user-visible "the path X you cited
    is now Y" outcome regardless of intermediate hops.
    """
    forward: dict[str, Rename] = {}
    for r in renames:
        # If we've already seen old_path, keep the most-recent (renames list
        # is reverse-chronological from git log).
        forward.setdefault(r.old_path, r)

    collapsed: dict[str, Rename] = {}
    for old in forward:
        # Walk the chain.
        seen = {old}
        current = forward[old]
        while current.new_path in forward and current.new_path not in seen:
            seen.add(current.new_path)
            next_step = forward[current.new_path]
            current = Rename(
                old_path=old,
                new_path=next_step.new_path,
                commit=next_step.commit,
                when=next_step.when,
                confidence=current.confidence,  # preserve original confidence
            )
        collapsed[old] = current
    return list(collapsed.values())


# ----------------------------------------------------------------------------
# Output rendering
# ----------------------------------------------------------------------------

def relpath(p: Path) -> str:
    try:
        return str(p.relative_to(REPO_ROOT))
    except ValueError:
        return str(p)


def render_report(
    proposals: list[Proposal],
    review_items: list[ReviewItem],
    rename_count_total: int,
    rename_count_with_hits: int,
    inferred_count: int,
    docs_scanned: int,
) -> str:
    out: list[str] = []
    out.append(f"# Path-Update Proposals — {TODAY.isoformat()}")
    out.append("")
    out.append(
        "Generated by `.claude/scripts/memory-kit/path-update-scan.py`. "
        "Mark `- [x] Accept` on each rename you confirm, then run "
        f"`python3 .claude/scripts/memory-kit/path-update-apply.py "
        f"{TODAY.isoformat()}/path-update-proposals.md`."
    )
    out.append("")
    out.append("## Summary")
    out.append("")
    out.append(f"- Renames detected (git + inferred): **{rename_count_total}**")
    out.append(f"- Renames cited in tracked docs: **{rename_count_with_hits}**")
    out.append(f"- Inferred-rename proposals (heuristic, not git-tracked): **{inferred_count}**")
    out.append(f"- Docs scanned: **{docs_scanned}**")
    out.append(f"- Items needing operator review (no Accept checkbox): **{len(review_items)}**")
    out.append("")
    out.append("---")
    out.append("")

    # Group proposals by affected doc for readability.
    if proposals:
        out.append("## Accept Candidates")
        out.append("")
        out.append(
            "Each entry below is a single OLD → NEW path replacement. Accepting "
            "applies the replacement to every listed affected file. Apply preserves "
            "line endings, indentation, and surrounding context — only the path "
            "string is rewritten."
        )
        out.append("")
        for i, prop in enumerate(proposals, start=1):
            r = prop.rename
            out.append(f"### {i}. `{r.confidence}` rename")
            out.append("")
            out.append(f"- **Old**: `{r.old_path}`")
            out.append(f"- **New**: `{r.new_path}`")
            if r.confidence == "git":
                out.append(f"- **Evidence**: git commit `{r.commit}` ({r.when})")
            else:
                out.append(
                    "- **Evidence**: heuristic match — old path missing, "
                    "new path exists; basename matches a known suffix-drop pattern"
                )
            # Group hits by file for compactness.
            by_file: dict[Path, list[Hit]] = defaultdict(list)
            for h in prop.hits:
                by_file[h.doc_path].append(h)
            out.append(f"- **Affected files**: {len(by_file)} ({len(prop.hits)} occurrence{'s' if len(prop.hits) != 1 else ''})")
            out.append("")
            for doc_path in sorted(by_file):
                file_hits = by_file[doc_path]
                line_numbers = ", ".join(str(h.line_number) for h in file_hits)
                out.append(f"  - `{relpath(doc_path)}` (line{'s' if len(file_hits) > 1 else ''}: {line_numbers})")
            out.append("")
            out.append(f"- [ ] Accept  <!-- id: {r.key} -->")
            out.append("")
            out.append("---")
            out.append("")

    if review_items:
        out.append("## Needs Operator Review")
        out.append("")
        out.append(
            "These findings have ambiguity the scanner won't resolve. No Accept "
            "checkbox is provided; investigate manually and either edit a doc "
            "directly or extend the scanner's heuristics."
        )
        out.append("")
        for i, item in enumerate(review_items, start=1):
            out.append(f"{i}. **{item.reason}** — {item.detail}")
        out.append("")

    if not proposals and not review_items:
        out.append("_No path-update proposals. Either no renames cited in docs, or no renames detected in window._")
        out.append("")

    return "\n".join(out)


# ----------------------------------------------------------------------------
# Main
# ----------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--window-days",
        type=int,
        default=DEFAULT_RENAME_WINDOW_DAYS,
        help=f"How far back to walk git history for renames (default: {DEFAULT_RENAME_WINDOW_DAYS})",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Override output directory (default: .claude/memory-kit/<TODAY>/)",
    )
    args = parser.parse_args()

    out_dir = args.out_dir or (REPO_ROOT / ".claude" / "memory-kit" / TODAY.isoformat())
    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"scanning git renames over last {args.window_days} days...")
    git_renames = detect_git_renames(args.window_days)
    print(f"  raw git renames: {len(git_renames)}")

    existing = collect_existing_repo_paths()
    print(f"  current tracked files under repo prefixes: {len(existing)}")

    git_renames, ambiguity_review = filter_renames(git_renames, existing)
    git_renames = collapse_chained_renames(git_renames)
    print(f"  filtered + chain-collapsed git renames: {len(git_renames)}")

    print("collecting docs to scan...")
    doc_paths = gather_doc_paths()
    print(f"  doc files: {len(doc_paths)}")

    # Inferred-rename pass: only for paths actually cited in our docs that
    # don't exist (no point inferring renames the docs never mention).
    print("collecting dangling references in docs (for inferred-rename pass)...")
    dangling = collect_dangling_paths_from_docs(doc_paths, existing)
    print(f"  distinct dangling paths cited in docs: {len(dangling)}")

    # Skip dangling paths already explained by a git rename.
    git_old_paths = {r.old_path for r in git_renames}
    inferred: list[Rename] = []
    for missing in dangling:
        if missing in git_old_paths:
            continue
        candidate = infer_rename_from_missing_path(missing, existing)
        if candidate:
            inferred.append(candidate)
    print(f"  inferred renames (suffix-drop heuristics): {len(inferred)}")

    all_renames = git_renames + inferred

    # Build target-path set so we can scan each doc once for any old_path mention.
    target_paths = {r.old_path for r in all_renames}
    rename_by_old: dict[str, Rename] = {r.old_path: r for r in all_renames}

    print(f"scanning {len(doc_paths)} docs for {len(target_paths)} candidate old-paths...")
    proposals_by_old: dict[str, Proposal] = {}
    for doc in doc_paths:
        hits = find_path_hits_in_doc(doc, target_paths)
        for hit in hits:
            # Re-resolve the old path for this hit.
            for m in PATH_PATTERN.finditer(hit.line_text):
                candidate = m.group(1)
                if candidate in rename_by_old:
                    prop = proposals_by_old.setdefault(
                        candidate,
                        Proposal(rename_by_old[candidate]),
                    )
                    # Avoid duplicate hit entries (same doc+line+old_path).
                    if not any(
                        h.doc_path == hit.doc_path and h.line_number == hit.line_number
                        for h in prop.hits
                    ):
                        prop.hits.append(hit)

    proposals = sorted(
        proposals_by_old.values(),
        key=lambda p: (p.rename.confidence, p.rename.old_path),
    )

    report = render_report(
        proposals=proposals,
        review_items=ambiguity_review,
        rename_count_total=len(all_renames),
        rename_count_with_hits=len(proposals),
        inferred_count=len(inferred),
        docs_scanned=len(doc_paths),
    )
    out_path = out_dir / "path-update-proposals.md"
    out_path.write_text(report, encoding="utf-8")

    print()
    print(f"renames detected: {len(all_renames)} ({len(git_renames)} git, {len(inferred)} inferred)")
    print(f"renames cited in docs: {len(proposals)}")
    print(f"items needing review: {len(ambiguity_review)}")
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
