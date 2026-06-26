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

print(f"\n  {_passed} assertions passed ✅")
