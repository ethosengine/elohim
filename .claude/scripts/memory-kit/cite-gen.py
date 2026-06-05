#!/usr/bin/env python3
"""cite-gen — generate / migrate / verify content-addressed cites so nobody hand-writes a slug or fingerprint.

  cite-gen.py <target>            emit the envelope line for a target doc (resolve id:, title->desc, fingerprint)
  cite-gen.py --assign-id <doc>   ensure <doc> has a collision-guarded `id:` slug in frontmatter (migration pass 1)
  cite-gen.py --into <doc>        rewrite <doc>'s cites: — legacy DOC path-strings -> envelopes (migration pass 2)
  cite-gen.py --verify <doc>      exit non-zero if any cite is legacy / unresolvable (the BACK-fire dissolution gate)
  cite-gen.py --seal <doc>        born-linked COMPOSITE: --assign-id -> --into -> --verify, + flag title-default descs
  cite-gen.py --seal-all          end-of-sprint sweep: --seal every doc-root .md carrying un-sealed cite debt

Three uses, one primitive: author one cite, migrate the corpus (the cites-migration workflow calls --assign-id
then --into per doc), and gate dissolution (--verify). `--seal` is the single deterministic command the
ceremonies (brainstorm/plan POST-step), the postHook, and end-of-sprint decompose invoke so a NEW doc enters
the graph content-addressed in one shot — no remembering the multi-step. DOC targets (.md under the doc roots)
get slug envelopes; code/external path cites (.claude/scripts/*.py, URLs) stay legacy path-strings.

Verified = the script exits 0 (process-meta verification). Spec: 2026-06-02-semantic-computable-links-design.md.
"""
from __future__ import annotations

import os
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

ROOT = next(p for p in Path(__file__).resolve().parents if (p / ".git").exists() or (p / ".claude").is_dir())
_LIST_ITEM = re.compile(r"^\s*-\s+")


def _doc_roots():
    """The doc graph: where slug-bearing docs live (targets that convert to slug envelopes).
    Overridable via CITE_GEN_DOC_ROOTS (os.pathsep-separated) for tests."""
    override = os.environ.get("CITE_GEN_DOC_ROOTS")
    if override:
        return [Path(p) for p in override.split(os.pathsep)]
    return [ROOT / "genesis" / "docs", ROOT / ".claude" / "memory"]


def _slug_index():
    # Doc-root index + repo-wide id-declaring gospel CLAUDE.mds (2026-06-05 —
    # contextual-gospel concern routing is content-addressed like everything else).
    return cg.extend_index_with_gospels(
        cg.build_slug_index([str(r) for r in _doc_roots()]), ROOT
    )


def _split(text):
    """-> (fm_lines, body) or None if no frontmatter."""
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return None
    end = next((i for i in range(1, len(lines)) if lines[i].strip() == "---"), None)
    if end is None:
        return None
    return lines[1:end], "\n".join(lines[end + 1:])


def _reassemble(fm_lines, body):
    return "---\n" + "\n".join(fm_lines) + "\n---\n" + body + ("\n" if not body.endswith("\n") else "")


def _resolve_doc(ref, index):
    """A cite ref (slug or repo-relative path) -> absolute Path of the target DOC, or None if not a doc."""
    if ref in index:
        return Path(index[ref])
    p = (ROOT / ref) if not Path(ref).is_absolute() else Path(ref)
    if p.suffix == ".md" and p.is_file():
        return p
    return None


def _is_doc_root(p: Path) -> bool:
    # Gospel CLAUDE.mds join the doc graph (2026-06-05) — see cite_graph.extend_index_with_gospels.
    return any(str(p).startswith(str(r)) for r in _doc_roots()) or cg.is_gospel_claude_md(p, ROOT)


def emit(target: str) -> str | None:
    index = _slug_index()
    doc = _resolve_doc(target, index)
    if not doc:
        print(f"cite-gen: not a resolvable doc target: {target}", file=sys.stderr)
        return None
    f = fm.parse_file(doc)
    sid = f.get("id") or cg.derive_slug(str(doc), f.get("title"))
    title = f.get("title") or doc.stem
    env = {"ref": sid, "legacy_path": False, "desc": title.split(" — ")[0].strip(),
           "fingerprint": cg.fingerprint(doc), "status": ""}
    return "- " + cg.serialize_cite(env)


