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

# _lib bootstrap (walk up to find .claude/scripts/_lib; see _lib/__init__.py)
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import frontmatter as _fm  # noqa: E402
from _lib import subject_routing as _sr  # noqa: E402
from _lib import cite_graph as _cg  # noqa: E402
from _lib import env_scope as _es  # noqa: E402
from _lib import paths as _paths  # noqa: E402

ROOT = _paths.repo_root_from_file(__file__)
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


def _gate_report(hard, signals, klass, declared, cites_to_migrate=0) -> int:
    """Fail-loud on a hard mis-class signal: print it, return non-zero. Gap-items were already written
    with the INFERRED class (correct routing) — loud, but not destructive."""
    if cites_to_migrate:
        print(f"  ℹ dissolution-gate (soft): {cites_to_migrate} legacy doc-cite(s) — run "
              f"`cite-gen --into <this doc>` to make them content-addressed before graduating.")
    for s in signals:
        if s.severity != "hard":
            print(f"  ℹ advisory [{s.id}]: {s.detail}")
    if hard:
        print("  ❌ SUBJECT-CLASS MIS-CLASS (fail-loud — exit 1):")
        for s in hard:
            print(f"     [{s.id}] {s.detail}")
        if declared and declared != klass:
            print(f"     stamped class:{declared} but residue resolves to {klass} — fix the frontmatter.")
        print("     (gap-items were written with the INFERRED class; the routing is correct, the frontmatter is not.)")
        return 1
    return 0


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
    text = doc.read_text(errors="replace")
    lines = text.splitlines()

    # Subject-class: the FRONT gate stamped `class:`; we read it, reconcile from the deliverable-TARGETs,
    # and run the fail-loud gate_signals so a mis-classed doc (e.g. domain:D# + all-.claude targets — the
    # D4 collision) does NOT silently route process residue into substrate legs. See
    # genesis/docs/superpowers/specs/2026-06-02-subject-routed-decomposition-design.md.
    fm = _fm.parse(text)
    targets = _sr.targets_from_frontmatter(fm.fields)
    declared = fm.get("class")
    klass = _sr.classify(targets, declared_class=declared)
    signals = _sr.gate_check(fm.fields, targets)
    hard = [s for s in signals if s.severity == "hard"]

    # Dissolution gate (semantic-computable-links Phase 5, SOFT for now): when this doc graduates, its
    # cites survive into the permanent graph — so they should be content-addressed envelopes. Count legacy
    # doc-cites whose target HAS an id: (migratable but not migrated); advise `cite-gen --into`.
    cites_to_migrate = 0
    for _c in _cg.parse_cites(fm):
        if _c.get("legacy_path"):
            _t = ROOT / _c["ref"].strip("/")
            if _t.suffix == ".md" and _t.is_file() and _fm.parse_file(_t).get("id"):
                cites_to_migrate += 1

    items, seen = [], set()

    # 1) checkbox tasks
    for i, line in enumerate(lines, 1):
        m = CHECKBOX.match(line)
        if not m:
            continue
        txt = clean(m.group(2))
        if not txt or META.search(txt):
            continue
        tags = _es.requires_tags(line)  # per-gap @requires:<cap> override (isomorphic with a2o scenario tags)
        if tags:
            txt = re.sub(r"\s*@requires:[a-z0-9-]+", "", txt).strip()
        key = txt.lower()[:80]
        if key in seen:
            continue
        seen.add(key)
        it = {"text": txt, "line": i, "state": "CLAIMED" if m.group(1).lower() == "x" else "OPEN"}
        if tags:
            it["requires_env"] = tags
        items.append(it)

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
                    tags = _es.requires_tags(line)
                    if tags:
                        txt = re.sub(r"\s*@requires:[a-z0-9-]+", "", txt).strip()
                    if txt and not META.search(txt) and txt.lower()[:80] not in seen:
                        seen.add(txt.lower()[:80])
                        it = {"text": txt, "line": i, "state": "OPEN"}
                        if tags:
                            it["requires_env"] = tags
                        items.append(it)

    GAP_DIR.mkdir(parents=True, exist_ok=True)
    # Surface-prefix the slug so two docs with the same stem in different dirs
    # (e.g. plans/CLAUDE.md vs specs/CLAUDE.md) don't collide on one output file.
    slug = f"{doc.parent.name}__{doc.stem}"
    out = GAP_DIR / f"{slug}.json"

    if not items:
        out.write_text(json.dumps({"doc": str(rel), "slug": slug, "method": "none", "class": klass,
                                   "note": "no machine-extractable gap structure — needs AGENT decomposition",
                                   "items": []}, indent=1))
        print(f"decompose: {rel}   (class: {klass})")
        print("  ⚠ no checkboxes or requirement bullets found → this needs AGENT decomposition")
        print("    (the /brainstorm post-step agent should extract the spec's components into gap-items)")
        return _gate_report(hard, signals, klass, declared, cites_to_migrate)

    for n, it in enumerate(items, 1):
        it["id"] = f"{slug}#{n}"
        it["cites"] = f"{rel}#L{it['line']}"
        it["class"] = klass  # subject-class stamp — routes the BACK-fire per-class (substrate vs process homes)
    # doc-level requires_env = the inheritance default for every gap (the UNIFORMLY-blocked signal). A MIXED
    # plan declares none here and tags only its divergent gaps @requires:<cap> (see _lib/env_scope.py).
    doc_req = _es.parse_requires_env(fm.fields.get("requires_env"))
    record = {"doc": str(rel), "slug": slug, "method": method, "class": klass,
              "doc_requires_env": doc_req, "count": len(items), "items": items}
    out.write_text(json.dumps(record, indent=1))

    opn = sum(1 for it in items if it["state"] == "OPEN")
    clm = len(items) - opn
    print(f"decompose: {rel}   (method: {method}, class: {klass})")
    print(f"  → {len(items)} gap-items: {opn} OPEN (unimplemented), {clm} CLAIMED (checked ≠ verified)")
    print(f"  written: {out.relative_to(ROOT)}")
    if len(items) > THRESHOLD:
        print(f"  ⚠ {len(items)} items > {THRESHOLD} — this doc is large; consider SPLITTING it (runaway-doc signal),")
        print("    not dumping all gaps into one plan.")
    print("\n  For /plan: IMPLEMENT the OPEN gaps; VERIFY the CLAIMED ones (ci-investigator) — do NOT trust")
    print("  checked boxes as done. Each gap cites its source line. `placement-audit.py --ledger` rolls these up.")
    return _gate_report(hard, signals, klass, declared, cites_to_migrate)


if __name__ == "__main__":
    raise SystemExit(main())
