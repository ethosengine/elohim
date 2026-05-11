#!/usr/bin/env python3
"""
Sprint-result digest generator — periodic legibility for shift artifacts.

Finds sprint-result-shaped markdown files across the repo, extracts their
structured fields (objective, outcome, what shipped, what blocked,
learnings, open questions), and writes a single human-scan-friendly digest
sorted by date (newest first).

The OPERATOR (or a future tool) decides what graduates to memory or specs
from the digest. This tool is read-only: it surfaces the raw signal so it
stops being scattered.

Sibling tool of `/cleanup` in the `/memory-kit` toolkit.
Spec reference: genesis/docs/superpowers/specs/2026-05-10-memory-lifecycle-design.md
The corpus axis named in the v1 inputs of `/dream` ("Sprint-result artifacts
from `/shift` and `/deliver`") is the surface this tool digests.

Output: .claude/memory-kit/<YYYY-MM-DD>/sprint-digest.md

Pure stdlib. No LLM. No network. No file mutation outside its output.
"""

from __future__ import annotations

import argparse
import os
import re
import sys
from collections import Counter
from dataclasses import dataclass, field
from datetime import date, datetime
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
TODAY = date.today()

# Where sprint-result files live. Documented patterns observed in this repo:
#   - .claude/shifts/<shift-id>.sprint-result.md           (older flat convention)
#   - .claude/shifts/<shift-id>/sprint-result.md           (newer directory convention)
#   - genesis/docs/shifts/**/*sprint-result*.md            (potential future home)
#   - genesis/docs/retrospectives/**/*.md                  (currently template-only)
# Worktree shifts under .claude/worktrees/**/.claude/shifts/ are excluded by
# default — they're transient/parallel and not part of the canonical record.
SPRINT_GLOBS = [
    ".claude/shifts/*.sprint-result.md",
    ".claude/shifts/*/sprint-result.md",
    ".claude/shifts/*/deliver-result.md",
    ".claude/shifts/*.deliver-result.md",
    "genesis/docs/shifts/**/sprint-result*.md",
    "genesis/docs/shifts/**/shift-result*.md",
    "genesis/docs/retrospectives/**/*.md",
]

# Excluded path fragments (templates, worktree shadows, archive).
EXCLUDE_FRAGMENTS = (
    "TEMPLATE",
    ".claude/worktrees/",
    ".claude/archive/",
    "memory-kit/",  # don't recursively digest our own output
)

# Heading patterns we try to extract. Case-insensitive. Each tuple is
# (canonical_field_name, list_of_regex_alternatives_for_the_heading_text).
# The script captures the body from the heading until the next ## or higher.
FIELD_PATTERNS: list[tuple[str, list[str]]] = [
    ("objective", [r"objective", r"what\s+was\s+promised", r"feature\s+promise"]),
    ("status", [r"status", r"disposition", r"final\s+(?:tier-?\d\s+)?verdict"]),
    ("outcome", [r"outcome", r"what\s+was\s+achieved", r"result", r"final\s+(?:tier-?\d\s+)?verdict"]),
    ("shipped", [
        r"what\s+(?:was\s+)?shipped",
        r"(?:the\s+)?\d+\s+fixes?\s+that\s+landed",
        r"glue\s+written",
        r"delivery\s+commit\s+lineage",
        r"changes?\s+landed",
        r"plan-?vs-?delivery\s+match",
    ]),
    ("blocked", [
        r"(?:what\s+)?blocked",
        r"bail\s+reason",
        r"known\s+gaps?",
        r"interruption",
        r"measurement\s+trustworthiness",
    ]),
    ("learnings", [
        r"(?:key\s+)?learnings?",
        r"discoveries?\s+worth\s+memory",
        r"observed\s+anti-?patterns?",
        r"lessons?",
        r"discoveries?",
    ]),
    ("open_questions", [
        r"open\s+(?:questions?|follow-?ups?)",
        r"follow-?ups?",
        r"next\s+steps?",
        r"proposed\s+next\s+step",
        r"question\s+for\s+operator",
    ]),
    ("decisions", [
        r"decisions?",
        r"pivots?",
        r"judgment\s+calls?\s+log",
    ]),
    ("iterations", [
        r"iterations?(?:\s+run)?",
        r"total\s+iterations",
        r"trajectory\s+summary",
    ]),
]

