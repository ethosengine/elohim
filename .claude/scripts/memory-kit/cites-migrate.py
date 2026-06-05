#!/usr/bin/env python3
"""cites-migrate.py — one-time migration: assign id: slugs + convert legacy DOC cites to envelopes.

Deterministic two-pass over the doc graph (genesis/docs + .claude/memory, PLUS id-or-cites-bearing gospel
CLAUDE.mds since 2026-06-05 — passive CLAUDE.mds without frontmatter stay untouched), idempotent. Phase 7
of semantic-computable-links. Default = dry-run preview; --apply writes. Re-running is a no-op (docs that
already have id: / envelope cites are skipped). The audit resolves slugs, so this never DEAD-storms.
Envelopes are minted with the tool-managed path: locator cache (cite-propagate refreshes it).
"""
from __future__ import annotations

import re
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
_LIST_ITEM = re.compile(r"^\s*-\s+")


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
    # Gospel CLAUDE.mds that ALREADY opted into the graph (id: or cites: frontmatter) join the
    # migration (2026-06-05). Membership is opt-in by declaration — the sweep never force-mints
    # ids onto passive CLAUDE.mds.
    for g in cg.gospel_claude_md_paths(ROOT):
        try:
            f = fm.parse_file(g)
        except OSError:
            continue
        if f.get("id") or f.get("cites"):
            out.append(g)
    return sorted(set(out))


def _split(text):
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    end = next((i for i in range(1, len(lines)) if lines[i].strip() == "---"), None)
    if end is None:
        return None
    return lines[1:end], "\n".join(lines[end + 1:])


def _reassemble(fm_lines, body):
    return "---\n" + "\n".join(fm_lines) + "\n---\n" + body + ("\n" if not body.endswith("\n") else "")


def _insert_id(fm_lines, slug):
    out, done = [], False
    for ln in fm_lines:
        out.append(ln)
        if not done and ln.lstrip().startswith("title:"):
            out.append(f"id: {slug}")
            done = True
    if not done:
        out.insert(0, f"id: {slug}")
    return out


def _set_cites(fm_lines, items):
    out, i, n, ok = [], 0, len(fm_lines), False
    while i < n:
        if fm_lines[i].strip() == "cites:":
            out.append("cites:")
            i += 1
            while i < n and _LIST_ITEM.match(fm_lines[i]):
                i += 1
            out += [f"  - {it}" for it in items]
            ok = True
            continue
        out.append(fm_lines[i])
        i += 1
    return out, ok


def _is_doc(ref):
    p = ROOT / ref.strip("/")
    if not (p.suffix == ".md" and p.is_file()):
        return False
    # Doc-roots OR gospel CLAUDE.mds (2026-06-05) — the same membership cite-gen._is_doc_root uses.
    return any(str(p).startswith(str(r)) for r in DOC_ROOTS) or cg.is_gospel_claude_md(p, ROOT)


def main(apply: bool) -> int:
    docs = corpus()
    # PASS 1 — assign id: (incremental collision tracking so newly-derived slugs don't collide)
    taken, id_by_path = set(), {}
    for d in docs:
        sid = fm.parse_file(d).get("id")
        if sid:
            taken.add(sid)
            id_by_path[str(d)] = sid
    assigned = 0
    for d in docs:
        if str(d) in id_by_path:
            continue
        f = fm.parse_file(d)
        text = d.read_text(encoding="utf-8", errors="replace")
        sp = _split(text)
        if sp is None:
            continue  # no frontmatter — skip
        slug = cg.allocate_slug(cg.derive_slug(str(d), f.get("title")), taken)
        taken.add(slug)
        id_by_path[str(d)] = slug
        if apply:
            d.write_text(_reassemble(_insert_id(sp[0], slug), sp[1]), encoding="utf-8")
        assigned += 1
    # PASS 2 — convert legacy DOC cites -> envelopes (resolve target path -> its id)
    converted = touched = 0
    for d in docs:
        text = d.read_text(encoding="utf-8", errors="replace")
        sp = _split(text)
        if sp is None:
            continue
        cites = cg.parse_cites(fm.parse_file(d))
        if not cites:
            continue
        new, did = [], 0
        for c in cites:
            if not c["legacy_path"]:
                new.append(cg.serialize_cite(c))
                continue
            tgt = (ROOT / c["ref"].strip("/")) if not Path(c["ref"]).is_absolute() else Path(c["ref"])
            if _is_doc(c["ref"]) and str(tgt) in id_by_path:
                tf = fm.parse_file(tgt)
                try:
                    _loc = tgt.resolve().relative_to(ROOT.resolve()).as_posix()
                except ValueError:
                    _loc = tgt.as_posix()
                env = {"ref": id_by_path[str(tgt)], "legacy_path": False,
                       "desc": (tf.get("title") or tgt.stem).split(" — ")[0].strip()[:90],
                       "fingerprint": cg.fingerprint(tgt), "status": "", "path": _loc}
                new.append(cg.serialize_cite(env))
                did += 1
            else:
                new.append(cg.serialize_cite(c))
        if did:
            converted += did
            touched += 1
            if apply:
                out, ok = _set_cites(sp[0], new)
                if ok:
                    d.write_text(_reassemble(out, sp[1]), encoding="utf-8")
    verb = "APPLIED" if apply else "DRY-RUN"
    print(f"cites-migrate [{verb}]: {len(docs)} docs")
    print(f"  pass 1: {assigned} id: slugs assigned ({len(id_by_path) - assigned} already had one)")
    print(f"  pass 2: {converted} cites converted to envelopes across {touched} docs")
    if not apply:
        print("  (re-run with --apply to write)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main("--apply" in sys.argv[1:]))
