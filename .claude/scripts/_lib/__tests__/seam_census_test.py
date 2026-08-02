"""seam_census.py test — the decision-point census (plan P3.2). Run:
python3 .claude/scripts/_lib/__tests__/seam_census_test.py  (exit 0 = pass)

Every fixture registry lives under a fresh tempfile.TemporaryDirectory() — NEVER touches
the real `.claude/epr-meta/*.yaml` or any real `seam-registry.yaml`. Each synthetic repo
root gets its own copy of the REAL published schema (read from the real repo, written into
the fixture tree) so validation runs against the actual contract, not a hand-rolled mirror
of it."""
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

_passed = 0


def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


_REAL_SCHEMA_TEXT = (REPO / sc.SEAM_REGISTRY_SCHEMA_REL).read_text()


def _w(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)


def _with_schema(repo_root: Path) -> Path:
    """Every fixture repo root needs the real schema at its real relative path — validation
    runs against the ACTUAL published contract, never a duplicate."""
    _w(repo_root / sc.SEAM_REGISTRY_SCHEMA_REL, _REAL_SCHEMA_TEXT)
    return repo_root


with tempfile.TemporaryDirectory() as td:
    root = Path(td)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 1 — a clean multi-crate repo: two valid registries, both
    # concern-canon homes populated, one concern deliberately left unbound.
    # ═══════════════════════════════════════════════════════════════
    good_root = _with_schema(root / "repo_good")
    _w(good_root / "crate-a" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-a
crateRoot: crate-a
seamLocus: fixture locus A
decisionPoints:
  - name: decide_thing
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C1
        status: answered
        justification: fixture — exhaustively tested.
    contractTests:
      - path: crate-a/src/lib.rs
        testName: decide_thing_is_total
        kind: unit
  - name: OtherType
    kind: boundary-answer-type
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C4
        status: partial
        justification: fixture — partially covered.
    contractTests: null
    gapNote: fixture — no test yet, honest gap.
""")
    _w(good_root / "crate-a" / "src" / "lib.rs", """\
#[test]
fn decide_thing_is_total() {
    assert!(true);
}
""")
    _w(good_root / "crate-b" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-b
crateRoot: crate-b
seamLocus: fixture locus B
decisionPoints:
  - name: verdict_b
    kind: verdict-fn
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C2
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-b/src/lib.rs
        testName: verdict_b_is_antisymmetric
        kind: unit
""")
    _w(good_root / "crate-b" / "src" / "lib.rs", """\
#[test]
fn verdict_b_is_antisymmetric() {
    assert!(true);
}
""")
    _w(good_root / ".claude" / "epr-meta" / "concerns.yaml", """\
concern-canon-version: 1
concerns:
  - id: c1-test
    version: 1
    concern: C1
  - id: c4-test
    version: 1
    concern: C4
  - id: c9-test
    version: 1
    concern: C9
""")
    _w(good_root / ".claude" / "epr-meta" / "policies.yaml", """\
epr-meta-policies-version: 1
policies:
  - id: c2-test
    version: 1
    class: inject
    concern: C2
    scope: { write: "*.rs" }
    require-frontmatter: [x]
    why: fixture row binding C2's home to policies.yaml.
""")

    d = sc.census_data(good_root)
    check("good repo: both registries discovered and loaded",
          d["n_registries_discovered"] == 2 and d["n_registries_loaded"] == 2)
    check("good repo: zero findings",
          d["findings"] == [])
    regs = {r["crate"]: r for r in d["registries"]}
    check("good repo: crate-a has 2 points, 1 cited, 1 uncited (honest gap)",
          regs["crate-a"]["n_points"] == 2 and regs["crate-a"]["n_cited"] == 1
          and regs["crate-a"]["n_uncited"] == 1)
    check("good repo: crate-b has 1 point, cited",
          regs["crate-b"]["n_points"] == 1 and regs["crate-b"]["n_cited"] == 1)
    cs = d["concern_summary"]
    check("good repo: C1 bound via concerns.yaml, 1 point",
          cs["C1"]["home"] == "concerns.yaml" and cs["C1"]["n_bound_points"] == 1)
    check("good repo: C4 bound via concerns.yaml, status partial",
          cs["C4"]["home"] == "concerns.yaml" and cs["C4"]["status_counts"] == {"partial": 1})
    check("good repo: C2 bound via policies.yaml (the enforcement-row home)",
          cs["C2"]["home"] == "policies.yaml" and cs["C2"]["n_bound_points"] == 1)
    check("good repo: C9 has a home (concerns.yaml) but ZERO bound points -> reported unbound",
          cs["C9"]["home"] == "concerns.yaml" and cs["C9"]["n_bound_points"] == 0
          and "C9" in d["unbound_concerns"])
    check("good repo: a concern with real points is never in unbound_concerns",
          "C1" not in d["unbound_concerns"] and "C2" not in d["unbound_concerns"])
    rendered = sc.render_census(d)
    check("render_census produces the expected section header",
          "DECISION-POINT CENSUS" in rendered and "Concern canon" in rendered)
    check("render_census: JSON mode round-trips the same data",
          __import__("json").loads(sc.render_census(d, as_json=True))["n_registries_loaded"] == 2)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 2 — MISSING-REGISTRATION: one crate's registry is structurally
    # broken (top-level `crate` key entirely absent); the FILE fails to load,
    # but the OTHER (good) registry in the same repo still loads in full —
    # per-row (file-level) degradation, "N-1 still load".
    # ═══════════════════════════════════════════════════════════════
    bad_root = _with_schema(root / "repo_badreg")
    _w(bad_root / "crate-a" / "seam-registry.yaml",
       (good_root / "crate-a" / "seam-registry.yaml").read_text())
    _w(bad_root / "crate-a" / "src" / "lib.rs",
       (good_root / "crate-a" / "src" / "lib.rs").read_text())
    _w(bad_root / "crate-bad" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crateRoot: crate-bad
seamLocus: missing its required `crate` field entirely — must fail TOP-LEVEL validation
decisionPoints:
  - name: something
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C1
        status: answered
        justification: fixture.
    contractTests: null
    gapNote: fixture gap.
""")
    d2 = sc.census_data(bad_root)
    check("missing-registration: 2 discovered, only 1 loaded",
          d2["n_registries_discovered"] == 2 and d2["n_registries_loaded"] == 1)
    reg2 = {r["path"]: r for r in d2["registries"]}
    check("missing-registration: the bad file is flagged unloaded",
          reg2["crate-bad/seam-registry.yaml"]["loaded"] is False)
    check("missing-registration: the good file in the SAME repo still loads fully (N-1 still load)",
          reg2["crate-a/seam-registry.yaml"]["loaded"] is True
          and reg2["crate-a/seam-registry.yaml"]["n_points"] == 2)
    check("missing-registration: a `registry-unparseable` finding fires",
          any(f["kind"] == "registry-unparseable" for f in d2["findings"]))
    check("missing-registration: crate-a's concern bindings still made it into the census",
          d2["concern_summary"]["C1"]["n_bound_points"] == 1)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 3 — PER-ROW degradation within ONE otherwise-valid file: one
    # decisionPoint has an invalid `kind`; the other N-1 rows in the SAME
    # file still load.
    # ═══════════════════════════════════════════════════════════════
    row_root = _with_schema(root / "repo_badrow")
    _w(row_root / "crate-c" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-c
crateRoot: crate-c
seamLocus: fixture locus C — one bad row among two good ones
decisionPoints:
  - name: good_point_1
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C3
        status: answered
        justification: fixture.
    contractTests: null
    gapNote: fixture gap.
  - name: broken_point
    kind: not-a-real-kind
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C11
        status: answered
        justification: fixture.
    contractTests: null
    gapNote: fixture gap.
  - name: good_point_2
    kind: reason-outcome-enum
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C8
        status: answered
        justification: fixture.
    contractTests: null
    gapNote: fixture gap.
""")
    d3 = sc.census_data(row_root)
    reg3 = d3["registries"][0]
    check("per-row degradation: file still loads (not dropped whole)",
          reg3["loaded"] is True)
    check("per-row degradation: exactly 2 of 3 rows survive, 1 flagged",
          reg3["n_points"] == 2 and reg3["n_invalid_rows"] == 1)
    check("per-row degradation: a `decision-point-invalid` finding names the bad row",
          any(f["kind"] == "decision-point-invalid" and "broken_point" in f["detail"]
              for f in d3["findings"]))
    check("per-row degradation: the two GOOD rows' concern bindings still landed",
          d3["concern_summary"]["C3"]["n_bound_points"] == 1
          and d3["concern_summary"]["C8"]["n_bound_points"] == 1)
    check("per-row degradation: the BAD row's concern (C11) was never bound",
          d3["concern_summary"]["C11"]["n_bound_points"] == 0)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 4 — MISSING-CONTRACT-TEST: a citation to a nonexistent test
    # name in a REAL file, and a citation to a nonexistent FILE.
    # ═══════════════════════════════════════════════════════════════
    ct_root = _with_schema(root / "repo_missingtest")
    _w(ct_root / "crate-d" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-d
crateRoot: crate-d
seamLocus: fixture locus D
decisionPoints:
  - name: phantom_test_name
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C7
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-d/src/lib.rs
        testName: this_test_does_not_exist
        kind: unit
  - name: phantom_file
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C7
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-d/src/nowhere.rs
        testName: whatever
        kind: unit
""")
    _w(ct_root / "crate-d" / "src" / "lib.rs", """\
#[test]
fn a_real_test_that_exists() {
    assert!(true);
}
""")
    d4 = sc.census_data(ct_root)
    kinds4 = [f["kind"] for f in d4["findings"]]
    check("missing-contract-test: citation to a nonexistent test NAME fails loud",
          any(f["kind"] == "missing-contract-test" and "phantom_test_name" in f["point"]
              and "this_test_does_not_exist" in f["detail"] for f in d4["findings"]))
    check("missing-contract-test: citation to a nonexistent FILE fails loud",
          any(f["kind"] == "missing-contract-test" and "phantom_file" in f["point"]
              and "does not exist" in f["detail"] for f in d4["findings"]))
    check("missing-contract-test: both points tally as uncited",
          d4["registries"][0]["n_cited"] == 0 and d4["registries"][0]["n_uncited"] == 2)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 5 — ANTI-MIRROR: default heuristic fires, `mirrorRisk: false`
    # suppresses it, `mirrorRisk: true` self-flag surfaces regardless.
    # ═══════════════════════════════════════════════════════════════
    mirror_root = _with_schema(root / "repo_mirror")
    _w(mirror_root / "crate-e" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-e
crateRoot: crate-e
seamLocus: fixture locus E — anti-mirror
decisionPoints:
  - name: mirror_fn
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C11
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-e/src/lib.rs
        testName: test_mirrors_the_call
        kind: unit
  - name: mirror_fn
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C11
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-e/src/lib.rs
        testName: test_cleared_mirror_risk
        kind: unit
        mirrorRisk: false
  - name: mirror_fn
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C11
        status: answered
        justification: fixture.
    contractTests:
      - path: crate-e/src/lib.rs
        testName: test_declared_risk_no_heuristic_match
        kind: unit
        mirrorRisk: true
""")
    _w(mirror_root / "crate-e" / "src" / "lib.rs", """\
fn mirror_fn(x: i32) -> i32 { x + 1 }

#[test]
fn test_mirrors_the_call() {
    let expected_val = mirror_fn(2);
    let actual = mirror_fn(2);
    assert_eq!(actual, expected_val);
}

#[test]
fn test_cleared_mirror_risk() {
    let expected_val = mirror_fn(2);
    let actual = mirror_fn(3);
    assert_eq!(actual, expected_val);
}

#[test]
fn test_declared_risk_no_heuristic_match() {
    assert_eq!(mirror_fn(2), 3);
}
""")
    d5 = sc.census_data(mirror_root)
    mirror_findings = [f for f in d5["findings"] if f["kind"] == "possible-mirror"]
    by_test = {f["detail"].split("`")[1]: f for f in mirror_findings}
    check("anti-mirror: default heuristic fires on the self-referential test",
          "test_mirrors_the_call" in by_test)
    check("anti-mirror: mirrorRisk:false SUPPRESSES the heuristic for that citation",
          "test_cleared_mirror_risk" not in by_test)
    check("anti-mirror: mirrorRisk:true self-flag surfaces regardless of heuristic",
          "test_declared_risk_no_heuristic_match" in by_test)
    check("anti-mirror: exactly 2 possible-mirror findings total (fired + declared, not cleared)",
          len(mirror_findings) == 2)

    # ═══════════════════════════════════════════════════════════════
    # Fixture 6 — CONCERN-MISSING-HOME: a registry cites a concern id with
    # NEITHER canon home present in the repo at all.
    # ═══════════════════════════════════════════════════════════════
    nohome_root = _with_schema(root / "repo_nohome")
    _w(nohome_root / "crate-f" / "seam-registry.yaml", """\
seamRegistryVersion: 1
crate: crate-f
crateRoot: crate-f
seamLocus: fixture locus F — no concern-canon home present anywhere
decisionPoints:
  - name: orphan_point
    kind: pure-decision-predicate
    sourceLocation:
      file: src/lib.rs
    concernIds:
      - id: C1
        status: answered
        justification: fixture.
    contractTests: null
    gapNote: fixture gap.
""")
    d6 = sc.census_data(nohome_root)
    check("concern-missing-home: C1 has no home when neither canon file exists",
          d6["concern_summary"]["C1"]["home"] is None)
    check("concern-missing-home: a `concern-missing-home` finding fires",
          any(f["kind"] == "concern-missing-home" and "C1" in f["detail"] for f in d6["findings"]))

    # ═══════════════════════════════════════════════════════════════
    # Fixture 7 — discovery bounds: `held/` and `genesis/research` trees are
    # never scanned; a sibling registry outside those trees IS found.
    # ═══════════════════════════════════════════════════════════════
    disc_root = root / "repo_discovery"
    _w(disc_root / "held" / "some-crate" / "seam-registry.yaml", "seamRegistryVersion: 1\n")
    _w(disc_root / "genesis" / "research" / "seam-registry.yaml", "seamRegistryVersion: 1\n")
    _w(disc_root / "live-crate" / "seam-registry.yaml", "seamRegistryVersion: 1\n")
    found = sc.discover_registry_paths(disc_root)
    rels = sorted(p.relative_to(disc_root).as_posix() for p in found)
    check("discovery: held/ and genesis/research trees excluded, sibling still found",
          rels == ["live-crate/seam-registry.yaml"])

    # ═══════════════════════════════════════════════════════════════
    # Pure unit checks — helper functions, no fixture I/O.
    # ═══════════════════════════════════════════════════════════════
    check("_symbol_from_name: clean identifier passes through",
          sc._symbol_from_name("decide_head_action") == "decide_head_action")
    check("_symbol_from_name: Type::method with trailing annotation strips the annotation",
          sc._symbol_from_name("K2Store::put_at (MemK2Store)") == "put_at")
    check("_symbol_from_name: natural-language name is deliberately declined (None)",
          sc._symbol_from_name("contest failure classes (metrics::inc_contest_failed)") is None)
    check("_possible_mirror: fires on a self-referential expect binding",
          sc._possible_mirror(
              "fn t() { let expected = f(1); let actual = f(1); assert_eq!(actual, expected); }",
              "f") is True)
    check("_possible_mirror: a single call site never fires (need >=2 calls)",
          sc._possible_mirror("fn t() { assert_eq!(f(1), 2); }", "f") is False)
    check("_possible_mirror: two calls with no expect-named binding never fires",
          sc._possible_mirror("fn t() { let a = f(1); let b = f(2); assert_eq!(a, b); }",
                               "f") is False)

    # ═══════════════════════════════════════════════════════════════
    # load_concerns — mirrors epr_meta.load_policies' own test conventions.
    # ═══════════════════════════════════════════════════════════════
    lc_root = root / "repo_loadconcerns"
    check("load_concerns: missing file is legitimate -> ({}, [])",
          sc.load_concerns(lc_root) == ({}, []))
    _w(lc_root / sc.CONCERNS_REGISTRY_REL, "not: a registry\n")
    c_bad, e_bad = sc.load_concerns(lc_root)
    check("load_concerns: wrong version marker fails LOUD",
          c_bad == {} and len(e_bad) == 1)
    _w(lc_root / sc.CONCERNS_REGISTRY_REL, """\
concern-canon-version: 1
concerns:
  - id: dup
    version: 1
    concern: C1
  - id: dup
    version: 1
    concern: C1
  - id: bad-concern
    version: 1
    concern: NOT-A-CONCERN
  - id: good-one
    version: 1
    concern: C5
""")
    c_ok, e_ok = sc.load_concerns(lc_root)
    check("load_concerns: keeps the good row, flags duplicate + invalid concern id",
          set(c_ok) == {"dup@1", "good-one@1"} and len(e_ok) == 2)

print(f"\n  {_passed} assertions passed ✅")