# Theme detection: words to ignore when frequency-counting cross-cutting topics.
# Heavily biased toward dev-shift vocabulary so the surfaced themes are
# project-domain words ("iroh", "doorway", "alpha"), not generic noise.
THEME_STOPWORDS = {
    # generic English
    "the", "a", "an", "is", "of", "to", "and", "in", "for", "with", "that",
    "this", "it", "as", "on", "at", "by", "from", "or", "are", "be", "was",
    "were", "not", "can", "if", "then", "but", "which", "you", "your", "our",
    "will", "would", "should", "could", "may", "might", "do", "does", "did",
    "have", "has", "had", "all", "any", "some", "each", "no", "yes", "than",
    "too", "very", "also", "more", "most", "less", "other", "such", "only",
    "own", "same", "what", "when", "where", "who", "how", "why", "while",
    "into", "out", "up", "down", "over", "under", "here", "there", "now",
    "just", "like", "see", "use", "used", "via", "per", "vs", "etc",
    "between", "across", "within", "without", "before", "after", "during",
    # dev-shift jargon that appears everywhere — uninformative as a theme
    "shift", "sprint", "result", "iteration", "iterations", "objective",
    "build", "builds", "ci", "pipeline", "pipelines", "stage", "stages",
    "fresh", "trigger", "stable", "stability", "measurement", "measure",
    "predicate", "delivered", "deliver", "done", "bail", "bailed", "interrupted",
    "outcome", "status", "disposition", "verdict", "fix", "fixed", "fixing",
    "feature", "features", "scenario", "scenarios", "test", "tests", "testing",
    "commit", "commits", "branch", "push", "pushed", "land", "landed", "landing",
    "wall", "clock", "min", "minutes", "iter", "step", "steps",
    "files", "file", "path", "paths", "lines", "line", "code",
    "json", "yaml", "html", "log", "logs", "error", "errors", "failure",
    "failures", "fail", "failed", "failing", "pass", "passed", "passing",
    "green", "red", "unstable", "success",
    "with", "from", "into", "this", "that",
}

WORD_RE = re.compile(r"[a-z][a-z0-9-]{2,}")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*$", re.MULTILINE)
FRONTMATTER_DATE_RE = re.compile(r"^date\s*:\s*(\d{4}-\d{2}-\d{2})", re.MULTILINE | re.IGNORECASE)
DATE_PREFIX_RE = re.compile(r"(\d{4}-\d{2}-\d{2})")
ISO_TIMESTAMP_RE = re.compile(r"(\d{4}-\d{2}-\d{2})T\d{2}-\d{2}")


@dataclass
class SprintEntry:
    path: Path
    title: str
    file_date: date
    date_source: str  # "filename" | "frontmatter" | "mtime"
    fields: dict[str, tuple[str, int]] = field(default_factory=dict)  # field_name -> (body, lineno)
    raw_text: str = ""

    @property
    def rel_path(self) -> Path:
        try:
            return self.path.relative_to(REPO_ROOT)
        except ValueError:
            return self.path


def gather_sprint_files() -> list[Path]:
    """Find all sprint-result-shaped files matching SPRINT_GLOBS, deduped."""
    seen: set[Path] = set()
    out: list[Path] = []
    for pattern in SPRINT_GLOBS:
        for p in REPO_ROOT.glob(pattern):
            if not p.is_file():
                continue
            if any(frag in str(p) for frag in EXCLUDE_FRAGMENTS):
                continue
            resolved = p.resolve()
            if resolved in seen:
                continue
            seen.add(resolved)
            out.append(p)
    return out


