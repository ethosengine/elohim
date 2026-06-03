"""Tests for cite-gen. Run: python3 .claude/scripts/memory-kit/__tests__/cite_gen_test.py (exit 0 = pass)."""
import importlib.util
import os
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

# fixtures under a temp doc-root
D = Path(tempfile.mkdtemp())
os.environ["CITE_GEN_DOC_ROOTS"] = str(D)
target = D / "target-thing.md"  # stem -> slug 'target-thing' (stem-preferred derive_slug)
target.write_text("---\ntitle: Target Thing — a subtitle\n---\nthe target body\n")
citer = D / "citer.md"
citer.write_text(f"---\ntitle: Citer\ncites:\n  - {target}\nderived_from:\n  - {D}/lineage.md\n---\nciter body\n")

# import cite-gen (hyphenated filename) by path, AFTER the env is set
spec = importlib.util.spec_from_file_location("cite_gen", str(here / ".claude/scripts/memory-kit/cite-gen.py"))
cgmod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(cgmod)

from _lib import frontmatter as fm  # noqa: E402
from _lib import cite_graph as cg   # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── assign-id (migration pass 1) ──
wrote = cgmod.assign_id(target)
check("assign_id writes an id", wrote and fm.parse_file(target).get("id") == "target-thing")
check("assign_id is idempotent", cgmod.assign_id(target) is False)

# ── emit ──
line = cgmod.emit(str(target))
env = cg.parse_cite(line[2:])  # strip '- '
check("emit produces an envelope (slug ref)", env["ref"] == "target-thing" and not env["legacy_path"])
check("emit desc from title first segment", env["desc"] == "Target Thing")
check("emit carries a fingerprint", env["fingerprint"].startswith("sha256:"))

# ── --into (migration pass 2): legacy DOC path -> envelope ──
conv, left = cgmod.rewrite_into(citer)
check("rewrite_into converted the legacy doc cite", conv == 1)
after = cg.parse_cites(fm.parse_file(citer))
check("citer cite is now a slug envelope", after[0]["ref"] == "target-thing" and not after[0]["legacy_path"])
check("rewrite_into is idempotent (no double-convert)", cgmod.rewrite_into(citer)[0] == 0)
check("derived_from preserved (not touched)", "derived_from:" in citer.read_text())

# ── --verify (the dissolution gate) ──
check("verify passes on a fully-migrated doc", cgmod.verify(citer) == [])
legacy = D / "legacy.md"
legacy.write_text(f"---\ntitle: Legacy\ncites:\n  - {target}\n---\nbody\n")
check("verify FAILS on a legacy doc cite", len(cgmod.verify(legacy)) == 1)

# code/external path cites stay legacy + don't fail verify
extern = D / "extern.md"
extern.write_text("---\ntitle: Ext\ncites:\n  - .claude/scripts/memory-kit/decompose.py\n---\nbody\n")
check("code/external path cite stays legacy (verify clean)", cgmod.verify(extern) == [])

# ── weak_desc_count: title-default descs are the progressive-discovery debt ──
check("weak_desc_count flags title-default desc", cgmod.weak_desc_count(citer) == 1)
citer.write_text(citer.read_text().replace("Target Thing |", "the thing this builds on |", 1))
check("weak_desc_count clears once a relationship hint is authored", cgmod.weak_desc_count(citer) == 0)

# ── _legacy_doc_cite_with_id: the sealable-debt detector ──
lc = cg.parse_cites(fm.parse_file(legacy))[0]
check("_legacy_doc_cite_with_id true: legacy path-cite → id-bearing doc", cgmod._legacy_doc_cite_with_id(lc) is True)
ec = cg.parse_cites(fm.parse_file(extern))[0]
check("_legacy_doc_cite_with_id false: code path-cite", cgmod._legacy_doc_cite_with_id(ec) is False)

# ── seal: the born-linked composite (assign-id → into → verify) ──
rc = cgmod.seal(legacy)
check("seal returns 0 (gate passes after conversion)", rc == 0)
check("seal assigned an id", fm.parse_file(legacy).get("id") is not None)
check("seal converted the legacy cite to envelope", not cg.parse_cites(fm.parse_file(legacy))[0]["legacy_path"])

# ── seal_all: the end-of-sprint sweep seals un-sealed docs ──
fresh = D / "fresh.md"
fresh.write_text(f"---\ntitle: Fresh\ncites:\n  - {target}\n---\nbody\n")
cgmod.seal_all()
check("seal_all sealed the fresh doc (id assigned)", fm.parse_file(fresh).get("id") is not None)
check("seal_all converted fresh's cite to envelope", not cg.parse_cites(fm.parse_file(fresh))[0]["legacy_path"])

print(f"\n  {_p} assertions passed ✅")
