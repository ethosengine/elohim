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

# ── path: keyed segment (materialized locator — tool-managed CACHE; slug stays identity) ──
pc = cg.parse_cite("foo | a desc | sha256:abc | status: held | path: genesis/docs/x.md")
check("path: segment parses", pc["path"] == "genesis/docs/x.md")
check("path: does not eat status", pc["status"] == "held")
pc2 = cg.parse_cite("foo | a desc | sha256:abc | path: a/b.md")
check("path: parses without status", pc2["path"] == "a/b.md" and pc2["status"] == "")
check("legacy cite has empty path key", cg.parse_cite("x/y.md")["path"] == "")
check("envelope without path has empty path key", ec["path"] == "")
ser = cg.serialize_cite(pc)
check("serialize keeps keyed order (status before path)", ser.index("status:") < ser.index("path:"))
rt2 = cg.parse_cite(ser)
check("path survives serialize round-trip",
      rt2["path"] == "genesis/docs/x.md" and rt2["status"] == "held" and rt2["fingerprint"] == "sha256:abc")

# ── materialize_paths: stamp/refresh the locator cache from the slug index ──
cs = [cg.parse_cite("x-slug | d | sha256:abc"),                          # resolvable → stamped
      cg.parse_cite("gone-slug | d | sha256:abc | path: old/loc.md"),    # dead → breadcrumb kept
      cg.parse_cite("some/code/path.py")]                                # legacy → untouched
n = cg.materialize_paths(cs, {"x-slug": str(p)}, d)
check("materialize stamps resolvable slug repo-relative", cs[0]["path"] == "x.md")
check("materialize counts changes", n == 1)
check("materialize keeps dead-slug breadcrumb", cs[1]["path"] == "old/loc.md")
check("materialize leaves legacy alone", cs[2]["path"] == "")
cs2 = [cg.parse_cite("x-slug | d | sha256:abc | path: stale/place.md")]
cg.materialize_paths(cs2, {"x-slug": str(p)}, d)
check("materialize refreshes a stale path", cs2[0]["path"] == "x.md")

# ── gospel walk + index extension (2026-06-05 surfaces) ──
groot = Path(tempfile.mkdtemp())
(groot / "app" / "pillar").mkdir(parents=True)
(groot / "node_modules" / "dep").mkdir(parents=True)
(groot / ".hidden").mkdir()
(groot / "app" / "pillar" / "CLAUDE.md").write_text("---\ntitle: G\nid: pillar-gospel\n---\nbody\n")
(groot / "app" / "CLAUDE.md").write_text("# no id\n")
(groot / "node_modules" / "dep" / "CLAUDE.md").write_text("---\nid: vendored\n---\nx\n")
(groot / ".hidden" / "CLAUDE.md").write_text("---\nid: hidden\n---\nx\n")
gpaths = {pp.relative_to(groot).as_posix() for pp in cg.gospel_claude_md_paths(groot)}
check("gospel walk finds app CLAUDE.mds", {"app/pillar/CLAUDE.md", "app/CLAUDE.md"} <= gpaths)
check("gospel walk prunes vendored + dot dirs", not any("node_modules" in g or ".hidden" in g for g in gpaths))
gidx = cg.extend_index_with_gospels({"pillar-gospel": "existing/wins.md"}, groot)
check("existing slug wins over gospel on collision", gidx["pillar-gospel"] == "existing/wins.md")
gidx2 = cg.extend_index_with_gospels({}, groot)
check("id-declaring gospel enters index", str(gidx2.get("pillar-gospel", "")).endswith("CLAUDE.md"))
check("id-less gospel stays absent from index", len(gidx2) == 1)
check("is_gospel_claude_md accepts in-scope", cg.is_gospel_claude_md(groot / "app" / "pillar" / "CLAUDE.md", groot))
check("is_gospel_claude_md rejects vendored", not cg.is_gospel_claude_md(groot / "node_modules" / "dep" / "CLAUDE.md", groot))
check("is_gospel_claude_md rejects non-CLAUDE.md name", not cg.is_gospel_claude_md(groot / "app" / "x.md", groot))

print(f"\n  {_p} assertions passed ✅")
