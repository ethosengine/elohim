#!/usr/bin/env python3
"""cite-propagate.py — materialize the optional inline `status:` field on cites (the epr-head edge hint).

Phase 4 of semantic-computable-links. The fingerprints + reverse-index make the corpus a COMPUTE SURFACE:
when a target goes held / stale / dead, this fans the verdict OUT to every citing edge, stamping
`status: <hint>` inline; when the target recovers, it clears the field. A healthy cite never carries one.
This is the difference between an audit REPORT and a self-describing EDGE — the link-health travels WITH
the doc, in git, readable in the raw file. Each run is a reviewable commit (the link-health ledger).

Triggered by a scope-tree move, an audit `--apply`, or the stasis loop. Default = dry-run; --apply writes.
Idempotent. Verified = the script exits 0.
"""
from __future__ import annotations

import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import cite_graph as cg  # noqa: E402
from _lib import frontmatter as fm  # noqa: E402

ROOT = next(p for p in Path(__file__).resolve().parents if (p / ".git").exists())
DOC_ROOTS = [ROOT / "genesis" / "docs", ROOT / ".claude" / "memory"]
SKIP = {"CLAUDE.md", "MEMORY.md", "INDEX.md", "README.md", "TRAJECTORY.md", "claude.md"}

_HINT = {
    "held": "held — target sequestered (see cluster-state.yaml)",
    "stale": "stale — target content moved on; re-verify",
    "dead": "dead — target no longer resolves",
}


def corpus():
    out = []
    for r in DOC_ROOTS:
        if not r.exists():
            continue
        for md in r.rglob("*.md"):
            s = str(md)
            if md.name in SKIP or "/_state/" in s or "/memory-kit/" in s:
                continue
            out.append(md)
    return sorted(out)


def main(apply: bool) -> int:
    docs = corpus()
    held_roots = [str(r) for r in DOC_ROOTS] + [str(ROOT / "genesis/docs/superpowers/held")]
    slug_index = cg.extend_index_with_gospels(cg.build_slug_index(held_roots), ROOT)
    stamped = cleared = touched = 0
    by_verdict: dict = {}
    for d in docs:
        text = d.read_text(encoding="utf-8", errors="replace")
        sp = cg.split_frontmatter(text)
        if sp is None:
            continue
        cites = cg.parse_cites(fm.parse_file(d))
        if not cites:
            continue
        changed = False
        for c in cites:
            if c.get("legacy_path"):
                continue
            v = cg.envelope_verdict(c, slug_index)
            want = "" if v == "ok" else _HINT[v]
            if c.get("status", "") != want:
                if want:
                    stamped += 1
                    by_verdict[v] = by_verdict.get(v, 0) + 1
                else:
                    cleared += 1
                c["status"] = want
                changed = True
        if changed:
            touched += 1
            if apply:
                out, ok = cg.set_cites_block(sp[0], cg.serialize_cites(cites))
                if ok:
                    d.write_text(cg.reassemble(out, sp[1]), encoding="utf-8")
    verb = "APPLIED" if apply else "DRY-RUN"
    print(f"cite-propagate [{verb}]: {len(docs)} docs")
    print(f"  status stamped: {stamped} {by_verdict or ''}  |  cleared (recovered/healthy): {cleared}"
          f"  |  docs touched: {touched}")
    if not apply and touched:
        print("  (re-run with --apply to write the self-describing status: hints)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main("--apply" in sys.argv[1:]))
