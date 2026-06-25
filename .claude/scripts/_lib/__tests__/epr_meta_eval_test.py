"""Validate + pure-guard evaluator test. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_eval_test.py  (exit 0 = pass)"""
import sys
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

def _merged(rules):
    return {"rules": {r["id"]: r for r in rules}, "validators": {}, "sources": ["x"]}

check("validate_meta catches bad class",
      any("class" in e for e in epr_meta.validate_meta(
          {"epr-meta-version": 1, "rules": [{"id": "r", "class": "nope"}]})))
check("validate_meta passes a good manifest",
      epr_meta.validate_meta({"epr-meta-version": 1, "rules": [{"id": "r", "class": "deny"}]}) == [])

m = _merged([{"id": "fm", "class": "deny", "when": {"write": "*.md", "new": True},
              "require-frontmatter": ["id", "status"], "why": "need id+status"}])
v = epr_meta.combine(epr_meta.evaluate(m, {"path": "specs/new.md", "content": "no fm", "is_new": True}))
check("require-frontmatter denies when missing", v.cls == "deny" and "status" in v.reason)
check("require-frontmatter allows when present",
      epr_meta.combine(epr_meta.evaluate(m,
          {"path": "specs/new.md", "content": "---\nid: x\nstatus: Draft\n---\n# b", "is_new": True})) is None)

m = _merged([{"id": "route", "class": "ask", "when": {"write": "*-plan.md"},
              "route-to": {"type": "*-plan.md", "dest": "plans/"}, "why": "to plans/"}])
v = epr_meta.combine(epr_meta.evaluate(m, {"path": "specs/foo-plan.md", "content": "x", "is_new": True}))
check("route-to asks", v.cls == "ask" and "plans/" in v.reason)

check("combine most-severe-wins",
      epr_meta.combine([epr_meta.Verdict("inject", "a", "1"), epr_meta.Verdict("deny", "b", "2"),
                        epr_meta.Verdict("ask", "c", "3")]).cls == "deny")

m = _merged([{"id": "nosub", "class": "deny", "no-new-subdirs": True, "why": "flat"}])
check("no-new-subdirs fires on new subdir",
      epr_meta.combine(epr_meta.evaluate(m,
          {"path": "s/brand/n.md", "content": "x", "is_new": True, "is_new_subdir": True})).cls == "deny")
check("no-new-subdirs silent on existing dir",
      epr_meta.combine(epr_meta.evaluate(m,
          {"path": "s/n.md", "content": "x", "is_new": True, "is_new_subdir": False})) is None)

m = _merged([{"id": "orphan", "class": "ask", "require-sibling": ".epr-meta", "why": "manifest"}])
v = epr_meta.combine(epr_meta.evaluate(m,
        {"path": "s/brand/note.md", "content": "x", "is_new": True, "is_new_subdir": True}))
check("require-sibling asks for orphan tree", v.cls == "ask" and ".epr-meta" in v.reason)
check("require-sibling exempts the .epr-meta itself",
      epr_meta.combine(epr_meta.evaluate(m,
          {"path": "s/brand/.epr-meta", "content": "x", "is_new": True, "is_new_subdir": True})) is None)

print(f"\n  {_passed} assertions passed ✅")
