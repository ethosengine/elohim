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

print(f"\n  {_passed} assertions passed ✅")
