#!/usr/bin/env python3
"""
Phase 3 of /converge: apply operator-approved per-theme proposals to canonical plans.

Reads per-theme proposal files at .claude/memory-kit/<date>/converge/<theme>-proposal.md.
For each entry marked `- [x] Accept`, performs the indicated edit on the canonical plan.

v1 supports two edit kinds:
  - mark-done: change `- [ ] <text>` to `- [x] <text>` in the plan; append evidence comment
  - add-as-outstanding: insert a new `- [ ] <text>` after a named anchor section

Other edit kinds (merge-redundant, remove-obsolete, surface-question) are handled
manually by the operator in v1 — flagged as "manual" in the proposal.

Usage:
    converge-apply.py [proposals-date]

If proposals-date is omitted, uses today's directory.
"""

from __future__ import annotations

import argparse
import re
import sys
from datetime import date
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]


# Each accepted block has form:
#   ### N. <kind> — <description>
#   ...
#   **Plan**: `path/to/plan.md`
#   **Edit**: ```diff or yaml-like with old/new
#   ...
#   **Evidence**: <citation>
#   - [x] Accept  <!-- id: <plan-path>:<edit-id> -->
ACCEPT_PATTERN = re.compile(
    r"- \[[xX]\] Accept\s*<!--\s*id:\s*([^\s]+):([^\s]+)\s*-->",
)
PLAN_PATTERN = re.compile(r"\*\*Plan\*\*:\s*`([^`]+)`")
EDIT_PATTERN = re.compile(
    r"\*\*Edit\*\*:.*?```(?:diff|edit)?\n(.+?)\n```",
    re.DOTALL,
)
KIND_PATTERN = re.compile(r"###\s+\d+\.\s+`?(mark-done|add-as-outstanding|merge-redundant|remove-obsolete|surface-question)`?")


def find_section_around_accept(text: str, accept_pos: int) -> str:
    """Return the markdown section ending at the Accept line (back to last ###)."""
    section_start = text.rfind("\n### ", 0, accept_pos)
    if section_start < 0:
        section_start = 0
    section_end = text.find("\n---", accept_pos)
    if section_end < 0:
        section_end = len(text)
    return text[section_start:section_end]


def apply_mark_done(plan_path: Path, target_text: str, evidence: str) -> tuple[bool, str]:
    """Find `- [ ] <target_text>` in plan and change to `- [x] <target_text>` with evidence."""
    if not plan_path.exists():
        return False, f"plan missing: {plan_path}"
    content = plan_path.read_text(encoding="utf-8")
    # Be flexible — match the target text after `- [ ]` or `* [ ]`, possibly with leading whitespace
    pattern = re.compile(
        r"^([\t ]*[-*])\s*\[ \]\s*" + re.escape(target_text.strip()) + r"\s*$",
        re.MULTILINE,
    )
    if not pattern.search(content):
        return False, f"checkbox not found in {plan_path.name}: {target_text[:60]}"
    new_content = pattern.sub(
        r"\1 [x] " + target_text.strip() + (f"  <!-- converge: {evidence} -->" if evidence else ""),
        content,
        count=1,
    )
    plan_path.write_text(new_content, encoding="utf-8")
    return True, f"marked done in {plan_path.name}: {target_text[:60]}"


def apply_add_outstanding(plan_path: Path, anchor: str, new_item: str) -> tuple[bool, str]:
    """Insert `- [ ] <new_item>` immediately after the line matching anchor (typically a heading)."""
    if not plan_path.exists():
        return False, f"plan missing: {plan_path}"
    content = plan_path.read_text(encoding="utf-8")
    lines = content.splitlines()
    anchor_idx = -1
    for i, line in enumerate(lines):
        if anchor.strip() in line:
            anchor_idx = i
            break
    if anchor_idx < 0:
        return False, f"anchor not found in {plan_path.name}: {anchor[:60]}"
    # Insert after anchor (and skip any blank line that follows)
    insert_at = anchor_idx + 1
    while insert_at < len(lines) and not lines[insert_at].strip():
        insert_at += 1
    lines.insert(insert_at, f"- [ ] {new_item}  <!-- converge: added-as-outstanding -->")
    plan_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    return True, f"added to {plan_path.name}: {new_item[:60]}"


def parse_edit_block(section_text: str) -> dict[str, str] | None:
    """Pull out the YAML-like fields from an Edit block. Returns dict with kind/target/evidence/anchor/new_item."""
    em = EDIT_PATTERN.search(section_text)
    if not em:
        return None
    edit_text = em.group(1)
    fields: dict[str, str] = {}
    current_key = None
    buf: list[str] = []
    for line in edit_text.splitlines():
        m = re.match(r"^(\w+):\s*(.*)$", line)
        if m:
            if current_key:
                fields[current_key] = "\n".join(buf).strip()
            current_key = m.group(1).lower()
            buf = [m.group(2)]
        else:
            buf.append(line)
    if current_key:
        fields[current_key] = "\n".join(buf).strip()
    return fields


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("proposals_date", nargs="?", help="Date dir under .claude/memory-kit/ (default: today)")
    args = parser.parse_args()

    if args.proposals_date:
        proposals_date = args.proposals_date
    else:
        proposals_date = date.today().isoformat()
    converge_dir = REPO_ROOT / ".claude" / "memory-kit" / proposals_date / "converge"
    if not converge_dir.exists():
        print(f"error: converge dir not found: {converge_dir}", file=sys.stderr)
        return 1

    proposal_files = sorted(converge_dir.glob("*-proposal.md"))
    if not proposal_files:
        print(f"no proposal files in {converge_dir}")
        return 0

    success = 0
    skip = 0
    fail = 0
    for prop_file in proposal_files:
        text = prop_file.read_text(encoding="utf-8")
        for accept_match in ACCEPT_PATTERN.finditer(text):
            plan_path_str = accept_match.group(1)
            edit_id = accept_match.group(2)
            section = find_section_around_accept(text, accept_match.start())

            kind_m = KIND_PATTERN.search(section)
            if not kind_m:
                print(f"  skip [{prop_file.name}:{edit_id}]: kind not parseable")
                skip += 1
                continue
            kind = kind_m.group(1)

            plan_path = REPO_ROOT / plan_path_str
            fields = parse_edit_block(section) or {}

            if kind == "mark-done":
                target = fields.get("target", "")
                evidence = fields.get("evidence", "")
                if not target:
                    print(f"  skip [{prop_file.name}:{edit_id}]: mark-done missing target")
                    skip += 1
                    continue
                ok, msg = apply_mark_done(plan_path, target, evidence)
            elif kind == "add-as-outstanding":
                anchor = fields.get("anchor", "")
                new_item = fields.get("new_item") or fields.get("text", "")
                if not anchor or not new_item:
                    print(f"  skip [{prop_file.name}:{edit_id}]: add-as-outstanding needs anchor + new_item")
                    skip += 1
                    continue
                ok, msg = apply_add_outstanding(plan_path, anchor, new_item)
            else:
                print(f"  skip [{prop_file.name}:{edit_id}]: kind '{kind}' is manual in v1")
                skip += 1
                continue

            print("  " + ("✓ " if ok else "✗ ") + msg)
            if ok:
                success += 1
            else:
                fail += 1

    print()
    print(f"applied: {success}")
    if skip:
        print(f"skipped: {skip}")
    if fail:
        print(f"failed: {fail}")
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