def extract_date(path: Path, text: str) -> tuple[date, str]:
    """Best-effort date extraction. Tries (in order): filename ISO timestamp,
    filename YYYY-MM-DD prefix, frontmatter date:, mtime fallback."""
    name = path.name
    parent = path.parent.name

    # Try filename ISO timestamp (e.g. 2026-05-09T16-30-...)
    m = ISO_TIMESTAMP_RE.search(name) or ISO_TIMESTAMP_RE.search(parent)
    if m:
        try:
            return date.fromisoformat(m.group(1)), "filename"
        except ValueError:
            pass

    # Try plain date prefix (filename or parent)
    for source in (name, parent):
        m = DATE_PREFIX_RE.search(source)
        if m:
            try:
                return date.fromisoformat(m.group(1)), "filename"
            except ValueError:
                pass

    # Try frontmatter date
    m = FRONTMATTER_DATE_RE.search(text[:500])
    if m:
        try:
            return date.fromisoformat(m.group(1)), "frontmatter"
        except ValueError:
            pass

    # Fall back to mtime
    return date.fromtimestamp(path.stat().st_mtime), "mtime"


def extract_title(text: str, path: Path) -> str:
    """First-heading title or filename-derived title."""
    for line in text.splitlines():
        line = line.strip()
        if line.startswith("# "):
            return line[2:].strip()
    # Fallback: derive from filename
    stem = path.stem
    if stem == "sprint-result" or stem == "deliver-result":
        # use the parent directory name
        return path.parent.name
    # Strip common suffixes
    stem = re.sub(r"\.(?:sprint|deliver|shift)-result$", "", stem)
    return stem


