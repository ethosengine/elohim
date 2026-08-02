"""seam_matrix.py test — the concern x seam matrix (plan P4.3). Run:
python3 .claude/scripts/_lib/__tests__/seam_matrix_test.py  (exit 0 = pass)

Two halves, deliberately:
  (a) SYNTHETIC — cell-state derivation, routing, ranking and digest stability over fixture repos
      under tempfile.TemporaryDirectory(). The real registries/catalog/spine are never written.
  (b) LIVE, READ-ONLY — the structural contract of `.claude/epr-meta/seam-catalog.yaml` against
      the actual tree: every `registry_crates` entry must name a crate that really has a
      `seam-registry.yaml`, every `spine_nodes` entry must name a real spine node, and every
      `point_overrides` target must be a real seam id. A catalog that routes to nothing scores
      cells `unexamined` forever while looking healthy — that is precisely the silent-blindness
      class this whole plan exists to end, so it is a test, not a comment.
"""
import json
import sys
import tempfile
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import seam_census as sc  # noqa: E402
from _lib import seam_matrix as sm  # noqa: E402

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


def _w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


_REAL_SCHEMA_TEXT = (REPO / sc.SEAM_REGISTRY_SCHEMA_REL).read_text()

_CATALOG = """\
seam-catalog-version: 1
atlas_source: fixture
plane_groups:
  - id: data
    title: Data plane
  - id: projection
    title: Projection plane
seams:
  - id: SX
    source_section: "9.1"
    name: Fixture data seam
    plane: data
    tracks: [T2]
    reach_tier: familiar
    registry_crates: [fixcrate]
    spine_nodes: [fix-red-active]
    n_a_concerns: []
  - id: SY
    source_section: "9.2"
    name: Fixture projection seam
    plane: projection
    tracks: [T4]
    reach_tier: public
    registry_crates: []
    point_overrides:
      "fixcrate::moved_point": SY
    spine_nodes: [fix-green]
    n_a_concerns: []
"""

_SPINE = """\
version: 1
nodes:
  - id: fix-red-active
    status: red
    active: true
  - id: fix-green
    status: green
    active: false
  - id: fix-unwired
    status: unwired
    active: false
"""

_POLICIES = """\
epr-meta-policies-version: 1
policies:
  - id: c2-fixture
    version: 1
    concern: C2
    title: fixture C2
    binding: binding-local
    class: inject
    scope: { write: "*.rs" }
    dedupe-of: "fixture"
    severity_class: silent-corruption
    recurrence: { unit: distinct-shifts, shifts: 5 }
    status: active
    why: fixture
"""

_CONCERNS = """\
concern-canon-version: 1
concerns:
  - id: c4-fixture
    version: 1
    concern: C4
    title: fixture C4
    statement: fixture
    severity_class: loud-fail
    recurrence: { unit: distinct-shifts, shifts: 2 }
    status: active
    why: fixture
"""


def _registry(points_yaml: str, crate="fixcrate") -> str:
    return (f"seamRegistryVersion: 1\n"
            f"crate: {crate}\n"
            f"crateRoot: fix\n"
            f"seamLocus: fixture locus\n"
            f"decisionPoints:\n" + points_yaml)


def _point(name, concerns_yaml, tests_yaml):
    return (f"  - name: {name}\n"
            f"    kind: pure-decision-predicate\n"
            f"    sourceLocation:\n"
            f"      file: src/lib.rs\n"
            f"    summary: fixture\n"
            f"    concernIds:\n{concerns_yaml}"
            f"{tests_yaml}")


_REAL_TEST = ("    contractTests:\n"
              "      - path: fix/tests/t.rs\n"
              "        testName: a_real_test\n"
              "        kind: unit\n"
              "        mirrorRisk: false\n")
_NO_TEST = ("    contractTests: null\n"
            "    gapNote: no contract test exists yet — honest gap\n")


def _fixture_repo(base: Path, points_yaml: str) -> Path:
    root = base / "repo"
    _w(root / sc.SEAM_REGISTRY_SCHEMA_REL, _REAL_SCHEMA_TEXT)
    _w(root / sm.SEAM_CATALOG_REL, _CATALOG)
    _w(root / sm.SPINE_REL, _SPINE)
    _w(root / ".claude/epr-meta/policies.yaml", _POLICIES)
    _w(root / ".claude/epr-meta/concerns.yaml", _CONCERNS)
    _w(root / "fix/seam-registry.yaml", _registry(points_yaml))
    _w(root / "fix/tests/t.rs", "#[test]\nfn a_real_test() { assert!(true); }\n")
    return root


def _cell(d, concern, seam):
    return next(c for c in d["cells"] if c["concern"] == concern and c["seam"] == seam)


