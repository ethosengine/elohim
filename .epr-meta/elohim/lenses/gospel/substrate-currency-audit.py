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
import subprocess
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

# "worktrees" — .claude/worktrees/ is gitignored ephemeral agent scratch; its CLAUDE.md
# copies are duplicates of real surfaces and inflate the ranking with phantom rows.
# "repos" — genesis/research/repos/ holds untracked upstream clones; their CLAUDE.mds are
# other projects' gospel, not ours to audit or rewrite.
CLAUDE_MD_EXCLUDE = (
    "node_modules",
    "target",
    ".cargo-target-pool",
    "sophia",
    "held",
    "worktrees",
    "repos",
)


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


def _blank_out(m: re.Match[str]) -> str:
    """Replace a matched region with its own newline count, preserving line numbers."""
    return "\n" * m.group(0).count("\n")


def extract_body(text: str) -> str:
    """Blank out YAML frontmatter + fenced code blocks. Leaves prose + inline backticks.

    Stripped regions are replaced by an equal number of newlines rather than
    deleted, so a finding's line number matches the line in the actual file.
    Deleting them shifted every reported line number by the size of the
    frontmatter plus every preceding code block, which made findings hard to
    locate in the surface being audited.
    """
    body = FRONTMATTER_RE.sub(_blank_out, text, count=1)
    body = FENCED_CODE_RE.sub(_blank_out, body)
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


# Non-path token classes that the backtick sweep picks up but which are never
# filesystem claims. Excluding them at the predicate is what keeps the ranked
# drift list honest — see the header note on measurement discipline.
PROTOCOL_STR_RE = re.compile(r"^/[a-z0-9-]+(/[a-z0-9._-]+)*/\d+\.\d+\.\d+$")
SLASH_CMD_RE = re.compile(r"^/[a-z0-9][a-z0-9-]*$")
BARE_EXT_RE = re.compile(r"^\.\w+$")
# All-lowercase slash-joined words with no extension: vocabulary triples like
# `own/ownership/sovereign`, `steward/contributor/authored`, or an HTTP route.
VOCAB_OR_ROUTE_RE = re.compile(r"^[a-z][a-z0-9-]*(/[a-z][a-z0-9-]*)+$")


def _slash_command_names(root: Path) -> set[str]:
    """Skill + slash-command names, so `/converge` is not read as a directory."""
    names = {p.parent.name for p in (root / ".claude" / "skills").glob("*/SKILL.md")}
    names |= {p.stem for p in (root / ".claude" / "commands").glob("*.md")}
    return names


def looks_like_path(tok: str) -> bool:
    """Shape-only pre-filter: could this token possibly be a path claim?

    Only classes that are never a path REGARDLESS of the filesystem are
    excluded here. Anything whose path-ness depends on whether it resolves is
    decided later by is_non_filesystem_shape, AFTER a resolution attempt —
    deciding "not a path" before trying to resolve is what silently hid real
    drift: an all-lowercase directory citation (`elohim/holochain/dna/bogus`),
    the single most common citation style in this corpus, was classified as
    vocabulary and never checked at all.
    """
    if tok.startswith(("http://", "https://", "//")):
        return False
    if "*" in tok:
        return False
    if tok.startswith("-"):
        return False
    if "::" in tok:  # Rust paths like `crate::module`
        return False
    if "..." in tok or "…" in tok:  # `app/.../foo.ts` — an elided abbreviation
        return False
    if BARE_EXT_RE.match(tok):  # a bare `.ts` / `.md` mention
        return False
    if "/" in tok:
        return True
    if tok.endswith(KNOWN_EXTS):
        return True
    if tok.endswith("/"):
        return True
    return False


def _has_file_extension(clean: str) -> bool:
    base = os.path.basename(clean.rstrip("/"))
    return base.endswith(KNOWN_EXTS)


