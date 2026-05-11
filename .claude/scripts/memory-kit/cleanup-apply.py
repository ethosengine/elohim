#!/usr/bin/env python3
"""
Apply the operator-approved entries from a cleanup proposals.md.

Reads proposals.md, finds entries with `- [x] Accept`, moves the target
files to the dated archive directory.

The proposals.md itself is left in place as audit record.

Usage:
    cleanup-apply.py [proposals-path]

If proposals-path is omitted, uses today's proposals at
.claude/cleanup/<YYYY-MM-DD>/proposals.md.
"""

from __future__ import annotations

import re
import shutil
import sys
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]

# Matches an accepted entry. The id comment carries the relative path.
ACCEPT_PATTERN = re.compile(
    r"- \[[xX]\] Accept\s*<!--\s*id:\s*([^\s]+)\s*-->",
)


def parse_accepted(proposals_text: str) -> list[Path]:
    """Extract the list of accepted relative paths."""
    out = []
    for m in ACCEPT_PATTERN.finditer(proposals_text):
        out.append(Path(m.group(1)))
    return out


def archive_one(rel_path: Path, archive_root: Path) -> tuple[bool, str]:
    """Move REPO_ROOT/rel_path to archive_root/rel_path. Returns (success, message)."""
    src = REPO_ROOT / rel_path
    if not src.exists():
        return False, f"source missing: {rel_path}"
    dest = archive_root / rel_path
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists():
        return False, f"destination already exists: {dest.relative_to(REPO_ROOT)}"
    shutil.move(str(src), str(dest))
    return True, f"archived: {rel_path} → {dest.relative_to(REPO_ROOT)}"


def main() -> int:
    if len(sys.argv) > 1:
        proposals_path = Path(sys.argv[1])
        if not proposals_path.is_absolute():
            proposals_path = REPO_ROOT / proposals_path
    else:
        today = date.today().isoformat()
        cleanup_dir = REPO_ROOT / ".claude" / "memory-kit" / today
        # Prefer the judged file (Phase 2 output) when present
        judged = cleanup_dir / "cleanup-proposals-judged.md"
        proposals_path = judged if judged.exists() else cleanup_dir / "cleanup-proposals.md"

    if not proposals_path.exists():
        print(f"error: proposals file not found: {proposals_path}", file=sys.stderr)
        return 1
    print(f"reading: {proposals_path}")

    # Archive root = sibling 'archive' directory, dated to match proposals
    proposals_date = proposals_path.parent.name  # YYYY-MM-DD
    archive_root = REPO_ROOT / ".claude" / "archive" / proposals_date

    text = proposals_path.read_text(encoding="utf-8")
    accepted = parse_accepted(text)

    if not accepted:
        print("no entries accepted — nothing to do")
        return 0

    archive_root.mkdir(parents=True, exist_ok=True)

    success_count = 0
    fail_count = 0
    for rel_path in accepted:
        ok, msg = archive_one(rel_path, archive_root)
        print(msg)
        if ok:
            success_count += 1
        else:
            fail_count += 1

    print()
    print(f"archived: {success_count}")
    if fail_count:
        print(f"failed: {fail_count}")
        return 2

    return 0


if __name__ == "__main__":
    sys.exit(main())