# ═══════════════════════════════════════════════════════════════
# 1. CELL STATE DERIVATION
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = _fixture_repo(Path(td), (
        _point("conformant_point", "      - id: C2\n        status: answered\n", _REAL_TEST)
        + _point("waiver_point",
                 "      - id: C4\n        status: partial\n"
                 "        justification: a documented, negotiated waiver\n", _REAL_TEST)
        + _point("unverified_point", "      - id: C6b\n        status: answered\n", _NO_TEST)
        + _point("na_point",
                 "      - id: C11\n        status: n-a\n"
                 "        justification: structurally inapplicable at this seam\n", _REAL_TEST)))
    d = sm.matrix_data(root)
    check("matrix: 16 classes x 2 fixture seams = 32 cells", d["n_cells"] == 32)
    check("cell state: answered + verified contract test -> conformant",
          _cell(d, "C2", "SX")["state"] == "conformant")
    check("cell state: a documented waiver (status partial) -> variant",
          _cell(d, "C4", "SX")["state"] == "variant"
          and "documented waiver" in _cell(d, "C4", "SX")["reason"])
    check("cell state: answered with NO verified test -> variant, never conformant "
          "(precision bias: a claim with nothing behind it must not stop anyone looking)",
          _cell(d, "C6b", "SX")["state"] == "variant"
          and "NO verified contract test" in _cell(d, "C6b", "SX")["reason"])
    check("cell state: every citing point declaring n-a -> n-a",
          _cell(d, "C11", "SX")["state"] == "n-a")
    check("cell state: no citing point -> unexamined",
          _cell(d, "C9", "SX")["state"] == "unexamined")
    check("cell attributes carry the seam's track + reach tier (attributes, never axes)",
          _cell(d, "C2", "SX")["tracks"] == ["T2"]
          and _cell(d, "C2", "SX")["reach_tier"] == "familiar")
    check("matrix: no errors on a clean fixture", d["errors"] == [])

# ── mixed cell: one answered+verified, one partial -> variant (weakest wins) ──
with tempfile.TemporaryDirectory() as td:
    root = _fixture_repo(Path(td), (
        _point("strong", "      - id: C2\n        status: answered\n", _REAL_TEST)
        + _point("weak", "      - id: C2\n        status: unbound\n"
                          "        justification: nothing binds this yet\n", _REAL_TEST)))
    d = sm.matrix_data(root)
    check("mixed cell: one strong + one weak point reads variant, not conformant "
          "(never over-claim across sibling points)",
          _cell(d, "C2", "SX")["state"] == "variant")

# ── point override routes a point to a different column than its crate ──
with tempfile.TemporaryDirectory() as td:
    root = _fixture_repo(Path(td), (
        _point("moved_point", "      - id: C2\n        status: answered\n", _REAL_TEST)))
    d = sm.matrix_data(root)
    check("point_overrides win over the crate mapping (admission != cluster ops)",
          _cell(d, "C2", "SY")["state"] == "conformant"
          and _cell(d, "C2", "SX")["state"] == "unexamined")

# ── an unrouted crate is an ERROR, never a silent zero ──
with tempfile.TemporaryDirectory() as td:
    root = _fixture_repo(Path(td), (
        _point("p", "      - id: C2\n        status: answered\n", _REAL_TEST)))
    _w(root / "orphan/seam-registry.yaml",
       _registry(_point("q", "      - id: C4\n        status: answered\n", _REAL_TEST),
                 crate="unrouted-crate"))
    d = sm.matrix_data(root)
    check("an unrouted registry crate fails LOUD (an unrouted registry contributes zero cells, "
          "which would read as unexamined everywhere)",
          any("not routed to any seam column" in e for e in d["errors"]))

# ═══════════════════════════════════════════════════════════════
# 2. RANKING + DIGEST
# ═══════════════════════════════════════════════════════════════
with tempfile.TemporaryDirectory() as td:
    root = _fixture_repo(Path(td), (
        _point("p", "      - id: C2\n        status: answered\n", _REAL_TEST)))
    d = sm.matrix_data(root)
    c4sx = _cell(d, "C4", "SX")
    c4sy = _cell(d, "C4", "SY")
    check("rung-proximity: a red+active spine node outranks a green one",
          c4sx["score_parts"]["rung_proximity"] == 4.0
          and c4sy["score_parts"]["rung_proximity"] == 1.0)
    check("severity: silent-corruption weighs 2.0, loud-fail 1.0",
          _cell(d, "C2", "SY")["score_parts"]["severity"] == 2.0
          and c4sy["score_parts"]["severity"] == 1.0)
    check("score = recurrence x severity x rung-proximity",
          c4sx["score"] == 2 * 1.0 * 4.0)
    check("a class with no canon row still ranks, at the recurrence/severity floor",
          _cell(d, "C14", "SX")["score"] == 1 * 1.0 * 4.0)
    check("ranked_unexamined is sorted by descending score",
          all(d["ranked_unexamined"][i]["score"] >= d["ranked_unexamined"][i + 1]["score"]
              for i in range(len(d["ranked_unexamined"]) - 1)))
    check("a seam with no spine linkage falls to the proximity floor, never to zero",
          sm.rung_proximity({"spine_nodes": []}, {})[0] == 0.5)

    dig = d["digest"]
    check("digest is stable across identical runs", sm.matrix_data(root)["digest"] == dig)
    written = sm.write_generated(root, d)
    check("write_generated writes both the JSON read-model and its markdown projection",
          len(written) == 2 and (root / sm.MATRIX_JSON_REL).is_file()
          and (root / sm.MATRIX_MD_REL).is_file())
    check("freshness: a just-written artifact reads fresh",
          sm.generated_freshness(root, d)[0] == "fresh")
    check("the generated markdown carries the DO-NOT-HAND-EDIT banner",
          (root / sm.MATRIX_MD_REL).read_text().startswith("<!-- GENERATED"))
    payload = json.loads((root / sm.MATRIX_JSON_REL).read_text())
    check("the generated JSON records how to regenerate itself",
          "seam-audit.py --matrix --write" in payload["_generated"]["regenerate"])

    # A substrate move (spine flip) must move the digest — otherwise the freshness check is inert.
    _w(root / sm.SPINE_REL, _SPINE.replace("id: fix-green\n    status: green",
                                            "id: fix-green\n    status: red"))
    d2 = sm.matrix_data(root)
    check("a spine flip changes the digest (the freshness check is not inert)",
          d2["digest"] != dig and sm.generated_freshness(root, d2)[0] == "stale")

