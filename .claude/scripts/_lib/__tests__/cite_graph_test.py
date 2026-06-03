"""Tests for _lib.cite_graph. Run: python3 .claude/scripts/_lib/__tests__/cite_graph_test.py (exit 0 = pass)."""
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import cite_graph as cg  # noqa: E402

_p = 0


def check(label, cond):
    global _p
    assert cond, f"FAIL: {label}"
    _p += 1
    print(f"  ✅ {label}")


# ── slug ──
check("slugify", cg.slugify("Semantic-Computable Links — Foo") == "semantic-computable-links-foo")
check("derive_slug prefers stem (unique, keeps -design/-plan suffix)", cg.derive_slug("x/2026-06-02-foo-bar-design.md") == "foo-bar-design")
check("derive_slug falls back to title when stem is generic", cg.derive_slug("x/2026-01-01-design.md", title="Cool Thing — subtitle") == "cool-thing")
check("allocate_slug collision", cg.allocate_slug("foo", {"foo", "foo-2"}) == "foo-3")
check("allocate_slug free", cg.allocate_slug("foo", {"bar"}) == "foo")

# ── fingerprint: body-only, invariant under frontmatter edit ──
d = tempfile.mkdtemp()
p = Path(d) / "x.md"
p.write_text("---\ntitle: A\nid: x-slug\n---\nbody line one\nbody line two\n")
fp1 = cg.fingerprint(p)
p.write_text("---\ntitle: A CHANGED\nid: x-slug\nextra: y\n---\nbody line one\nbody line two\n")
check("fingerprint invariant under frontmatter-only edit", cg.fingerprint(p) == fp1)
p.write_text("---\ntitle: A\nid: x-slug\n---\nbody line one CHANGED\nbody line two\n")
check("fingerprint changes on body edit", cg.fingerprint(p) != fp1)
check("fingerprint format", fp1.startswith("sha256:"))

# ── slug-index ──
p.write_text("---\ntitle: A\nid: x-slug\n---\nbody\n")
idx = cg.build_slug_index([d])
check("slug-index round-trips id:", idx.get("x-slug") == str(p))
check("slug-index skips id-less docs", "missing" not in idx)

# ── parse: legacy + envelope ──
lc = cg.parse_cite("genesis/docs/superpowers/specs/foo.md")
check("legacy path parses as legacy", lc["legacy_path"] and lc["ref"].endswith("foo.md"))
ec = cg.parse_cite("semantic-computable-links | the graph this depends on | sha256:abc123")
check("envelope ref (slug, not legacy)", ec["ref"] == "semantic-computable-links" and not ec["legacy_path"])
check("envelope desc", ec["desc"] == "the graph this depends on")
check("envelope fingerprint", ec["fingerprint"] == "sha256:abc123")
sc = cg.parse_cite("foo | a desc | sha256:abc | status: held — needs remote-compute")
check("envelope status (the epr-head edge hint)", sc["status"] == "held — needs remote-compute")

# ── serialize round-trips ──
back = cg.serialize_cite(ec)
rt = cg.parse_cite(back)
check("serialize→parse round-trips", rt["ref"] == ec["ref"] and rt["fingerprint"] == ec["fingerprint"] and rt["desc"] == ec["desc"])
check("legacy serialize is unchanged path", cg.serialize_cite(lc) == lc["ref"])
check("status survives serialize round-trip", cg.parse_cite(cg.serialize_cite(sc))["status"] == sc["status"])

print(f"\n  {_p} assertions passed ✅")
