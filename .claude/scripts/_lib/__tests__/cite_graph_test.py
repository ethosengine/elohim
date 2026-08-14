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
from _lib import frontmatter as _fm  # noqa: E402

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

# ── full-CID tokens in the fingerprint slot (cite↔CID convergence, Slice-3) ──
# A `bafk…` raw-codec body CID is now valid in the fingerprint slot; verdict decodes it and
# compares the sha2-256 digest against the recomputed body digest. Golden CIDs from `eprfs cid`.
target_cid = Path(d) / "cid-target.md"
target_cid.write_text("---\ntitle: T\nid: cid-doc\n---\ncid body content\n")
OK_CID = "bafkreidmi27tswvwvwm45sbvlytbbe4yo5acu3jpif42eynnk3h4sntyty"
WRONG_CID = "bafkreib7g36t3a3n4i3w5k3g6efwxam25af2fy3e2vjtwibu5cftdii5ui"
sidx = {"cid-doc": str(target_cid)}
check("_is_fingerprint accepts a raw (bafk) CID", cg._is_fingerprint(OK_CID))
check("_is_fingerprint accepts a dag-cbor (bafy) CID", cg._is_fingerprint("bafyreiabc"))
check("_is_fingerprint accepts sha256 short-form", cg._is_fingerprint("sha256:abc"))
check("_cid_sha256_digest decodes 32 raw bytes", len(cg._cid_sha256_digest(OK_CID)) == 32)
check("_cid_sha256_digest rejects a non-CID", cg._cid_sha256_digest("sha256:abc") is None)
ok_cite = {"legacy_path": False, "ref": "cid-doc", "fingerprint": OK_CID}
check("envelope_verdict ok on a matching full-CID token", cg.envelope_verdict(ok_cite, sidx) == "ok")
stale_cite = {"legacy_path": False, "ref": "cid-doc", "fingerprint": WRONG_CID}
check("envelope_verdict stale on a mismatched full-CID token", cg.envelope_verdict(stale_cite, sidx) == "stale")
# the short-form path still works alongside the CID path
short_ok = {"legacy_path": False, "ref": "cid-doc", "fingerprint": cg.fingerprint(target_cid)}
check("envelope_verdict ok on a matching short-form token", cg.envelope_verdict(short_ok, sidx) == "ok")
# the full CID's digest and the short-form truncate the SAME sha2-256 digest
check("full-CID digest[:8].hex() == short-form hex16",
      cg._cid_sha256_digest(OK_CID).hex()[:16] == cg.fingerprint(target_cid).split(":")[1])

# ── remote verdict: absent-locally + POSITIVE remote evidence (full-CID token) ──
# Conservative rule: an absent target is 'remote' ONLY when the fingerprint slot carries a full CID
# (`baf…`, substrate-resolvable). A short-form/absent fingerprint keeps the existing 'dead' verdict —
# existing dead cites are NOT reclassified. A resolvable target is never remote (precedence unchanged).
absent_full_cid = {"legacy_path": False, "ref": "no-such-slug", "fingerprint": OK_CID}
check("absent target + full-CID (baf…) fingerprint → remote", cg.envelope_verdict(absent_full_cid, sidx) == "remote")
absent_short = {"legacy_path": False, "ref": "no-such-slug", "fingerprint": "sha256:abc123def4567890"}
check("absent target + short-form sha256: fingerprint → still dead", cg.envelope_verdict(absent_short, sidx) == "dead")
absent_nofp = {"legacy_path": False, "ref": "no-such-slug", "fingerprint": ""}
check("absent target + no fingerprint → still dead", cg.envelope_verdict(absent_nofp, sidx) == "dead")
absent_nofp_key = {"legacy_path": False, "ref": "no-such-slug"}
check("absent target + missing fingerprint key → still dead", cg.envelope_verdict(absent_nofp_key, sidx) == "dead")
# precedence: a resolvable target with a full-CID token is never remote (ok/held/stale as before)
check("resolvable target + full-CID token is unaffected (ok, not remote)", cg.envelope_verdict(ok_cite, sidx) == "ok")
check("resolvable target + mismatched full-CID token stays stale (not remote)", cg.envelope_verdict(stale_cite, sidx) == "stale")

# ── YAML-valid envelope minting (2026-08-13 defect: unquoted `path: ` segment is invalid plain YAML) ──
env_with_path = "some-slug | a desc | sha256:abcd1234abcd1234 | status: healthy | path: genesis/docs/x.md"
env_plain = "slug2 | d2 | sha256:ef01ef01ef01ef01"
check("yaml_cite_item quotes an envelope carrying ': ' segments",
      cg.yaml_cite_item(env_with_path) == '"' + env_with_path + '"')
check("yaml_cite_item leaves a plain-safe envelope unquoted", cg.yaml_cite_item(env_plain) == env_plain)
q_lines, q_ok = cg.set_cites_block(["title: x", "cites:", "  - old"], [env_with_path, env_plain])
check("set_cites_block quotes at mint", q_ok and any(ln == f'  - "{env_with_path}"' for ln in q_lines))
q_doc = "---\n" + "\n".join(q_lines) + "\n---\nbody\n"
q_fm = _fm.parse(q_doc)
check("frontmatter parse unwraps quoted envelopes", q_fm.fields["cites"][0] == env_with_path)
check("frontmatter parse keeps unquoted envelopes verbatim", q_fm.fields["cites"][1] == env_plain)
q_cite = cg.parse_cite('"' + env_with_path + '"')
check("parse_cite tolerates a raw quoted line",
      q_cite["ref"] == "some-slug" and q_cite["path"] == "genesis/docs/x.md" and q_cite["status"] == "healthy")

print(f"\n  {_p} assertions passed ✅")