def extract_fields(text: str) -> dict[str, tuple[str, int]]:
    """Walk all headings; for each canonical field, capture the first matching
    section's body and starting line number.

    Returns: {canonical_field_name: (body_text, line_number_of_heading)}.
    """
    # Build line index: char position -> line number (1-based)
    line_starts = [0]
    for i, ch in enumerate(text):
        if ch == "\n":
            line_starts.append(i + 1)

    def char_to_line(pos: int) -> int:
        # binary search
        lo, hi = 0, len(line_starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if line_starts[mid] <= pos:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    # Collect all headings (level, normalized text, char-pos)
    headings: list[tuple[int, str, int]] = []
    for m in HEADING_RE.finditer(text):
        level = len(m.group(1))
        h_text = m.group(2).strip().lower()
        # Strip markdown-style decorations from heading text
        h_text = re.sub(r"[*_`]", "", h_text)
        headings.append((level, h_text, m.start()))

    out: dict[str, tuple[str, int]] = {}

    for canonical, alt_patterns in FIELD_PATTERNS:
        if canonical in out:
            continue
        combined = re.compile(r"^(?:" + "|".join(alt_patterns) + r")\b", re.IGNORECASE)
        for i, (level, h_text, pos) in enumerate(headings):
            if level < 2:  # skip H1 title
                continue
            if combined.match(h_text):
                # Body runs from end-of-this-heading-line to start of next H<=level
                body_start = text.find("\n", pos) + 1
                body_end = len(text)
                for j in range(i + 1, len(headings)):
                    if headings[j][0] <= level:
                        body_end = headings[j][2]
                        break
                body = text[body_start:body_end].strip()
                out[canonical] = (body, char_to_line(pos))
                break

    return out


def truncate(body: str, max_chars: int = 600) -> str:
    """Trim body to fit in a digest entry. Prefer cutting at a sentence/line."""
    body = body.strip()
    if len(body) <= max_chars:
        return body
    # cut at line/sentence boundary near the limit
    cut = body[:max_chars]
    for sep in ("\n\n", "\n", ". "):
        idx = cut.rfind(sep)
        if idx > max_chars * 0.5:
            return cut[: idx + len(sep)].rstrip() + " ..."
    return cut.rstrip() + " ..."


def detect_themes(entries: list[SprintEntry], min_occurrences: int = 3) -> list[tuple[str, int]]:
    """Word-frequency across titles + objective + status fields. Surface
    words that appear in >= min_occurrences DISTINCT sprints. No stemming,
    no semantic analysis — by design."""
    sprint_word_sets: list[set[str]] = []
    for e in entries:
        bag = e.title.lower() + " "
        for fname in ("objective", "status", "outcome"):
            if fname in e.fields:
                # only the leading prose, not whole body
                bag += " " + e.fields[fname][0][:400].lower()
        words = {w for w in WORD_RE.findall(bag) if w not in THEME_STOPWORDS}
        sprint_word_sets.append(words)

    counter: Counter[str] = Counter()
    for words in sprint_word_sets:
        counter.update(words)

    return [(w, c) for w, c in counter.most_common(40) if c >= min_occurrences]


def parse_open_questions(body: str) -> list[str]:
    """Extract individual question/follow-up items from a body. Looks for
    numbered lists, bulleted lists, or paragraphs starting with a strong
    pattern (e.g. **Question:**)."""
    items: list[str] = []
    lines = body.splitlines()
    current: list[str] = []

    def flush():
        if current:
            joined = " ".join(s.strip() for s in current).strip()
            if joined and len(joined) > 10:
                items.append(joined)
            current.clear()

    item_start_re = re.compile(r"^\s*(?:\d+\.|[-*])\s+(.+)")
    for line in lines:
        m = item_start_re.match(line)
        if m:
            flush()
            current.append(m.group(1))
        elif line.strip() == "":
            flush()
        elif current:
            # continuation of current item
            current.append(line)
    flush()

    return [truncate(it, max_chars=300) for it in items]


def render_entry(idx: int, entry: SprintEntry) -> str:
    out: list[str] = []
    out.append(f"### {idx}. {entry.title}")
    out.append("")
    out.append(f"- **Path**: `{entry.rel_path}`")
    out.append(f"- **Date**: {entry.file_date.isoformat()} _(source: {entry.date_source})_")

    status_field = entry.fields.get("status")
    if status_field:
        # take first non-empty line of status section
        first_line = next((ln.strip() for ln in status_field[0].splitlines() if ln.strip()), "")
        if first_line:
            # strip leading "**Status:**" type prefixes
            cleaned = re.sub(r"^\*?\*?(?:status|disposition)\*?\*?\s*:?\s*", "", first_line, flags=re.IGNORECASE)
            out.append(f"- **Status**: {truncate(cleaned, max_chars=200)}")

    out.append("")

    section_order = [
        ("objective", "Objective"),
        ("outcome", "Outcome"),
        ("shipped", "What shipped"),
        ("blocked", "What blocked"),
        ("decisions", "Decisions / pivots"),
        ("learnings", "Key learnings"),
        ("open_questions", "Open questions / followups"),
        ("iterations", "Iteration trajectory"),
    ]
    rendered_any = False
    for fname, label in section_order:
        if fname not in entry.fields:
            continue
        body, lineno = entry.fields[fname]
        body = body.strip()
        if not body:
            continue
        rendered_any = True
        out.append(f"**{label}** _(line {lineno})_:")
        out.append("")
        out.append(truncate(body))
        out.append("")

    if not rendered_any:
        out.append("_(no recognizable sections — heading structure unfamiliar; read the file)_")
        out.append("")

    out.append("---")
    out.append("")
    return "\n".join(out)


def render_digest(entries: list[SprintEntry], out_path: Path, since: date | None) -> str:
    out: list[str] = []
    out.append(f"# Sprint Digest — {TODAY.isoformat()}")
    out.append("")
    out.append("Generated by `.claude/scripts/memory-kit/sprint-distill.py`. Read-only.")
    out.append("")
    out.append(
        "Periodic digest of sprint-result artifacts. Scattered across the repo, "
        "they carry real engineering signal — what shipped, what blocked, what "
        "was learned — but lose visibility quickly. This file makes them legible "
        "in one place. The operator (or a future agent) decides what graduates "
        "to memory or specs."
    )
    out.append("")

    if entries:
        date_range = f"{entries[-1].file_date.isoformat()} to {entries[0].file_date.isoformat()}"
    else:
        date_range = "n/a"

    out.append(f"- **Sprint files digested**: {len(entries)}")
    out.append(f"- **Date range**: {date_range}")
    if since:
        out.append(f"- **Filtered**: only sprints on or after {since.isoformat()}")
    try:
        rel_out = out_path.relative_to(REPO_ROOT)
    except ValueError:
        rel_out = out_path
    out.append(f"- **Output**: `{rel_out}`")
    out.append("")
    out.append("---")
    out.append("")

    if not entries:
        out.append("_No sprint-result files found matching the search patterns._")
        out.append("")
        out.append("Search patterns (relative to repo root):")
        for pat in SPRINT_GLOBS:
            out.append(f"- `{pat}`")
        return "\n".join(out)

    # Sprints, newest-first
    out.append("## Sprints (newest first)")
    out.append("")
    for i, entry in enumerate(entries, start=1):
        out.append(render_entry(i, entry))

    # Cross-cutting themes
    themes = detect_themes(entries)
    out.append("## Cross-cutting themes")
    out.append("")
    if themes:
        out.append(
            "Words appearing in 3+ distinct sprints' titles/objective/status. "
            "Surface-level word-frequency only; not semantic analysis. Use as a "
            "hint about what's recurring — confirm by reading the cited sprints."
        )
        out.append("")
        out.append("| Word | Sprints |")
        out.append("|------|---------|")
        for word, count in themes:
            out.append(f"| `{word}` | {count} |")
        out.append("")
    else:
        out.append("_No words crossed the 3-sprint threshold._")
        out.append("")

    # Aggregated open questions
    out.append("## Open questions (aggregated across sprints)")
    out.append("")
    aggregated: list[tuple[str, str]] = []  # (sprint_title, question)
    for entry in entries:
        if "open_questions" in entry.fields:
            body, _ = entry.fields["open_questions"]
            qs = parse_open_questions(body)
            for q in qs:
                aggregated.append((entry.title, q))

    if aggregated:
        out.append(
            "Every individual item from each sprint's open-questions / followups "
            "section. Gold for the operator — these are what the shifts wanted "
            "the next iteration to address."
        )
        out.append("")
        for sprint_title, q in aggregated:
            out.append(f"- **{sprint_title}**: {q}")
        out.append("")
    else:
        out.append("_No open-questions sections detected in any sprint._")
        out.append("")

    return "\n".join(out)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[1])
    parser.add_argument(
        "--since",
        type=str,
        default=None,
        help="Only include sprints on or after this date (YYYY-MM-DD). Default: all.",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=None,
        help="Override output directory. Default: .claude/memory-kit/<TODAY>/",
    )
    args = parser.parse_args()

    since: date | None = None
    if args.since:
        try:
            since = date.fromisoformat(args.since)
        except ValueError:
            print(f"error: --since must be YYYY-MM-DD, got: {args.since}", file=sys.stderr)
            return 2

    out_dir: Path = args.out_dir or (REPO_ROOT / ".claude" / "memory-kit" / TODAY.isoformat())
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path = out_dir / "sprint-digest.md"

    paths = gather_sprint_files()

    entries: list[SprintEntry] = []
    skipped_unreadable = 0
    for p in paths:
        try:
            text = p.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            skipped_unreadable += 1
            continue
        d, source = extract_date(p, text)
        if since and d < since:
            continue
        entries.append(
            SprintEntry(
                path=p,
                title=extract_title(text, p),
                file_date=d,
                date_source=source,
                fields=extract_fields(text),
                raw_text=text,
            )
        )

    # Newest first
    entries.sort(key=lambda e: e.file_date, reverse=True)

    digest = render_digest(entries, out_path, since)
    out_path.write_text(digest, encoding="utf-8")

    print(f"sprint files found: {len(paths)}")
    if skipped_unreadable:
        print(f"sprint files skipped (unreadable): {skipped_unreadable}")
    print(f"sprint files digested: {len(entries)}")
    if entries:
        print(f"date range: {entries[-1].file_date.isoformat()} to {entries[0].file_date.isoformat()}")
    print(f"digest: {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
