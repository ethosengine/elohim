#!/usr/bin/env python3
"""
CLAUDE.md Review — the ceremony.

Walks every CLAUDE.md in the repo, classifies each by drift signals + content
patterns, and produces a per-file proposal report. Read-only; no mutations.
Operator reads the report and decides which files to revise.

The ceremony is *signal-triggered*, not time-triggered: the accumulator hook
(`.claude/hooks/claude-md-drift-signal.py`) tracks edit signals between
audits. When `drift_score >= threshold` for a file, the audit surfaces it
as needing attention. Files below threshold render as FRESH.

Layered phases (cheap → expensive):
  Phase 1 (always): line count, mtime, accumulated drift score, bias-imperative
                    regex, dead-path detection (against filesystem).
  Phase 2 (flagged-only): cross-file overlap detection, memory-cluster overlap.
  Phase 3 (operator on-demand): judgment subagent dispatch (not implemented yet
                                — parallel to cleanup-scan's Phase 2 structure).

Output: .claude/memory-kit/<YYYY-MM-DD>/claude-md-audit.md
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections import defaultdict
from dataclasses import dataclass, field
from datetime import date, datetime, timezone
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up. Anchor on .git too
# so an accidental .claude/.claude/ artifact can't trick the walk into
# resolving to .claude/ as repo root.
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir() and (_here / ".git").exists():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import paths as _paths, store as _store  # noqa: E402
from _lib import drift_score as _drift_score  # noqa: E402
from _lib import frontmatter as _fm  # noqa: E402

REPO_ROOT = _paths.repo_root_from_file(__file__)
DEFAULT_OUT_ROOT = _paths.reports_root(REPO_ROOT)
DRIFT_STORE = DEFAULT_OUT_ROOT / "claude-md-drift.json"
TODAY = date.today()

# Tunables
LINE_BUDGET_WARN = 200          # per-file warning threshold
DEFAULT_DRIFT_THRESHOLD = 3.0
IMPERATIVE_PATTERNS = [
    re.compile(r"\b(always|never|must|MUST|do not|don'?t)\b", re.IGNORECASE),
]
# A "rationale anchor" within N lines justifies an imperative; otherwise it's
# flagged as OVER-IMPERATIVE
RATIONALE_PATTERNS = [
    re.compile(r"\b(because|due to|to (avoid|prevent|ensure)|reason:|why:|fixes:)\b", re.IGNORECASE),
]
RATIONALE_WINDOW = 3  # lines before+after to check for rationale

# Fit-analysis tunables (claude_md_lines vs scope complexity)
# Direct scope = files in dir-tree minus descendant-CLAUDE.md subscopes
FIT_TUNABLES = {
    "tiny_scope_files": 3,         # ≤ this many files = tiny scope
    "tiny_doc_max_lines": 20,      # CLAUDE.md > this is too much for tiny scope
    "expected_base_lines": 20,     # baseline (header + overview)
    "expected_per_extension": 8,   # lines per distinct extension in scope
    "expected_per_file_factor": 0.5,  # gentle growth per file
    "under_wrote_ratio": 0.4,      # claude_md_lines / expected < this
    "over_wrote_ratio": 2.5,       # claude_md_lines / expected > this
}

# Missing-CLAUDE.md detection: find directories that probably need one
MISSING_TUNABLES = {
    "min_direct_files": 15,        # this many files in dir warrants doc consideration
    "min_subdir_count": 4,         # OR this many subdirs (architectural complexity)
    "min_distinct_extensions": 2,  # complexity threshold (single-ext dirs often need less doc)
    "min_ancestor_distance": 2,    # closest ancestor CLAUDE.md must be ≥ N levels up
    "max_candidates_reported": 30, # cap report length
}

# Opt-out marker: a directory containing this file is intentionally excluded
# from MISSING-CLAUDE-MD candidacy. The file's content is the operator's
# rationale (markdown). Future audits surface these in a separate section so
# the decision-chain stays auditable and surrounding CLAUDE.md updates can
# reference what's already been considered.
SKIP_MARKER_FILE = ".no-claude.md"
# Extensions that *count* toward distinct-extensions complexity. Ignore generic noise.
COMPLEXITY_EXTS = {
    ".ts", ".tsx", ".js", ".jsx", ".py", ".rs", ".rb", ".go",
    ".java", ".kt", ".swift", ".sh", ".sql",
    ".html", ".scss", ".css",
    ".feature", ".gherkin",
    ".sol",  # solidity (just in case)
}
# Skip directories that don't reflect real scope complexity
SCOPE_SKIP_DIRS = {
    "node_modules", ".git", ".angular", "target", "dist", "build",
    "coverage", ".cargo", ".pnpm-store", "__pycache__", ".pytest_cache",
    "worktrees", ".claude-config",
}


@dataclass
class FitAnalysis:
    scope_file_count: int = 0
    scope_distinct_extensions: int = 0
    scope_subscope_count: int = 0    # descendant dirs that have their own CLAUDE.md
    expected_lines: int = 0
    fit_ratio: float = 0.0           # claude_md_lines / expected_lines
    fit_class: str = "APPROPRIATE-FIT"  # APPROPRIATE-FIT | MAYBE-UNNECESSARY | UNDER-WROTE | OVER-WROTE


@dataclass
class FileReport:
    path: Path
    rel: str
    line_count: int = 0
    mtime: datetime | None = None
    drift_score: float = 0.0
    direct_edits: int = 0
    scope_edits: int = 0
    structural_edits: int = 0
    last_audited: str | None = None
    cited_paths: list[str] = field(default_factory=list)
    dead_paths: list[str] = field(default_factory=list)
    cited_commands: list[str] = field(default_factory=list)
    imperatives_without_rationale: list[tuple[int, str]] = field(default_factory=list)
    over_budget: bool = False
    fit: FitAnalysis = field(default_factory=FitAnalysis)
    classification: str = "FRESH"
    notes: list[str] = field(default_factory=list)


def find_claude_md_files() -> list[Path]:
    """All CLAUDE.md files in the repo (excluding worktrees and node_modules)."""
    results: list[Path] = []
    # Use git ls-files for speed and to honor .gitignore
    try:
        out = subprocess.run(
            ["git", "-c", "safe.directory=*", "-C", str(REPO_ROOT),
             "ls-files", "--cached", "--others", "--exclude-standard",
             "*CLAUDE.md", ":!*node_modules*", ":!*worktrees*"],
            capture_output=True, text=True, timeout=10,
        )
        for line in out.stdout.splitlines():
            line = line.strip()
            if not line:
                continue
            results.append(REPO_ROOT / line)
        if results:
            return sorted(results)
    except (subprocess.SubprocessError, OSError, FileNotFoundError):
        pass
    # Fallback: filesystem walk
    for p in REPO_ROOT.rglob("CLAUDE.md"):
        rel = str(p.relative_to(REPO_ROOT))
        if "node_modules" in rel or "worktrees" in rel or ".git" in rel:
            continue
        results.append(p)
    return sorted(results)


def load_drift_store() -> dict:
    return _store.load_json(
        DRIFT_STORE, default={"files": {}, "threshold": DEFAULT_DRIFT_THRESHOLD}
    )


PATH_CITATION_RE = re.compile(r"`([a-zA-Z0-9_./\-]+\.(?:ts|tsx|js|py|rs|md|json|yaml|toml|sh))`")


def extract_cited_paths(text: str) -> list[str]:
    return list({m.group(1) for m in PATH_CITATION_RE.finditer(text)})


def check_dead_paths(repo_root: Path, citations: list[str],
                     citing_file_dir: Path | None = None) -> list[str]:
    """Check each citation for existence; try multiple resolution strategies.

    Strategies (first match wins):
      1. Relative to the citing CLAUDE.md's directory (most natural for prose
         like "see `views.rs`" when CLAUDE.md lives next to views.rs).
      2. Relative to repo root.
      3. Walk up from citing dir toward repo root, checking at each level.
      4. Filesystem fallback: rglob for bare-basename citations (single match
         under repo root, excluding skip-dirs).

    A citation is "dead" only if NONE of these resolve to an existing path.
    """
    dead: list[str] = []
    for cit in citations:
        stripped = cit.lstrip("/")
        # Strategy 1: relative to citing file's dir
        if citing_file_dir is not None:
            candidate = (citing_file_dir / stripped).resolve()
            if candidate.exists():
                continue
        # Strategy 2: relative to repo root
        candidate = (repo_root / stripped).resolve()
        if candidate.exists():
            continue
        # Strategy 3: walk up from citing dir toward repo root
        found = False
        if citing_file_dir is not None:
            cur = citing_file_dir.parent
            try:
                cur.relative_to(repo_root)
                walking = True
            except ValueError:
                walking = False
            while walking:
                cand = (cur / stripped).resolve()
                if cand.exists():
                    found = True
                    break
                if cur == repo_root:
                    break
                cur = cur.parent
                try:
                    cur.relative_to(repo_root)
                except ValueError:
                    break
        if found:
            continue
        # Strategy 4: bare-basename rglob fallback (only when citation has no
        # path separator — otherwise the prior strategies would have found it)
        if "/" not in stripped:
            # Cheap check via os.walk skipping known noise dirs
            hit = False
            for root, dirs, files in os.walk(repo_root):
                dirs[:] = [d for d in dirs if d not in SCOPE_SKIP_DIRS]
                if stripped in files:
                    hit = True
                    break
            if hit:
                continue
        dead.append(cit)
    return dead


def compute_direct_scopes(claude_md_files: list[Path]) -> dict[Path, set[Path]]:
    """For each CLAUDE.md, compute its *direct* scope: descendant files NOT
    owned by another CLAUDE.md.

    A CLAUDE.md at /a/CLAUDE.md owns everything under /a/, EXCEPT what
    /a/b/CLAUDE.md owns, etc. This gives each CLAUDE.md a non-overlapping
    slice of the tree — the files for which IT is the closest documentation.
    """
    # Map each CLAUDE.md dir to its set of descendant CLAUDE.md dirs (subscope boundaries)
    claude_md_dirs = {cm.parent.resolve() for cm in claude_md_files}
    scope_map: dict[Path, set[Path]] = {}
    for cm in claude_md_files:
        owner_dir = cm.parent.resolve()
        # Descendant subscope boundaries
        descendant_boundaries: set[Path] = set()
        for other_dir in claude_md_dirs:
            if other_dir == owner_dir:
                continue
            try:
                other_dir.relative_to(owner_dir)
            except ValueError:
                continue
            # other_dir IS a descendant of owner_dir
            descendant_boundaries.add(other_dir)
        # Walk owner_dir, collect files that are NOT inside a descendant boundary
        direct_files: set[Path] = set()
        for root, dirs, files in os.walk(owner_dir):
            # Prune skip-dirs in-place
            dirs[:] = [d for d in dirs if d not in SCOPE_SKIP_DIRS]
            root_path = Path(root)
            # Check if root is inside any descendant boundary (skip if so)
            in_boundary = any(
                _is_under(root_path, b) for b in descendant_boundaries
            )
            if in_boundary:
                dirs[:] = []  # don't descend further
                continue
            for fname in files:
                if fname == "CLAUDE.md":
                    continue  # don't count CLAUDE.md itself
                direct_files.add(root_path / fname)
        scope_map[cm.resolve()] = direct_files
    return scope_map


def _is_under(path: Path, ancestor: Path) -> bool:
    try:
        path.relative_to(ancestor)
        return True
    except ValueError:
        return False


def analyze_fit(claude_md_line_count: int, scope_files: set[Path],
                descendant_subscopes: int) -> FitAnalysis:
    fit = FitAnalysis()
    fit.scope_file_count = len(scope_files)
    fit.scope_subscope_count = descendant_subscopes
    exts = {p.suffix for p in scope_files if p.suffix in COMPLEXITY_EXTS}
    fit.scope_distinct_extensions = len(exts)

    # Expected lines = base + (distinct-extensions * per-extension) + (file-count * factor)
    expected = (
        FIT_TUNABLES["expected_base_lines"]
        + fit.scope_distinct_extensions * FIT_TUNABLES["expected_per_extension"]
        + int(fit.scope_file_count * FIT_TUNABLES["expected_per_file_factor"])
    )
    expected = max(10, min(LINE_BUDGET_WARN, expected))
    fit.expected_lines = expected
    fit.fit_ratio = round(claude_md_line_count / max(1, expected), 2)

    # Classify
    if (fit.scope_file_count <= FIT_TUNABLES["tiny_scope_files"]
            and claude_md_line_count > FIT_TUNABLES["tiny_doc_max_lines"]
            and descendant_subscopes == 0):
        fit.fit_class = "MAYBE-UNNECESSARY"
    elif fit.fit_ratio < FIT_TUNABLES["under_wrote_ratio"]:
        fit.fit_class = "UNDER-WROTE"
    elif fit.fit_ratio > FIT_TUNABLES["over_wrote_ratio"]:
        fit.fit_class = "OVER-WROTE"
    else:
        fit.fit_class = "APPROPRIATE-FIT"
    return fit


def find_skip_markers(repo_root: Path) -> list[dict]:
    """Find every .no-claude.md marker in the tree; collect rationale.

    These are the operator's explicit "no CLAUDE.md needed here, here's why"
    decisions. Surfacing them in the audit makes the decision-chain auditable
    and lets surrounding CLAUDE.md updates reference what's already been
    considered.
    """
    markers: list[dict] = []
    submodule_paths = get_submodule_paths(repo_root)

    def in_submodule(p: Path) -> bool:
        for sm in submodule_paths:
            try:
                p.relative_to(sm)
                return True
            except ValueError:
                continue
        return False

    for root, dirs, files in os.walk(repo_root):
        dirs[:] = [d for d in dirs if d not in SCOPE_SKIP_DIRS]
        root_path = Path(root).resolve()
        if in_submodule(root_path):
            dirs[:] = []
            continue
        if SKIP_MARKER_FILE not in files:
            continue
        marker_path = root_path / SKIP_MARKER_FILE
        try:
            text = marker_path.read_text(encoding="utf-8", errors="replace")
            mtime = datetime.fromtimestamp(marker_path.stat().st_mtime, tz=timezone.utc)
        except OSError:
            continue
        # Use the shared frontmatter parser so we skip past the YAML block
        # cleanly and pick a real body sentence as the rationale excerpt
        fm = _fm.parse(text)
        decided_date = fm.get("decided", "") or mtime.date().isoformat()
        rationale_summary = ""
        for line in fm.body.splitlines():
            line = line.strip()
            if not line or line.startswith("#") or line.startswith(">"):
                continue
            rationale_summary = line[:200]
            break
        markers.append({
            "path": str(root_path.relative_to(repo_root)),
            "marker_path": str(marker_path.relative_to(repo_root)),
            "rationale_summary": rationale_summary or "(no rationale text)",
            "decided": decided_date,
            "mtime": mtime.date().isoformat(),
        })
    markers.sort(key=lambda m: m["path"])
    return markers


def get_submodule_paths(repo_root: Path) -> set[Path]:
    """Read .gitmodules and return absolute paths of all submodule trees.

    Submodules belong to other projects; we don't audit them.
    """
    gm = repo_root / ".gitmodules"
    if not gm.is_file():
        return set()
    paths: set[Path] = set()
    try:
        for line in gm.read_text(encoding="utf-8").splitlines():
            line = line.strip()
            if line.startswith("path"):
                _, _, val = line.partition("=")
                rel = val.strip()
                if rel:
                    paths.add((repo_root / rel).resolve())
    except OSError:
        pass
    return paths


def find_missing_claude_md_candidates(
    repo_root: Path, claude_md_files: list[Path]
) -> list[dict]:
    """Walk the repo tree, flag directories that probably need a CLAUDE.md.

    Heuristic: a directory is a candidate if it has substantial content
    (≥15 files OR ≥4 subdirs, AND ≥2 distinct complexity extensions) AND
    its closest ancestor CLAUDE.md is ≥2 levels up (otherwise the parent's
    doc is close enough).

    Skips git submodules — they belong to other projects.
    """
    claude_md_dirs = {cm.parent.resolve() for cm in claude_md_files}
    submodule_paths = get_submodule_paths(repo_root)
    candidates: list[dict] = []

    def in_submodule(p: Path) -> bool:
        for sm in submodule_paths:
            try:
                p.relative_to(sm)
                return True
            except ValueError:
                continue
        return False

    for root, dirs, files in os.walk(repo_root):
        # Prune skip-dirs in-place (don't descend into node_modules, etc.)
        dirs[:] = [d for d in dirs if d not in SCOPE_SKIP_DIRS]
        root_path = Path(root).resolve()

        # Skip submodule trees entirely
        if in_submodule(root_path):
            dirs[:] = []  # don't descend
            continue

        # Skip if this dir has an explicit opt-out marker
        if SKIP_MARKER_FILE in files:
            continue

        # Skip if this dir already has a CLAUDE.md
        if root_path in claude_md_dirs:
            continue
        # Skip repo root itself (always has one — and if it doesn't, that's a
        # different conversation than this audit)
        if root_path == repo_root:
            continue

        # Distance to nearest ancestor CLAUDE.md
        ancestor_dist = 0
        cur = root_path.parent
        found_ancestor: Path | None = None
        while cur != cur.parent:
            ancestor_dist += 1
            if cur in claude_md_dirs:
                found_ancestor = cur
                break
            cur = cur.parent
        # If no ancestor found at all, treat as infinitely far
        if not found_ancestor:
            ancestor_dist = 999

        # Skip if parent CLAUDE.md is close enough
        if ancestor_dist < MISSING_TUNABLES["min_ancestor_distance"]:
            continue

        # Count direct files (in this dir only, not recursive)
        direct_files = [f for f in files if f != "CLAUDE.md"]
        subdir_count = len(dirs)
        exts = {Path(f).suffix for f in direct_files if Path(f).suffix in COMPLEXITY_EXTS}

        # Substantiality check
        if (len(direct_files) >= MISSING_TUNABLES["min_direct_files"]
                or subdir_count >= MISSING_TUNABLES["min_subdir_count"]):
            if len(exts) >= MISSING_TUNABLES["min_distinct_extensions"]:
                candidates.append({
                    "path": str(root_path.relative_to(repo_root)),
                    "direct_files": len(direct_files),
                    "subdirs": subdir_count,
                    "distinct_extensions": len(exts),
                    "extension_list": sorted(exts),
                    "ancestor_distance": ancestor_dist,
                    "ancestor": str(found_ancestor.relative_to(repo_root))
                        if found_ancestor and found_ancestor != repo_root
                        else "<repo-root>",
                })

    # Sort by likelihood (file count + ancestor distance — most-orphaned first)
    candidates.sort(
        key=lambda c: (-c["direct_files"], -c["ancestor_distance"], c["path"])
    )
    return candidates


def find_imperatives(lines: list[str]) -> list[tuple[int, str]]:
    """Find imperative lines that lack a rationale anchor within window."""
    flagged: list[tuple[int, str]] = []
    for i, line in enumerate(lines):
        if not any(p.search(line) for p in IMPERATIVE_PATTERNS):
            continue
        # Skip headers and command blocks
        stripped = line.strip()
        if stripped.startswith("#") or stripped.startswith("```"):
            continue
        # Window check for rationale
        start = max(0, i - RATIONALE_WINDOW)
        end = min(len(lines), i + RATIONALE_WINDOW + 1)
        window_text = "\n".join(lines[start:end])
        if any(p.search(window_text) for p in RATIONALE_PATTERNS):
            continue
        flagged.append((i + 1, line.strip()))
    return flagged


def analyze_file(path: Path, drift_store: dict, threshold: float,
                 scope_map: dict[Path, set[Path]],
                 subscope_counts: dict[Path, int]) -> FileReport:
    rep = FileReport(path=path, rel=str(path.relative_to(REPO_ROOT)))
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        rep.notes.append("unreadable")
        return rep
    lines = text.splitlines()
    rep.line_count = len(lines)
    rep.over_budget = rep.line_count > LINE_BUDGET_WARN
    try:
        st = path.stat()
        rep.mtime = datetime.fromtimestamp(st.st_mtime, tz=timezone.utc)
    except OSError:
        pass

    entry = drift_store.get("files", {}).get(rep.rel, {})
    rep.direct_edits = int(entry.get("direct_edits", 0))
    rep.scope_edits = int(entry.get("scope_edits", 0))
    rep.structural_edits = int(entry.get("structural_edits", 0))
    rep.last_audited = entry.get("last_audited")
    # Audit recomputes drift_score from counters at audit time (counters are
    # source of truth; the stored value is a cache that hooks update lazily).
    rep.drift_score = _drift_score.compute_score(entry)

    # Phase 1 deterministic checks
    rep.cited_paths = extract_cited_paths(text)
    rep.dead_paths = check_dead_paths(REPO_ROOT, rep.cited_paths,
                                       citing_file_dir=path.parent.resolve())
    rep.imperatives_without_rationale = find_imperatives(lines)

    # Fit analysis (new dimension)
    scope_files = scope_map.get(path.resolve(), set())
    descendants = subscope_counts.get(path.resolve(), 0)
    rep.fit = analyze_fit(rep.line_count, scope_files, descendants)

    # Classification (fit issues take priority over drift-only signals)
    if rep.fit.fit_class == "MAYBE-UNNECESSARY":
        rep.classification = "MAYBE-UNNECESSARY"
    elif rep.dead_paths:
        rep.classification = "DRIFTED-FACTUAL"
    elif rep.over_budget:
        rep.classification = "OVER-BUDGET"
    elif rep.fit.fit_class == "UNDER-WROTE":
        rep.classification = "UNDER-WROTE"
    elif rep.fit.fit_class == "OVER-WROTE":
        rep.classification = "OVER-WROTE"
    elif rep.imperatives_without_rationale and rep.drift_score >= threshold:
        rep.classification = "DRIFTED-NORMATIVE"
    elif rep.imperatives_without_rationale:
        rep.classification = "OVER-IMPERATIVE"
    elif rep.drift_score >= threshold:
        rep.classification = "DRIFTED-NORMATIVE"
    else:
        rep.classification = "FRESH"

    return rep


def render(reports: list[FileReport], threshold: float,
           missing_candidates: list[dict] | None = None,
           skip_markers: list[dict] | None = None) -> str:
    by_class: dict[str, list[FileReport]] = defaultdict(list)
    for r in reports:
        by_class[r.classification].append(r)

    out: list[str] = []
    out.append(f"# CLAUDE.md Review — {TODAY.isoformat()}\n")
    out.append(
        "Read-only ceremony. Walks every CLAUDE.md in the repo and classifies "
        "by drift signal + content patterns. Operator-gated; no auto-edits.\n"
    )
    out.append(f"**Threshold**: drift_score ≥ {threshold} flags a file.\n")
    out.append(f"**Total files audited**: {len(reports)}\n")

    classification_order = (
        "MAYBE-UNNECESSARY", "UNDER-WROTE", "OVER-WROTE",
        "DRIFTED-FACTUAL", "DRIFTED-NORMATIVE", "OVER-BUDGET", "OVER-IMPERATIVE",
        "FRESH",
    )

    out.append("\n## Summary by classification\n")
    out.append("| Classification | Count |\n|---|---:|\n")
    for cls in classification_order:
        out.append(f"| {cls} | {len(by_class.get(cls, []))} |\n")
    if missing_candidates is not None:
        out.append(f"| MISSING-CLAUDE-MD (directories with no doc) | {len(missing_candidates)} |\n")
    if skip_markers is not None:
        out.append(f"| OPTED-OUT (explicit `.no-claude.md` markers) | {len(skip_markers)} |\n")

    # Opt-out section — explicit decisions that this directory needs no doc
    if skip_markers:
        out.append("\n## Directories opted out of CLAUDE.md\n")
        out.append(
            "\nEach has a `.no-claude.md` marker file with operator rationale. "
            "These are excluded from MISSING-CLAUDE-MD candidacy. Decision chain "
            "is auditable — surrounding CLAUDE.md updates may reference these.\n\n"
        )
        out.append("| Directory | Marker | Decided | Rationale (excerpt) |\n")
        out.append("|---|---|---|---|\n")
        for m in skip_markers:
            out.append(
                f"| `{m['path']}` | `{m['marker_path']}` | {m.get('decided', m['mtime'])} "
                f"| {m['rationale_summary']} |\n"
            )

    # Missing CLAUDE.md section — directories that probably need one
    if missing_candidates:
        out.append("\n## Directories that may need a CLAUDE.md\n")
        out.append(
            f"\nThresholds: ≥{MISSING_TUNABLES['min_direct_files']} files OR "
            f"≥{MISSING_TUNABLES['min_subdir_count']} subdirs, "
            f"≥{MISSING_TUNABLES['min_distinct_extensions']} distinct complexity extensions, "
            f"nearest ancestor CLAUDE.md ≥{MISSING_TUNABLES['min_ancestor_distance']} levels up. "
            "Re-tunable via `MISSING_TUNABLES` in this script.\n"
        )
        out.append(
            f"\nShowing top {min(len(missing_candidates), MISSING_TUNABLES['max_candidates_reported'])}, "
            "sorted by likely orphaning (file count desc, ancestor distance desc):\n\n"
        )
        out.append("| Directory | Direct files | Subdirs | Distinct exts | Ancestor (distance) |\n")
        out.append("|---|---:|---:|---|---|\n")
        for c in missing_candidates[:MISSING_TUNABLES["max_candidates_reported"]]:
            ext_str = " ".join(c["extension_list"]) or "—"
            out.append(
                f"| `{c['path']}` | {c['direct_files']} | {c['subdirs']} "
                f"| {c['distinct_extensions']} ({ext_str}) "
                f"| `{c['ancestor']}` ({c['ancestor_distance']}↑) |\n"
            )
        if len(missing_candidates) > MISSING_TUNABLES["max_candidates_reported"]:
            out.append(
                f"\n_...and {len(missing_candidates) - MISSING_TUNABLES['max_candidates_reported']} "
                "more — tune `MISSING_TUNABLES.max_candidates_reported` if you want to see them all._\n"
            )

    out.append("\n## Per-file findings\n")
    for cls in classification_order:
        bucket = by_class.get(cls, [])
        if not bucket:
            continue
        out.append(f"\n### {cls} ({len(bucket)})\n")
        for r in sorted(bucket, key=lambda x: (-x.drift_score, x.rel)):
            mtime_str = r.mtime.date().isoformat() if r.mtime else "—"
            out.append(f"\n#### `{r.rel}` — drift {r.drift_score:.2f}, mtime {mtime_str}, {r.line_count} lines\n")
            out.append(
                f"- Signals: {r.direct_edits} direct edits, {r.scope_edits} scope edits, "
                f"**{r.structural_edits} structural ops** (mv/cp/rm) since last audit\n"
            )
            out.append(
                f"- Fit: {r.fit.scope_file_count} files in direct scope, "
                f"{r.fit.scope_distinct_extensions} distinct extensions, "
                f"{r.fit.scope_subscope_count} sub-CLAUDE.md scopes; "
                f"ratio {r.fit.fit_ratio} (expected ~{r.fit.expected_lines} lines)\n"
            )
            if r.over_budget:
                out.append(f"- **Over budget**: {r.line_count} lines (warn at {LINE_BUDGET_WARN})\n")
            if r.dead_paths:
                out.append(f"- **Dead paths cited** ({len(r.dead_paths)}):\n")
                for dp in r.dead_paths[:10]:
                    out.append(f"  - `{dp}`\n")
                if len(r.dead_paths) > 10:
                    out.append(f"  - ...and {len(r.dead_paths) - 10} more.\n")
            if r.imperatives_without_rationale:
                out.append(
                    f"- **Imperatives without rationale** ({len(r.imperatives_without_rationale)}; "
                    "consider adding a `because…` or removing):\n"
                )
                for line_no, txt in r.imperatives_without_rationale[:8]:
                    snippet = txt[:140] + ("…" if len(txt) > 140 else "")
                    out.append(f"  - L{line_no}: {snippet}\n")
                if len(r.imperatives_without_rationale) > 8:
                    out.append(f"  - ...and {len(r.imperatives_without_rationale) - 8} more.\n")
            if r.notes:
                for n in r.notes:
                    out.append(f"- Note: {n}\n")

    out.append("\n---\n")
    out.append(
        "_Operator-gated. To act: revise flagged sections in their CLAUDE.md files. "
        "After making changes, optionally reset signals for that file by editing "
        "`.claude/memory-kit/claude-md-drift.json` (set `last_audited` to today and "
        "zero out `direct_edits` / `scope_edits`); the next audit will start from a "
        "fresh signal baseline._\n"
    )
    return "".join(out)


def reset_signal_for_audited(drift_store: dict, audited_files: list[FileReport]) -> dict:
    """After producing the report, mark each file's last_audited=today.

    This is benign — operator can revise CLAUDE.md, but the audit itself counts
    as having *seen* the current state. Edits counters reset so future signals
    are net-new since this run.
    """
    today_iso = TODAY.isoformat()
    for r in audited_files:
        entry = drift_store.get("files", {}).setdefault(r.rel, {})
        entry["last_audited"] = today_iso
        entry["direct_edits"] = 0
        entry["scope_edits"] = 0
        entry["structural_edits"] = 0
        entry["lines_changed_in_scope"] = 0
        entry["drift_score"] = 0.0
        entry["rescore_counter"] = 0
    return drift_store


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--threshold", type=float, help="Override drift threshold")
    parser.add_argument("--no-reset", action="store_true", help="Don't reset signal counters after audit")
    parser.add_argument("--output-dir", help=f"Override output dir (default: {DEFAULT_OUT_ROOT}/<YYYY-MM-DD>/)")
    args = parser.parse_args()

    drift_store = load_drift_store()
    threshold = args.threshold if args.threshold is not None else float(drift_store.get("threshold", DEFAULT_DRIFT_THRESHOLD))

    files = find_claude_md_files()
    missing_candidates = find_missing_claude_md_candidates(REPO_ROOT, files)
    skip_markers = find_skip_markers(REPO_ROOT)
    scope_map = compute_direct_scopes(files)
    # subscope_counts: how many descendant CLAUDE.md files each one has
    subscope_counts: dict[Path, int] = {}
    for owner in files:
        owner_dir = owner.parent.resolve()
        count = 0
        for other in files:
            if other == owner:
                continue
            try:
                other.parent.resolve().relative_to(owner_dir)
                count += 1
            except ValueError:
                continue
        subscope_counts[owner.resolve()] = count
    reports = [analyze_file(f, drift_store, threshold, scope_map, subscope_counts) for f in files]

    out_dir = Path(args.output_dir) if args.output_dir else DEFAULT_OUT_ROOT / TODAY.isoformat()
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "claude-md-audit.md"
    out_path.write_text(
        render(reports, threshold, missing_candidates, skip_markers), encoding="utf-8"
    )

    if not args.no_reset:
        updated = reset_signal_for_audited(drift_store, reports)
        _store.save_json(DRIFT_STORE, updated)

    print(f"audited: {len(reports)} CLAUDE.md files")
    cls_counts = defaultdict(int)
    for r in reports:
        cls_counts[r.classification] += 1
    for cls, n in sorted(cls_counts.items()):
        print(f"  {cls}: {n}")
    print(f"missing-claude-md candidates: {len(missing_candidates)}")
    print(f"threshold: {threshold}")
    print(f"report: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
