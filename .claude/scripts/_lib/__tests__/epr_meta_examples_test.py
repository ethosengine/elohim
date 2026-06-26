"""Seeded example .epr-meta test (real files). Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_examples_test.py  (exit 0 = pass)"""
import sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
REPO = here

from _lib import epr_meta  # noqa: E402

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

SUP = REPO / "genesis/docs/superpowers"
SPECS = SUP / "specs"

check("base .epr-meta passes schema",
      epr_meta.validate_meta(epr_meta.load_meta(SUP / ".epr-meta")) == [])
check("specs .epr-meta passes schema",
      epr_meta.validate_meta(epr_meta.load_meta(SPECS / ".epr-meta")) == [])

chain = epr_meta.collect_cascade(SPECS / "zzz-new-spec.md")
check("specs/ cascade includes the specs .epr-meta", any(m.parent == SPECS for m in chain))
check("specs/ cascade reaches the root: true base",
      any(epr_meta.load_meta(m).get("root") is True for m in chain))
merged = epr_meta.merge_rules(chain)

v = epr_meta.combine(epr_meta.evaluate(merged,
        {"path": str(SPECS / "zzz-new-spec.md"), "content": "no frontmatter", "is_new": True}))
check("specs/ denies a frontmatter-less new spec", v is not None and v.cls == "deny")

# A frontmatter-COMPLETE *-plan.md: the frontmatter rule is satisfied, so only route-to fires (ask).
complete_fm = ("---\nid: x\nstatus: Draft\nclass: process-meta\ncontext-tier: disclosed\n"
               "steward: cartographer\ngraduation-trigger: t\ncites:\n  - y\n---\n")
v = epr_meta.combine(epr_meta.evaluate(merged,
        {"path": str(SPECS / "zzz-foo-plan.md"), "content": complete_fm, "is_new": True}))
check("specs/ asks to route a frontmatter-complete *-plan.md out",
      v is not None and v.cls == "ask" and "plans/" in v.reason)

print(f"\n  {_passed} assertions passed ✅")