def is_non_filesystem_shape(
    tok: str, cmd_names: frozenset[str], top_level: frozenset[str], root: Path | None = None
) -> bool:
    """Is an UNRESOLVED token something that was never a filesystem claim?

    Applied only after resolution fails, so a real-but-broken path is still
    reported. The ordering matters: `/app/elohim-app/src/gone.ts` is a genuine
    broken repo path written with a leading slash and MUST surface, while
    `/db/content` (HTTP route) and `/opt/rust/cargo/bin/cargo-nextest`
    (absolute host path, asserted absent) must not.
    """
    if PROTOCOL_STR_RE.match(tok):  # libp2p protocol id, /elohim/sync/2.0.0
        return True
    if SLASH_CMD_RE.match(tok) and tok[1:] in cmd_names:  # `/converge`, `/shift`
        return True
    clean = tok.strip("/")
    if not clean:
        return True
    # Leading slash and no file extension → HTTP route or absolute host path
    # — UNLESS the first segment names a real top-level repo directory. That
    # guard mirrors the two-segment test the VOCAB_OR_ROUTE_RE branch below
    # already applies, and for the same reason: an absolute-form directory
    # citation like `/genesis/docs/superpowers/sprints` (a renamed or deleted
    # subdirectory) must still fall through to be checked, not be dismissed
    # outright as a route just because it starts with `/`.
    if tok.startswith("/") and not _has_file_extension(clean):
        if clean.split("/", 1)[0] not in top_level:
            return True
    # Vocabulary triple (`own/ownership/sovereign`, `steward/contributor/authored`)
    # or bare route: all-lowercase segments, no extension — dismissed ONLY when
    # its leading TWO segments do not form a real path. A one-segment guard is
    # not enough: `steward/` is a real top-level directory, so `steward/…`
    # vocabulary would be reported, while a two-segment test separates it
    # cleanly from `elohim/holochain/…` (where `elohim/holochain` is real).
    # This keeps genuine directory citations auditable — the thing a blanket
    # vocabulary exclusion silently destroyed.
    if VOCAB_OR_ROUTE_RE.match(clean):
        parts = clean.split("/")
        if parts[0] not in top_level:
            return True
        if root is not None and len(parts) >= 2 and not (root / parts[0] / parts[1]).exists():
            return True
    return False


def build_path_suffix_index(root: Path) -> frozenset[str]:
    """Every tracked path AND every trailing segment-slice of it.

    A gospel surface routinely cites a path relative to a context established
    earlier in the prose (`p2p/mod.rs` under a heading about elohim-storage),
    not relative to the repo root or to its own directory. Resolving only
    against the root reports those as drift when they are correct — which
    inverted the ranked drift list (a surface whose 50 findings were all
    context-relative outranked surfaces with real drift). Suffix resolution is
    deliberately permissive: it answers "does this path shape exist", which is
    the claim a prose citation actually makes.

    KNOWN PERMISSIVENESS: a suffix match confirms the path SHAPE exists
    somewhere, not that it exists where the prose implies. `elohim/projections/`
    resolves because `.epr-meta/elohim/projections/` is real, even though the
    top-level `elohim/` has no `projections/`. That is the deliberate trade:
    prose citations genuinely are context-relative, and the alternative
    (root-only) produced 80% false positives. Resolution is ordered
    root → surface-relative → suffix, so the more precise conventions win first.
    """
    try:
        out = subprocess.run(
            ["git", "ls-files"], cwd=root, capture_output=True, text=True, timeout=120
        )
        tracked = [ln for ln in out.stdout.split("\n") if ln]
    except (OSError, subprocess.SubprocessError):
        tracked = []
    # Every real path: each tracked file plus each of its ancestor directories.
    real_paths: set[str] = set()
    for rel in tracked:
        parts = rel.split("/")
        real_paths.add(rel)
        for k in range(1, len(parts)):
            real_paths.add("/".join(parts[:k]))
    # Then every true trailing suffix OF a real path (dirs in both forms).
    suffixes: set[str] = set()
    for p in real_paths:
        parts = p.split("/")
        for start in range(len(parts)):
            suf = "/".join(parts[start:])
            suffixes.add(suf)
            suffixes.add(suf + "/")
    return frozenset(suffixes)


