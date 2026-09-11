#!/usr/bin/env python3
"""locus-drift.py — per-locus drift roll-up over the cite graph.

A LOCUS is an id-bearing gospel (a subject home). This tool aggregates the deterministic drift signals on
each locus into a canon-health view, and shows how a change propagates UP the layered-drift hierarchy
(substrate < domain < implementation). It is the routing signal behind per-locus memory-ceremony / stasis:
"where is canon weakest, and where would a substrate change land."

It does NOT recompute drift — it composes the existing primitives in _lib.cite_graph:
  - envelope_verdict(cite, slug_index)  -> ok | held | stale | dead   (the per-edge drift truth)
  - reverse_index(docs)                 -> { cited-id -> [citer paths] }   (the propagation map)

Modes:
  (default)           per-locus table + ranking + layer summary
  --simulate <id>     one-hop propagation preview: if <id>'s BODY changed, which loci go STALE
  --json              machine output

Spec: genesis/docs/superpowers/specs/2026-06-11-subject-routing-locus-graph-design.md  (§2b layering, §4 scope)
Pure-stdlib. Read-only.
"""
from __future__ import annotations

import json
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
SKIP = {"MEMORY.md", "INDEX.md", "TRAJECTORY.md"}


def _rel(p) -> str:
    try:
        return str(Path(p).resolve().relative_to(ROOT))
    except ValueError:
        return str(p)


def layer_of(path: str) -> str:
    """The depth axis (§2b): substrate < domain < implementation."""
    p = path.replace("\\", "/")
    if "/sdk/domains/" in p:
        return "domain"
    if p.startswith("genesis/docs"):
        return "substrate"
    return "implementation"


_LAYER_ORDER = {"substrate": 0, "domain": 1, "implementation": 2}


def corpus():
    """Every citing/cited doc: doc-roots (with cites/id) + id-bearing gospel CLAUDE.mds."""
    out = []
    for r in DOC_ROOTS:
        if not r.exists():
            continue
        for md in r.rglob("*.md"):
            if md.name in SKIP or "/_state/" in str(md) or "/memory-kit/" in str(md):
                continue
            f = fm.parse_file(md)
            if f.get("id") or f.get("cites"):
                out.append(md)
    for g in cg.gospel_claude_md_paths(ROOT):
        try:
            f = fm.parse_file(g)
        except OSError:
            continue
        if f.get("id") or f.get("cites"):
            out.append(g)
    return sorted(set(out), key=str)


def build():
    docs = corpus()
    held_roots = [str(r) for r in DOC_ROOTS] + [str(ROOT / "genesis/docs/superpowers/held")]
    slug_index = cg.extend_index_with_gospels(cg.build_slug_index(held_roots), ROOT)
    rev = cg.reverse_index(docs)  # cited-id -> [citer paths]

    nodes = {}  # id -> record
    for d in docs:
        f = fm.parse_file(d)
        nid = f.get("id")
        if not nid:
            continue
        cites = cg.parse_cites(f)
        verdicts = [cg.envelope_verdict(c, slug_index) for c in cites]
        unsealed = sum(
            1 for c in cites
            if c.get("legacy_path") and c.get("ref") in slug_index  # migratable legacy path-cite
        )
        rel = _rel(d)
        is_gospel = Path(d).name == "CLAUDE.md"
        nodes[nid] = {
            "id": nid,
            "path": rel,
            "layer": layer_of(rel),
            "is_locus": is_gospel,
            "out": len(cites),
            "in": len(rev.get(nid, [])),
            "stale": verdicts.count("stale"),
            "held": verdicts.count("held"),
            "dead": verdicts.count("dead"),
            "unsealed": unsealed,
            "citers": sorted(_rel(p) for p in rev.get(nid, [])),
        }
    return nodes, slug_index


def health(n: dict) -> int:
    return n["dead"] * 5 + n["stale"] * 3 + n["unsealed"] * 1


def cmd_default(nodes, as_json):
    loci = [n for n in nodes.values() if n["is_locus"]]
    if as_json:
        print(json.dumps({"loci": loci, "anchors": [n for n in nodes.values() if not n["is_locus"]]}, indent=2))
        return
    loci.sort(key=lambda n: (-_LAYER_ORDER.get(n["layer"], 9) * 0, _LAYER_ORDER.get(n["layer"], 9), n["id"]))
    print(f"\nLOCUS DRIFT ROLL-UP — {len(loci)} loci  (drift below each subject home; health=dead·5+stale·3+unsealed·1)\n")
    print(f"  {'locus':34} {'layer':14} {'out':>3} {'in':>3} {'stale':>5} {'dead':>4} {'unseal':>6} {'health':>6}")
    print("  " + "-" * 86)
    for layer in ("substrate", "domain", "implementation"):
        group = [n for n in loci if n["layer"] == layer]
        if not group:
            continue
        for n in sorted(group, key=lambda n: (-health(n), n["id"])):
            print(f"  {n['id']:34} {n['layer']:14} {n['out']:>3} {n['in']:>3} {n['stale']:>5} "
                  f"{n['dead']:>4} {n['unsealed']:>6} {health(n):>6}")
    # layer summary + blast-radius (in-degree) leaders
    print("\n  blast radius (in-degree — what drifts if THIS changes):")
    for n in sorted(nodes.values(), key=lambda n: -n["in"])[:6]:
        if n["in"] == 0:
            break
        print(f"    {n['id']:36} ({n['layer']:14}) ← cited by {n['in']}")
    drifted = [n for n in loci if health(n) > 0]
    print(f"\n  ranking: {len(drifted)} loci carry drift" + (" — clean ✅" if not drifted else ""))
    for n in sorted(drifted, key=lambda n: -health(n))[:8]:
        print(f"    ⚠ {n['id']:34} health={health(n)}  (stale={n['stale']} dead={n['dead']} unsealed={n['unsealed']})  → {n['path']}")
    print()


def cmd_simulate(nodes, slug_index, target):
    if target not in nodes:
        # allow a slug that resolves but has no cites of its own (a pure canon target)
        if target not in slug_index:
            print(f"unknown id: {target}  (not in the cite graph)")
            return 2
    direct = nodes.get(target, {}).get("citers")
    if direct is None:
        # rebuild citers from the reverse index for a pure-target slug
        docs = corpus()
        rev = cg.reverse_index(docs)
        direct = sorted(_rel(p) for p in rev.get(target, []))
    tgt_layer = layer_of(_rel(slug_index.get(target, target)))
    print(f"\nPROPAGATION PREVIEW — if `{target}` ({tgt_layer}) body changes, these go STALE (one hop):\n")
    if not direct:
        print("  (nothing cites it — leaf node)\n")
        return 0
    by_layer = {}
    for c in direct:
        by_layer.setdefault(layer_of(c), []).append(c)
    for layer in ("substrate", "domain", "implementation"):
        for c in sorted(by_layer.get(layer, [])):
            print(f"  → STALE  [{layer:14}] {c}")
    print(f"\n  {len(direct)} locus(es) drift on a `{target}` change. Re-verifying each may itself")
    print("  change its body and cascade to the next layer up (HTML6 → frameworks → sites).\n")
    return 0


def main(argv):
    as_json = "--json" in argv
    nodes, slug_index = build()
    if "--simulate" in argv:
        i = argv.index("--simulate")
        if i + 1 >= len(argv):
            print("usage: locus-drift.py --simulate <id>")
            return 2
        return cmd_simulate(nodes, slug_index, argv[i + 1]) or 0
    cmd_default(nodes, as_json)
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