# ── a missing catalog is an ERROR, not an empty matrix ──
with tempfile.TemporaryDirectory() as td:
    root = Path(td) / "bare"
    root.mkdir(parents=True)
    cat, errs = sm.load_catalog(root)
    check("a missing seam catalog fails LOUD (a matrix with no column axis is a dead instrument)",
          cat == {} and errs and "no column axis" in errs[0])

# ═══════════════════════════════════════════════════════════════
# 3. LIVE, READ-ONLY — the catalog's structural contract
# ═══════════════════════════════════════════════════════════════
live_cat, live_errs = sm.load_catalog(REPO)
check("LIVE: the real seam catalog loads clean", live_cat and live_errs == [])

live_seam_ids = {s["id"] for s in live_cat["seams"]}
real_crates = set()
for p in sc.discover_registry_paths(REPO):
    data, ferr, _rerr = sc.load_registry(p, None, None)
    if data:
        real_crates.add(data.get("crate"))
declared = {c for s in live_cat["seams"] for c in (s.get("registry_crates") or [])}
check("LIVE: every crate the catalog routes really has a seam-registry.yaml",
      declared <= real_crates, )
check("LIVE: every crate with a seam-registry.yaml is routed to some column "
      "(an unrouted registry silently contributes zero cells)",
      real_crates <= declared)

spine_live, _ = sm.load_spine(REPO)
declared_nodes = {n for s in live_cat["seams"] for n in (s.get("spine_nodes") or [])}
check("LIVE: every spine node the catalog links is a real node in spine.yaml",
      declared_nodes <= set(spine_live))
override_targets = {v for s in live_cat["seams"] for v in (s.get("point_overrides") or {}).values()}
check("LIVE: every point_overrides target is a real seam id", override_targets <= live_seam_ids)
check("LIVE: every seam declares a plane that exists in plane_groups",
      {s["plane"] for s in live_cat["seams"]} <= {g["id"] for g in live_cat["plane_groups"]})

live_m = sm.matrix_data(REPO)
check("LIVE: the matrix computes with zero errors", live_m["errors"] == [])
check("LIVE: all 16 canon classes appear as rows, each at exactly one lineage tip",
      len(live_m["concern_tips"]) == 16)
check("LIVE: C6a's tip is the policies.yaml row it graduated INTO, not the superseded "
      "concerns.yaml row",
      live_m["concern_tips"]["C6a"]["ref"] == "c6a-bounded-work@2"
      and live_m["concern_tips"]["C6a"]["home"] == "policies.yaml")
check("LIVE: C2's tip is @2 (phase-1 graduation), read from policies.yaml",
      live_m["concern_tips"]["C2"]["ref"] == "c2-monotonic-authority@2")
check("LIVE: real decision points produce real cells (not everything unexamined)",
      live_m["counts"].get("conformant", 0) > 0 and live_m["counts"].get("variant", 0) > 0)

# ── the spot-check must agree with the census on the live tree ──
spot = sm.spot_check(REPO, 10)
check("LIVE spot-check: the independent disk re-derivation agrees with the census resolver",
      spot["verdict"] == "trust")
check("LIVE spot-check: the sample is reproducible for a given seed",
      [c["point"] for c in sm.spot_check(REPO, 6, seed="abc")["checked"]]
      == [c["point"] for c in sm.spot_check(REPO, 6, seed="abc")["checked"]])
check("LIVE spot-check: a different seed moves the window",
      [c["point"] for c in sm.spot_check(REPO, 4, seed="one")["checked"]]
      != [c["point"] for c in sm.spot_check(REPO, 4, seed="two")["checked"]])

print(f"\n  {_passed} matrix assertions passed ✅")
