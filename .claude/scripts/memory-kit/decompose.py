#!/usr/bin/env python3
"""
decompose.py <doc> — DETERMINISTIC decompose of a spec/plan into bounded, cited GAP-ITEMS.

The post-step of /brainstorm (on the new spec) and /plan (on the plan): turns a doc into the
gap list the NEXT phase targets, so implementation plans focus on what's actually unimplemented
instead of re-doing settled work. Gap-items are the bridge from "we designed it" to "what's left."

Extraction (deterministic, in priority order):
  - checkbox tasks: `- [ ]` → OPEN gap ; `- [x]` → CLAIMED gap
        (CHECKED ≠ VERIFIED — checkboxes lie, the iroh gates proved it. A checked box is a CLAIM
         awaiting the verification gate, never trusted as done.)
  - else requirement bullets under Requirements/Acceptance/Tasks/Gates/Components/Deliverable headings → OPEN
  - else no machine-extractable structure → flagged for AGENT decomposition (honest; no fabrication)

Admission gate (anti-runaway, no dump):
  - dedup by text; each item CITES doc#line; skip empty / obviously-meta (commit/lint/TDD chatter).
  - > THRESHOLD items → FLAG (never silently truncate) — a doc this big should be split, not dumped.

Writes .claude/memory-kit/gap-items/<slug>.json (the gap budget the audit + /plan read) + a summary.

Usage:  decompose.py genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
GAP_DIR = ROOT / ".claude/memory-kit/gap-items"
THRESHOLD = 40

CHECKBOX = re.compile(r"^\s*[-*]\s*\[([ xX])\]\s+(.*\S)\s*$")
HEADING = re.compile(r"^#{1,4}\s+(.*\S)\s*$")
BULLET = re.compile(r"^\s*[-*]\s+(.*\S)\s*$")
REQ_HEADING = re.compile(r"\b(requirement|acceptance|task|gate|component|deliverable|criteria|must)\b", re.I)
META = re.compile(r"\b(atomic commit|tdd|self-review|no tbd|no placeholder|cargo fmt|clippy|"
                  r"run the|spec coverage|scenario shapes?|definition of done|rustflags)\b", re.I)


def clean(s: str) -> str:
    return re.sub(r"\s+", " ", re.sub(r"`|\*\*|__", "", s)).strip().rstrip(".")


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: decompose.py <doc-path>", file=sys.stderr)
        return 2
    doc = Path(sys.argv[1])
    if not doc.is_absolute():
        doc = ROOT / doc
    if not doc.is_file():
        print(f"decompose: not a file: {doc}", file=sys.stderr)
        return 2
    try:
        rel = doc.relative_to(ROOT)
    except ValueError:
        print(f"decompose: path is outside repo root {ROOT}: {doc}", file=sys.stderr)
        return 2
    lines = doc.read_text(errors="replace").splitlines()

    items, seen = [], set()

    # 1) checkbox tasks
    for i, line in enumerate(lines, 1):
        m = CHECKBOX.match(line)
        if not m:
            continue
        txt = clean(m.group(2))
        if not txt or META.search(txt):
            continue
        key = txt.lower()[:80]
        if key in seen:
            continue
        seen.add(key)
        items.append({"text": txt, "line": i, "state": "CLAIMED" if m.group(1).lower() == "x" else "OPEN"})

    method = "checkbox-tasks"
    # 2) fallback: requirement bullets under requirement-style headings
    if not items:
        method = "requirement-bullets"
        in_req = False
        for i, line in enumerate(lines, 1):
            h = HEADING.match(line)
            if h:
                in_req = bool(REQ_HEADING.search(h.group(1)))
                continue
            if in_req:
                b = BULLET.match(line)
                if b:
                    txt = clean(b.group(1))
                    if txt and not META.search(txt) and txt.lower()[:80] not in seen:
                        seen.add(txt.lower()[:80])
                        items.append({"text": txt, "line": i, "state": "OPEN"})

    GAP_DIR.mkdir(parents=True, exist_ok=True)
    # Surface-prefix the slug so two docs with the same stem in different dirs
    # (e.g. plans/CLAUDE.md vs specs/CLAUDE.md) don't collide on one output file.
    slug = f"{doc.parent.name}__{doc.stem}"
    out = GAP_DIR / f"{slug}.json"

    if not items:
        out.write_text(json.dumps({"doc": str(rel), "slug": slug, "method": "none",
                                   "note": "no machine-extractable gap structure — needs AGENT decomposition",
                                   "items": []}, indent=1))
        print(f"decompose: {rel}")
        print("  ⚠ no checkboxes or requirement bullets found → this needs AGENT decomposition")
        print("    (the /brainstorm post-step agent should extract the spec's components into gap-items)")
        return 0

    for n, it in enumerate(items, 1):
        it["id"] = f"{slug}#{n}"
        it["cites"] = f"{rel}#L{it['line']}"
    record = {"doc": str(rel), "slug": slug, "method": method, "count": len(items), "items": items}
    out.write_text(json.dumps(record, indent=1))

    opn = sum(1 for it in items if it["state"] == "OPEN")
    clm = len(items) - opn
    print(f"decompose: {rel}   (method: {method})")
    print(f"  → {len(items)} gap-items: {opn} OPEN (unimplemented), {clm} CLAIMED (checked ≠ verified)")
    print(f"  written: {out.relative_to(ROOT)}")
    if len(items) > THRESHOLD:
        print(f"  ⚠ {len(items)} items > {THRESHOLD} — this doc is large; consider SPLITTING it (runaway-doc signal),")
        print("    not dumping all gaps into one plan.")
    print("\n  For /plan: IMPLEMENT the OPEN gaps; VERIFY the CLAIMED ones (ci-investigator) — do NOT trust")
    print("  checked boxes as done. Each gap cites its source line. `placement-audit.py --ledger` rolls these up.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