def assign_id(doc: Path) -> bool:
    """Ensure the doc has a collision-guarded id: slug. Returns True if it wrote one."""
    text = doc.read_text(encoding="utf-8", errors="replace")
    split = _split(text)
    if split is None:
        return False
    fm_lines, body = split
    f = fm.parse(text)
    if f.get("id"):
        return False  # already has one (idempotent)
    taken = set(_slug_index().keys())
    slug = cg.allocate_slug(cg.derive_slug(str(doc), f.get("title")), taken)
    # insert id: after title: (or at top of frontmatter)
    out, inserted = [], False
    for ln in fm_lines:
        out.append(ln)
        if not inserted and ln.lstrip().startswith("title:"):
            out.append(f"id: {slug}")
            inserted = True
    if not inserted:
        out.insert(0, f"id: {slug}")
    doc.write_text(_reassemble(out, body), encoding="utf-8")
    return True


def _set_cites(fm_lines, new_items):
    """Replace the cites: list block with new_items (list of cite strings, no leading '- ')."""
    out, i, replaced = [], 0, False
    n = len(fm_lines)
    while i < n:
        ln = fm_lines[i]
        if ln.rstrip() == "cites:" or ln.strip() == "cites:":
            out.append("cites:")
            i += 1
            while i < n and _LIST_ITEM.match(fm_lines[i]):
                i += 1  # drop old items
            for it in new_items:
                out.append(f"  - {it}")
            replaced = True
            continue
        out.append(ln)
        i += 1
    return out, replaced


def rewrite_into(doc: Path) -> tuple[int, int]:
    """Rewrite legacy DOC path-cites -> envelopes. Returns (converted, left_legacy)."""
    text = doc.read_text(encoding="utf-8", errors="replace")
    split = _split(text)
    if split is None:
        return (0, 0)
    fm_lines, body = split
    f = fm.parse(text)
    cites = cg.parse_cites(f)
    if not cites:
        return (0, 0)
    index = _slug_index()
    converted = left = 0
    new = []
    for c in cites:
        if not c["legacy_path"]:
            new.append(cg.serialize_cite(c))
            continue
        tgt = _resolve_doc(c["ref"], index)
        if tgt and _is_doc_root(tgt):
            tf = fm.parse_file(tgt)
            sid = tf.get("id")
            if sid:
                env = {"ref": sid, "legacy_path": False,
                       "desc": (tf.get("title") or tgt.stem).split(" — ")[0].strip(),
                       "fingerprint": cg.fingerprint(tgt), "status": ""}
                new.append(cg.serialize_cite(env))
                converted += 1
                continue
        new.append(cg.serialize_cite(c))  # leave legacy (code/external, or target lacks id:)
        left += 1
    out, ok = _set_cites(fm_lines, new)
    if ok and converted:
        doc.write_text(_reassemble(out, body), encoding="utf-8")
    return (converted, left)


def verify(doc: Path) -> list:
    """Return a list of problem strings: legacy cites + unresolvable refs. Empty = passes the gate."""
    f = fm.parse_file(doc)
    index = _slug_index()
    problems = []
    for c in cg.parse_cites(f):
        if c["legacy_path"]:
            tgt = _resolve_doc(c["ref"], index)
            # a problem ONLY if the target is a doc that HAS an id: (so it could be an envelope but isn't).
            # A legacy cite to a frontmatter-less doc (no id: possible) or to code/external is fine.
            if tgt and _is_doc_root(tgt) and fm.parse_file(tgt).get("id"):
                problems.append(f"legacy doc cite (migrate to envelope): {c['ref']}")
        else:
            if c["ref"] not in index:
                problems.append(f"unresolvable slug: {c['ref']}")
    return problems


def weak_desc_count(doc: Path) -> int:
    """Envelope cites still carrying the title-default desc (the weak migration default) — these need a
    relationship hint authored via cite-describe. The 'progressive-discovery debt' a seal reports."""
    f = fm.parse_file(doc)
    index = _slug_index()
    weak = 0
    for c in cg.parse_cites(f):
        if c["legacy_path"]:
            continue
        tgt = _resolve_doc(c["ref"], index)
        if tgt:
            title = (fm.parse_file(tgt).get("title") or tgt.stem).split(" — ")[0].strip()
            d = (c.get("desc") or "").strip()
            if not d or d.lower() == title.lower() or d == c["ref"]:
                weak += 1
    return weak