def path_exists_in_repo(
    root: Path,
    claim: str,
    surface_dir: Path | None = None,
    suffix_index: frozenset[str] = frozenset(),
) -> bool:
    """Resolve a cited path against every convention a gospel surface uses.

    Order: repo-root-relative, then relative to the citing surface's own
    directory (the per-crate CLAUDE.md convention), then as a suffix of any
    tracked path (the context-relative prose convention), then an absolute
    host path, then a bare filename anywhere.
    """
    clean = claim.strip("/")
    if not clean:
        return False
    if (root / clean).exists():
        return True
    # Relative to the citing surface's directory — how a per-crate CLAUDE.md
    # cites `src/routes/mod.rs`.
    if surface_dir is not None and (surface_dir / clean).exists():
        return True
    # Context-relative prose citation.
    if clean in suffix_index or clean + "/" in suffix_index:
        return True
    # Absolute host path (e.g. a config dir outside the repo).
    if claim.startswith("/") and Path(claim).exists():
        return True
    if "/" not in clean and "." in clean:
        # Bare filename — see if it exists anywhere in repo
        try:
            for _ in root.rglob(clean):
                return True
        except OSError:
            pass
        return False
    return False


def check_path_existence(
    root: Path,
    body: str,
    surface_dir: Path | None = None,
    suffix_index: frozenset[str] = frozenset(),
    cmd_names: frozenset[str] = frozenset(),
    top_level: frozenset[str] = frozenset(),
) -> list[dict]:
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
        if path_exists_in_repo(root, tok, surface_dir, suffix_index):
            continue
        # Resolution failed — only NOW may the token be dismissed as a shape
        # that was never a filesystem claim.
        if is_non_filesystem_shape(tok, cmd_names, top_level, root):
            continue
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
NEGATION_WINDOW = 60  # chars before the match, within its clause, to scan for framing negation
BARE_NEGATION_WINDOW = 20  # chars — bare no/not are common general-purpose words; only an
# occurrence right next to the match plausibly negates it. A 60-char window let an
# unrelated "no"/"not" earlier in the same (unpunctuated) clause suppress a real finding
# ("There is no doubt the module is currently under active development" has "no"
# negating "doubt", not "currently"). "never"/"avoid"/"instead of"/"rather than" are
# deliberate authorial framing words, not general negators, so they keep the wide window.
STRONG_NEGATION_RE = re.compile(r"\b(never|avoid|instead of|rather than)\b", re.IGNORECASE)
BARE_NEGATION_RE = re.compile(r"\b(not|no)\b", re.IGNORECASE)

# `in-flight` is also the PROPER NOUN of a shipped mechanism ("the in-flight
# hook", "in-flight memory-to-code coherence"). Used as a compound modifier
# before one of these nouns it names durable architecture, not sprint state.
COMPOUND_MODIFIER_NOUNS = (
    "edge", "hook", "hooks", "accumulator", "coherence", "signal", "signals",
    "layer", "tier", "check", "checks", "audit", "gate", "counter", "counters",
    "memory-to-code", "memory↔code", "design", "spec", "mechanism", "pass",
    "plane", "path", "invalidation", "reconciliation",
)
_COMPOUND_RE = re.compile(
    r"\bin[- ]flight[- ]+(" + "|".join(COMPOUND_MODIFIER_NOUNS) + r")\b", re.IGNORECASE
)


def _is_quoted_example(body: str, start: int, end: int) -> bool:
    """True when the match sits inside a quoted anti-pattern example.

    Gospel surfaces teach this very discipline by quoting what NOT to write
    (`never "currently noisy"`). The quoted illustration is correct authoring,
    so flagging it inverts the rule — the discipline's own statement of itself
    became its highest-ranked violation.
    """
    line_start = body.rfind("\n", 0, start) + 1
    line_end = body.find("\n", end)
    line = body[line_start : line_end if line_end != -1 else len(body)]
    rel = start - line_start
    # Inside an inline code span (`…`) the phrase is an identifier — a filename
    # like `2026-05-28-in-flight-memory-coherence-design.md`, a flag, a symbol —
    # never narrative status. An odd backtick count before the match means the
    # match opened inside one.
    if line[:rel].count("`") % 2 == 1:
        return True
    # Paired quote delimiters that straddle the match mean it sits inside a
    # quotation. Straight `"` is symmetric — an odd count before the match
    # means the match opened inside one. Curly quotes are NOT symmetric: `“`
    # opens and `”` closes, so a correctly-paired “…” never produces an odd
    # count of `“` alone (the old single-character parity test could never
    # fire) — track opens/closes separately and compare. The ASCII apostrophe
    # is excluded when it sits directly between two letters — a possessive
    # (`doorway's`) or contraction (`don't`) — since that shape is never a
    # quote delimiter and made ordinary possessive prose read as "quoted";
    # what's left of `'` still pairs like any other quote mark.
    if line.count('"') >= 2 and line[:rel].count('"') % 2 == 1:
        return True
    quote_apostrophes = [
        i
        for i, ch in enumerate(line)
        if ch == "'" and not (0 < i < len(line) - 1 and line[i - 1].isalpha() and line[i + 1].isalpha())
    ]
    if len(quote_apostrophes) >= 2 and sum(1 for i in quote_apostrophes if i < rel) % 2 == 1:
        return True
    opens_before = line[:rel].count("“")
    closes_before = line[:rel].count("”")
    if opens_before > closes_before and "”" in line[rel:]:
        return True
    # Explicit negation framing ("never X", "avoid X") — but only within the
    # SAME CLAUSE as the match. A line-wide search suppressed real drift
    # whenever an unrelated earlier clause happened to contain a negation:
    # "The design is not finalized, and the module is currently under dev."
    # has `not` negating `finalized`, not the flagged `currently`. Bare
    # "no"/"not" get a much tighter trailing window within that clause — see
    # BARE_NEGATION_WINDOW — since they are common general-purpose words that
    # routinely appear elsewhere in an unpunctuated clause without negating
    # the flagged phrase at all.
    clause = re.split(r"[,;:.—]", line[:rel])[-1]
    if STRONG_NEGATION_RE.search(clause[-NEGATION_WINDOW:]):
        return True
    if BARE_NEGATION_RE.search(clause[-BARE_NEGATION_WINDOW:]):
        return True
    return False


