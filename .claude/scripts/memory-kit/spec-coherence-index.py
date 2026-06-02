#!/usr/bin/env python3
"""
spec-coherence-index.py — DETERMINISTIC prior-art index over specs / plans / canonical.

The dependency that makes the brainstorm "have we spec'd this already?" check possible.
No embeddings — pure token overlap (cheap, explainable, no drift). Topic → ranked prior
specs with their lifecycle STATE, so brainstorming can COMPOSE from canonical instead of
re-speccing.

Build:  spec-coherence-index.py
        → writes .claude/memory-kit/spec-coherence-index.json
Query:  spec-coherence-index.py --query "doorway ssr routing"
        → ranked prior specs (canonical first), each with state + path
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ARGS = [a for a in sys.argv[1:] if not a.startswith("--")]
QUERY = None
if "--query" in sys.argv[1:]:
    i = sys.argv.index("--query")
    QUERY = " ".join(sys.argv[i + 1:])
ROOT = Path(__file__).resolve().parents[3]
INDEX_OUT = ROOT / ".claude/memory-kit/spec-coherence-index.json"

SURFACES = {
    "canonical": "genesis/docs/content/elohim-protocol/architecture",
    "history": "genesis/docs/content/elohim-protocol/history",
    "active-spec": "genesis/docs/superpowers/specs",
    "active-plan": "genesis/docs/superpowers/plans",
    "active-plan-legacy": "genesis/docs/plans",
}
STOP = set("the a an of for to and or in on with from this that into via able able-to "
           "design spec plan implementation phase sprint elohim genesis doc docs notes "
           "substrate system layer based new add update fix support draft proposal "
           "complete landed status date author scope goal v1 v2".split())
LANDED = {"landed", "stable", "done", "complete", "completed", "accepted", "shipped",
          "latest-stable", "claimed-not-verified"}
DEAD = {"superseded", "abandoned", "cancelled", "canceled", "deprecated", "retired"}


def fm_get(text, key):
    m = re.search(rf"^{key}:\s*(.+?)\s*$", text[:1500], re.M | re.I)
    return m.group(1).strip().strip("'\"") if m else ""


def tokens(*parts):
    toks = set()
    for p in parts:
        for t in re.split(r"[^a-z0-9]+", (p or "").lower()):
            if len(t) >= 3 and t not in STOP and not re.fullmatch(r"\d+", t) and not re.fullmatch(r"20\d\d", t):
                toks.add(t)
    return toks


def state_of(home, text):
    raw = (fm_get(text, "status") or "").lower()
    mds = re.search(r"^\*\*Status:\*\*\s*(.+?)\s*$", text, re.M)
    if not raw and mds:
        raw = mds.group(1).lower()
    w = raw.split()[0].strip(":*") if raw else ""
    if home == "canonical":
        return "canonical"
    if home == "history":
        return "history"
    if w in DEAD:
        return "superseded"
    if w in LANDED:
        verified = bool(fm_get(text, "landed_commit") or fm_get(text, "verified_by"))
        return "done" if verified else "claimed-unverified"
    return "active"


def build():
    entries = []
    for home, rel in SURFACES.items():
        d = ROOT / rel
        if not d.is_dir():
            continue
        for f in sorted(d.glob("*.md")):
            if f.name in ("INDEX.md", "CLAUDE.md", "README.md"):
                continue
            text = f.read_text(errors="replace")
            title = fm_get(text, "title")
            if not title:
                h1 = re.search(r"^#\s+(.+)$", text, re.M)
                title = h1.group(1).strip() if h1 else f.stem
            topic_fm = fm_get(text, "topic") + " " + fm_get(text, "tags")
            entries.append({
                "path": str(f.relative_to(ROOT)), "title": title, "home": home,
                "state": state_of(home, text),
                "tokens": sorted(tokens(title, topic_fm, f.stem)),
                "superseded_by": fm_get(text, "superseded-by") or fm_get(text, "superseded_by"),
            })
    return entries


def load_or_build():
    return build()  # always fresh — it's cheap


def query(q, entries, n=8):
    qt = tokens(q)
    scored = []
    for e in entries:
        overlap = qt & set(e["tokens"])
        if overlap:
            scored.append((len(overlap), e, sorted(overlap)))
    # canonical/history outrank active at equal overlap (compose from settled first)
    rank = {"canonical": 0, "history": 1, "done": 2, "claimed-unverified": 3, "active": 4, "superseded": 5}
    scored.sort(key=lambda x: (-x[0], rank.get(x[1]["state"], 9)))
    return scored[:n]


def main():
    if "--query" in sys.argv[1:] and not QUERY:
        print('usage: spec-coherence-index.py --query "<topic>"', file=sys.stderr)
        return 2
    entries = build()
    if not QUERY:
        INDEX_OUT.parent.mkdir(parents=True, exist_ok=True)
        INDEX_OUT.write_text(json.dumps({"generated": "deterministic", "count": len(entries),
                                         "entries": entries}, indent=1))
        print(f"spec-coherence-index: {len(entries)} specs indexed → {INDEX_OUT.relative_to(ROOT)}")
        by_state = {}
        for e in entries:
            by_state[e["state"]] = by_state.get(e["state"], 0) + 1
        for s, c in sorted(by_state.items(), key=lambda x: -x[1]):
            print(f"   {s:<20} {c}")
        return 0

    hits = query(QUERY, entries)
    print(f'PRIOR ART for: "{QUERY}"   ({len(hits)} matches — compose from these, do not re-spec)')
    if not hits:
        print("  (no close prior art found — a standalone spec is justified)")
        return 0
    badge = {"canonical": "✦ CANONICAL", "history": "✝ HISTORY(dead/lesson)", "done": "✓ done",
             "claimed-unverified": "? claimed-UNVERIFIED", "active": "· active", "superseded": "✗ SUPERSEDED"}
    for score, e, ov in hits:
        print(f"  [{badge.get(e['state'], e['state']):<22}] {e['title']}")
        print(f"      {e['path']}   (match: {', '.join(ov[:6])})")
        if e["state"] == "superseded" and e["superseded_by"]:
            print(f"      ↳ do NOT revive — superseded by {e['superseded_by']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
