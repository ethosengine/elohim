"""Cascade + merge test. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py  (exit 0 = pass)"""
import sys, tempfile, textwrap
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent

from _lib import epr_meta  # noqa: E402

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

def _wr(p, body):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(textwrap.dedent(body).lstrip())

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: base-rule
            class: inject
          - id: shared
            class: inject
        ---
        root manifest
    """)
    _wr(root / "a" / "b" / ".epr-meta", """
        ---
        epr-meta-version: 1
        rules:
          - id: shared
            class: deny
        ---
        nested manifest
    """)
    chain = epr_meta.collect_cascade(root / "a" / "b" / "new.md")
    check("root-first: root .epr-meta first", chain[0] == root / ".epr-meta")
    check("root-first: nested .epr-meta last", chain[-1] == root / "a" / "b" / ".epr-meta")
    merged = epr_meta.merge_rules(chain)
    check("merge inherits parent rule", merged["rules"]["base-rule"]["class"] == "inject")
    check("merge nearest-wins on id collision", merged["rules"]["shared"]["class"] == "deny")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta" / "manifest.md", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: root-dir-form
            class: inject
        ---
        root manifest
    """)
    _wr(root / "a" / ".epr-meta", """
        ---
        epr-meta-version: 1
        rules:
          - id: nearest-legacy
            class: ask
            route-to: { dest: docs/plans }
        ---
        nested legacy manifest
    """)
    chain = epr_meta.collect_cascade(root / "a" / "new.md")
    check("directory-form root manifest resolves first", chain[0] == root / ".epr-meta" / "manifest.md")
    check("legacy nested manifest still resolves", chain[-1] == root / "a" / ".epr-meta")
    merged = epr_meta.merge_rules(chain)
    check("directory-form root contributes rules", "root-dir-form" in merged["rules"])
    check("legacy nested contributes rules", "nearest-legacy" in merged["rules"])

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta" / "manifest.md", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: directory-wins
            class: inject
        ---
    """)
    check("load_meta accepts .epr-meta directory form",
          epr_meta.load_meta(root / ".epr-meta")["rules"][0]["id"] == "directory-wins")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", "---\nepr-meta-version: 1\nroot: true\n---\n")
    deep = root
    for _ in range(5):
        deep = deep / "d"
    _saved = epr_meta.MAX_CASCADE_DEPTH
    epr_meta.MAX_CASCADE_DEPTH = 2
    try:
        check("cascade depth is bounded", len(epr_meta.collect_cascade(deep / "x.md")) <= 2)
    finally:
        epr_meta.MAX_CASCADE_DEPTH = _saved

# Real-repo check: the elohim capability-package tree (.epr-meta/elohim/packages/) must be
# governed by a resolvable `capability-package-governance` rule (task A2a — govern capabilities
# at source). Uses the actual repo, not a synthetic tempdir, since this asserts against the
# real authored manifest at .epr-meta/elohim/packages/.epr-meta.
_repo_root = epr_meta.find_repo_root(Path(__file__))
_pkg_target = _repo_root / ".epr-meta" / "elohim" / "packages" / "skills" / "dummy.json"
_chain = epr_meta.collect_cascade(_pkg_target)
_merged = epr_meta.merge_rules(_chain)
_policies, _policy_errs = epr_meta.load_policies(_repo_root)
check("no policy registry load errors", _policy_errs == [])
_expand_errs = epr_meta.expand_policies(_merged, _policies)
check("no policy expansion errors", _expand_errs == [])
check("cascade collects capability-package-governance rule under packages/skills/",
      "capability-package-governance" in _merged["rules"])
check("capability-package-governance rule resolves to observation/measure class (never deny/ask)",
      _merged["rules"].get("capability-package-governance", {}).get("class") == "measure")

# ── Task 6 (B2): Python↔Rust `.epr-meta` resolver parity fixtures ──
# These fixtures are REAL, on-disk, and shared verbatim with the Rust resolver's parity test
# (elohim/eprfs/eprfs-meta/tests/parity.rs) — a single manifest corpus feeding both interpreters, so
# they cannot silently drift on cascade order or nearest-wins conflict resolution (the standing hazard
# tracked at genesis/data/timeline/backlog/epr-meta-python-rust-parser-parity.md).
_parity_fixtures = _repo_root / ".claude/scripts/_lib/__tests__/fixtures/epr_meta_parity"


def _ordered_rule_ids(chain):
    """merge_rules' own dict IS the ordered-resolution contract: first-seen wins POSITION
    (root-first insertion), nearest wins VALUE (later cascade entries overwrite in place)."""
    return list(epr_meta.merge_rules(chain)["rules"].keys())


# Fixture 1: directory-form root manifest only, no nested cascade.
_p1_target = _parity_fixtures / "root-directory-form" / "notes" / "new.md"
_p1_chain = epr_meta.collect_cascade(_p1_target)
check("parity fixture 1 (root-directory-form): single manifest resolves",
      len(_p1_chain) == 1 and _p1_chain[0].name == "manifest.md")
check("parity fixture 1: ordered rule-ids == [root-only-rule]",
      _ordered_rule_ids(_p1_chain) == ["root-only-rule"])

# Fixture 2: legacy nested `.epr-meta` flat file (not directory-form), root: true at the nested dir.
_p2_target = _parity_fixtures / "legacy-nested" / "sub" / "leaf.md"
_p2_chain = epr_meta.collect_cascade(_p2_target)
check("parity fixture 2 (legacy-nested): single manifest resolves",
      len(_p2_chain) == 1 and _p2_chain[0].name == ".epr-meta")
check("parity fixture 2: ordered rule-ids == [nested-legacy-rule]",
      _ordered_rule_ids(_p2_chain) == ["nested-legacy-rule"])

# Fixture 3: root + nested id-collision (nearest-wins). Rule ids are deliberately non-alphabetical
# in cascade order (zeta-root-rule, collide-rule, alpha-nested-rule) so a resolver that accidentally
# sorts by id instead of preserving cascade order would be caught here, not pass by coincidence.
_p3_target = _parity_fixtures / "cascade-conflict" / "src" / "leaf.md"
_p3_chain = epr_meta.collect_cascade(_p3_target)
_p3_merged = epr_meta.merge_rules(_p3_chain)
check("parity fixture 3 (cascade-conflict): both manifests resolve, root-first",
      len(_p3_chain) == 2 and _p3_chain[0].name == "manifest.md" and _p3_chain[1].name == ".epr-meta")
check("parity fixture 3: ordered rule-ids == [zeta-root-rule, collide-rule, alpha-nested-rule]",
      list(_p3_merged["rules"].keys())
      == ["zeta-root-rule", "collide-rule", "alpha-nested-rule"])
check("parity fixture 3: nearest-wins — collide-rule resolves to the NESTED class (deny)",
      _p3_merged["rules"]["collide-rule"]["class"] == "deny")

print(f"\n  {_passed} assertions passed ✅")