def check_process_status(body: str) -> list[dict]:
    findings: list[dict] = []
    for pattern, label in PROCESS_STATUS_PATTERNS:
        for m in pattern.finditer(body):
            start = max(0, m.start() - EXEMPT_SUBSTR_WINDOW)
            end = min(len(body), m.end() + EXEMPT_SUBSTR_WINDOW)
            window = body[start:end]
            if any(s in window for s in EXEMPT_SUBSTRINGS):
                continue
            # Proper-noun compound modifier: "the in-flight hook".
            if label == "in-flight" and _COMPOUND_RE.match(body, m.start()):
                continue
            if _is_quoted_example(body, m.start(), m.end()):
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


def audit_surface(
    path: Path,
    root: Path,
    family: str,
    suffix_index: frozenset[str] = frozenset(),
    cmd_names: frozenset[str] = frozenset(),
    top_level: frozenset[str] = frozenset(),
) -> tuple[SurfaceReport, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    body = extract_body(text)
    line_count = text.count("\n") + 1
    rep = SurfaceReport(
        path=str(path.relative_to(root)),
        family=family,
        line_count=line_count,
        drift_path=check_path_existence(
            root, body, path.parent, suffix_index, cmd_names, top_level
        ),
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
    lines.append("- **Path existence**: backticked tokens that look path-like (contain `/`, end in a known extension, or end in `/`). A claim counts as resolved if it exists relative to the repo root, relative to the **citing surface's own directory** (the per-crate CLAUDE.md convention), as a **suffix of any tracked path** (the context-relative prose convention — `p2p/mod.rs` under an elohim-storage heading), as an absolute host path, or as a bare filename anywhere. Excluded as never-filesystem-claims: URLs, `*`-wildcards, `crate::module`, elided abbreviations (`app/.../foo.ts`), bare extensions (`.ts`), libp2p protocol ids (`/elohim/sync/2.0.0`), slash-commands (`/converge`), and lowercase vocabulary triples or bare HTTP routes (`own/ownership/sovereign`).")
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

    suffix_index = build_path_suffix_index(REPO_ROOT)
    cmd_names = frozenset(_slash_command_names(REPO_ROOT))
    top_level = frozenset(q.name for q in REPO_ROOT.iterdir())

    surfaces: list[SurfaceReport] = []
    surface_texts: dict[Path, str] = {}
    for path in agents:
        rep, text = audit_surface(path, REPO_ROOT, "agent", suffix_index, cmd_names, top_level)
        surfaces.append(rep)
        surface_texts[path] = text
    for path in skills:
        rep, text = audit_surface(path, REPO_ROOT, "skill", suffix_index, cmd_names, top_level)
        surfaces.append(rep)
        surface_texts[path] = text
    for path in claude_mds:
        rep, text = audit_surface(path, REPO_ROOT, "claude-md", suffix_index, cmd_names, top_level)
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
