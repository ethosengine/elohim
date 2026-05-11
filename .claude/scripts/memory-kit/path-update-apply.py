#!/usr/bin/env python3
"""
Apply operator-approved path-update proposals.

Reads `path-update-proposals.md`, finds entries with `- [x] Accept`, and
performs in-place text replacements of OLD path → NEW path in every
affected file listed under that entry.

The proposals.md itself is left in place as audit record.

Usage:
    path-update-apply.py [proposals-path]

If proposals-path is omitted, defaults to today's
.claude/memory-kit/<YYYY-MM-DD>/path-update-proposals.md.
"""

from __future__ import annotations

import re
import sys
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MEMORY_ROOT = Path("/projects/.claude-config/projects/-projects-elohim/memory")

# Match an accepted entry. Each entry's <!-- id: --> encodes
# "<confidence>:<old_path>-><new_path>".
ACCEPT_PATTERN = re.compile(
    r"- \[[xX]\] Accept\s*<!--\s*id:\s*([^:]+):([^\s]+?)->([^\s]+?)\s*-->",
)

# Inside each Accept block, capture affected file paths in the bullet list.
# Lines look like:  "  - `path/to/file.md` (line: 12)" or "(lines: 12, 34)"
AFFECTED_FILE_PATTERN = re.compile(r"^\s*-\s+`([^`]+)`\s+\(line")


def parse_accepted(proposals_text: str) -> list[tuple[str, str, str, list[str]]]:
    """
    Walk the proposals doc and gather (confidence, old, new, affected_files)
    for each accepted entry. Affected files are pulled from the bullet list
    that immediately precedes the Accept checkbox in the same section.
    """
    out: list[tuple[str, str, str, list[str]]] = []

    # Split into per-entry sections by the Accept lines, preserving context.
    lines = proposals_text.splitlines()
    # Find the line index of each Accept line and walk backward to gather files.
    for i, line in enumerate(lines):
        m = ACCEPT_PATTERN.search(line)
        if not m:
            continue
        confidence, old_path, new_path = m.group(1), m.group(2), m.group(3)
        # Walk backward up to the previous "### " heading boundary.
        files: list[str] = []
        for back in range(i - 1, -1, -1):
            prev = lines[back]
            if prev.startswith("### "):
                break
            fm = AFFECTED_FILE_PATTERN.match(prev)
            if fm:
                files.append(fm.group(1))
        files.reverse()  # natural order
        out.append((confidence, old_path, new_path, files))
    return out


def resolve_doc_path(rel_or_abs: str) -> Path | None:
    """
    The scan report writes paths as relative to repo root for repo files,
    or as the full memory-dir basename for memory files. Try both.
    """
    p = Path(rel_or_abs)
    if p.is_absolute():
        return p if p.exists() else None
    # Try repo-relative.
    cand = REPO_ROOT / p
    if cand.exists():
        return cand
    # Try memory-relative.
    cand = MEMORY_ROOT / p
    if cand.exists():
        return cand
    return None


def apply_one(doc_path: Path, old_path: str, new_path: str) -> tuple[bool, int]:
    """
    Replace OLD path → NEW path in doc_path. Returns (changed, occurrence_count).
    Uses simple textual replace; preserves all line endings and surrounding context.
    """
    try:
        text = doc_path.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError) as exc:
        print(f"  ! read failed for {doc_path}: {exc}", file=sys.stderr)
        return False, 0
    occurrences = text.count(old_path)
    if occurrences == 0:
        return False, 0
    new_text = text.replace(old_path, new_path)
    if new_text == text:
        return False, 0
    try:
        doc_path.write_text(new_text, encoding="utf-8")
    except OSError as exc:
        print(f"  ! write failed for {doc_path}: {exc}", file=sys.stderr)
        return False, 0
    return True, occurrences


def main() -> int:
    if len(sys.argv) > 1:
        proposals_path = Path(sys.argv[1])
        if not proposals_path.is_absolute():
            # Allow either "<date>/path-update-proposals.md" or full relative.
            cand = REPO_ROOT / ".claude" / "memory-kit" / proposals_path
            if cand.exists():
                proposals_path = cand
            else:
                proposals_path = REPO_ROOT / proposals_path
    else:
        today = date.today().isoformat()
        proposals_path = (
            REPO_ROOT / ".claude" / "memory-kit" / today / "path-update-proposals.md"
        )

    if not proposals_path.exists():
        print(f"error: proposals file not found: {proposals_path}", file=sys.stderr)
        return 1

    print(f"reading: {proposals_path}")
    text = proposals_path.read_text(encoding="utf-8")
    accepted = parse_accepted(text)

    if not accepted:
        print("no entries accepted — nothing to do")
        return 0

    print(f"accepted entries: {len(accepted)}")
    print()

    total_files_modified = 0
    total_occurrences = 0
    skipped_files: list[tuple[str, str]] = []

    for confidence, old_path, new_path, files in accepted:
        print(f"[{confidence}] {old_path}")
        print(f"  → {new_path}")
        if not files:
            print("  (no affected files listed in proposal — skipping)")
            continue
        for file_str in files:
            doc = resolve_doc_path(file_str)
            if doc is None:
                print(f"  ! could not resolve path: {file_str}")
                skipped_files.append((file_str, "not-found"))
                continue
            changed, count = apply_one(doc, old_path, new_path)
            if changed:
                rel = file_str
                print(f"  ✓ {rel} ({count} occurrence{'s' if count != 1 else ''})")
                total_files_modified += 1
                total_occurrences += count
            else:
                print(f"  · {file_str} (no change — old path not found)")
                skipped_files.append((file_str, "no-occurrence"))
        print()

    print("---")
    print(f"files modified: {total_files_modified}")
    print(f"occurrences replaced: {total_occurrences}")
    if skipped_files:
        print(f"files skipped: {len(skipped_files)}")
        for f, reason in skipped_files:
            print(f"  - {f} ({reason})")

    return 0


if __name__ == "__main__":
    sys.exit(main())