def seal(doc: Path) -> int:
    """The born-linked composite: assign-id -> --into -> --verify, then flag weak descriptions. The single
    deterministic command ceremonies/hooks call so a new doc joins the graph content-addressed in one shot.
    Exit 0 iff the dissolution gate passes (no DEAD / legacy-migratable cites). Weak descriptions are
    advisory — authoring a relationship hint is agentic judgement (dispatch cite-describe)."""
    rel = doc.relative_to(ROOT) if (doc.is_absolute() and str(doc).startswith(str(ROOT))) else doc
    wrote_id = assign_id(doc)
    conv, left = rewrite_into(doc)
    probs = verify(doc)
    weak = weak_desc_count(doc)
    print(f"cite-gen --seal {rel}:")
    print(f"   id: {'assigned' if wrote_id else 'present'}  ·  cites: {conv} sealed to envelope, {left} legacy-kept")
    if probs:
        print(f"   ❌ gate: {len(probs)} unresolved — " + "; ".join(probs[:3]))
    else:
        print("   ✅ gate: all cites content-addressed + resolvable")
    if weak:
        print(f"   ✍ {weak} cite(s) on the title-default desc — author relationship hints "
              f"(cite-describe.py) for progressive discovery")
    return 1 if probs else 0


def _legacy_doc_cite_with_id(c) -> bool:
    """A legacy path-cite pointing at a doc-root .md that HAS an id: (a CITE-FORMAT-CANDIDATE — sealable)."""
    if not c.get("legacy_path"):
        return False
    ref = c["ref"]
    if not ref.endswith(".md"):
        return False
    tgt = (ROOT / ref) if not Path(ref).is_absolute() else Path(ref)
    try:
        return bool(tgt.is_file() and _is_doc_root(tgt) and fm.parse_file(tgt).get("id"))
    except OSError:
        return False


def seal_all(roots=None) -> int:
    """End-of-sprint sweep: seal EVERY doc-root .md that carries un-sealed cite debt, so all work authored
    during the sprint joins the permanent graph content-addressed before it graduates/dissolves. Cheap
    needs-check first (only debt-bearing docs pay the seal cost); already-born-linked docs are skipped."""
    roots = roots or _doc_roots()
    sealed = converted = weak_total = gate_fail = 0
    for r in roots:
        if not Path(r).is_dir():
            continue
        for doc in sorted(Path(r).rglob("*.md")):
            if doc.name in ("CLAUDE.md", "README.md", "MEMORY.md"):
                continue
            f = fm.parse_file(doc)
            cites = cg.parse_cites(f)
            if not cites:
                continue
            needs = (not f.get("id")) or any(_legacy_doc_cite_with_id(c) for c in cites)
            if not needs:
                continue
            assign_id(doc)
            conv, _ = rewrite_into(doc)
            converted += conv
            if verify(doc):
                gate_fail += 1
            weak_total += weak_desc_count(doc)
            sealed += 1
    print(f"cite-gen --seal-all: {sealed} doc(s) sealed, {converted} cite(s) → envelope")
    if gate_fail:
        print(f"   ❌ {gate_fail} doc(s) still have unresolved cites (DEAD-CITE — run --verify per doc)")
    if weak_total:
        print(f"   ✍ {weak_total} cite(s) on the title-default desc across the sweep — author relationship "
              f"hints (cite-describe.py / dispatch the corpus-describe workflow)")
    if not sealed:
        print("   ✅ corpus already born-linked (no un-sealed debt)")
    return 0


def main() -> int:
    args = sys.argv[1:]
    if not args:
        print(__doc__)
        return 2
    if args[0] == "--seal-all":
        return seal_all()
    if args[0] == "--seal" and len(args) > 1:
        doc = (ROOT / args[1]) if not Path(args[1]).is_absolute() else Path(args[1])
        if not doc.is_file():
            print(f"cite-gen --seal: no such doc: {args[1]}", file=sys.stderr)
            return 2
        return seal(doc)
    if args[0] == "--assign-id" and len(args) > 1:
        doc = (ROOT / args[1]) if not Path(args[1]).is_absolute() else Path(args[1])
        wrote = assign_id(doc)
        print(f"cite-gen --assign-id {args[1]}: {'assigned' if wrote else 'already had id: (no-op)'}")
        return 0
    if args[0] == "--into" and len(args) > 1:
        doc = (ROOT / args[1]) if not Path(args[1]).is_absolute() else Path(args[1])
        conv, left = rewrite_into(doc)
        print(f"cite-gen --into {args[1]}: {conv} converted to envelope, {left} left legacy (code/external/no-id)")
        return 0
    if args[0] == "--verify" and len(args) > 1:
        doc = (ROOT / args[1]) if not Path(args[1]).is_absolute() else Path(args[1])
        probs = verify(doc)
        if probs:
            print(f"cite-gen --verify {args[1]}: ❌ {len(probs)} problem(s):")
            for p in probs:
                print(f"   - {p}")
            return 1
        print(f"cite-gen --verify {args[1]}: ✅ all cites content-addressed + resolvable")
        return 0
    out = emit(args[0])
    if out is None:
        return 1
    print(out)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
