"""Validate + pure-guard evaluator test. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_eval_test.py  (exit 0 = pass)"""
import sys, tempfile
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
      epr_meta.validate_meta({"epr-meta-version": 1,
          "rules": [{"id": "r", "class": "deny", "no-new-subdirs": True}]}) == [])
check("validate_meta warns: enforcing rule with no actionable predicate (M2 footgun)",
      any("fires on nothing" in e for e in epr_meta.validate_meta(
          {"epr-meta-version": 1, "rules": [{"id": "nuke", "class": "deny"}]})))
check("validate_meta warns: unknown rule key (typo catch)",
      any("unknown key" in e for e in epr_meta.validate_meta(
          {"epr-meta-version": 1, "rules": [{"id": "x", "class": "deny",
              "no-new-subdirs": True, "reqire-frontmatter": ["id"]}]})))
check("validate_meta warns: >1 actionable predicate (evaluator fires on only the first)",
      any("multiple actionable predicates" in e for e in epr_meta.validate_meta(
          {"epr-meta-version": 1, "rules": [{"id": "m", "class": "deny",
              "no-new-subdirs": True, "require-frontmatter": ["id"]}]})))
check("validate_meta warns: duplicate rule id within one manifest",
      any("duplicate rule id" in e for e in epr_meta.validate_meta(
          {"epr-meta-version": 1, "rules": [
              {"id": "d", "class": "deny", "no-new-subdirs": True},
              {"id": "d", "class": "ask", "route-to": {"dest": "x"}}]})))

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

# M1: glob matching is case-insensitive so an uppercase extension can't bypass a rule
m = _merged([{"id": "md", "class": "deny", "when": {"write": "*.md", "new": True},
              "require-frontmatter": ["id"], "why": "case"}])
check("glob is case-insensitive (*.md matches FILE.MD)",
      epr_meta.combine(epr_meta.evaluate(m,
          {"path": "specs/FILE.MD", "content": "no fm", "is_new": True})).cls == "deny")

# check_meta: the resolver's health-reporter (size cap / deep-nest / parse-fail / schema)
with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    def _mk(name, body):
        p = root / name
        p.write_text(body)
        return p
    healthy = _mk("h", "---\nepr-meta-version: 1\nrules:\n  - id: r\n    class: deny\n    no-new-subdirs: true\n---\n")
    check("check_meta: healthy manifest → []", epr_meta.check_meta(healthy) == [])
    check("check_meta: schema error surfaced",
          any("epr-meta-version" in e for e in epr_meta.check_meta(_mk("b", "---\nepr-meta-version: 2\n---\n"))))
    big = _mk("big", "---\nx: " + "a" * (epr_meta.MAX_MANIFEST_BYTES + 10) + "\n---\n")
    check("check_meta: oversized manifest → size-cap error (parse-DoS guard)",
          any("size cap" in e for e in epr_meta.check_meta(big)))
    deep = _mk("deep", "---\nx: " + "[" * (epr_meta.MAX_FLOW_DEPTH + 5) + "]" * (epr_meta.MAX_FLOW_DEPTH + 5) + "\n---\n")
    check("check_meta: deeply-nested manifest → refused pre-parse (RecursionError guard)",
          any("too deep" in e for e in epr_meta.check_meta(deep)))
    check("load_meta: oversized/deep manifest → {} (cascade fast-path stays safe)",
          epr_meta.load_meta(big) == {} and epr_meta.load_meta(deep) == {})

print(f"\n  {_passed} assertions passed ✅")

# ── Unresolvable validator must not ESCALATE a rule above its declared class ──
# An unregistered validator routes to review so an unanalyzable BLOCKING rule can
# never silently soften (the S5 soundness law, epr_seal.py). But the same branch
# was also hardening rules whose declared class is ADVISORY: every `class: inject`
# rule naming an unregistered validator emitted `ask`, which blocks the agent and
# waits on a human. That is the inversion in the other direction — the rule's own
# text says "it never blocks" while it blocked. Advisory rules must stay advisory
# (warn the agent); only blocking rules route to review.
_UNREG = "epr:validator-does-not-exist"

_inject_rule = {"id": "advisory-with-phantom-validator", "class": "inject",
                "validator": _UNREG, "when": {"write": "*.rs"}, "why": "advisory text"}
v = epr_meta.combine(epr_meta.evaluate(_merged([_inject_rule]),
                                       {"path": "src/x.rs", "content": "fn main() {}"}))
check("inject rule w/ unregistered validator does NOT escalate to ask/deny",
      v is None or v.cls == "inject")

for _blocking in ("ask", "deny"):
    _r = {"id": f"blocking-{_blocking}", "class": _blocking, "validator": _UNREG,
          "when": {"write": "*.rs"}, "why": "gate"}
    _v = epr_meta.combine(epr_meta.evaluate(_merged([_r]),
                                            {"path": "src/x.rs", "content": "fn main() {}"}))
    check(f"{_blocking} rule w/ unregistered validator STILL routes to review (S5 law)",
          _v is not None and _v.cls in ("ask", "deny"))

# ── witness(): `actor` is present only when the caller supplies one (the actor plane's
# absence must read as "not consulted", never as "unclaimed" — that value lives inside the
# dict itself). Mirrors the `evaluator`-field precedent but with the opposite default.
import json as _json
with tempfile.TemporaryDirectory() as _wtd:
    _wroot = Path(_wtd)
    _actor = {"claimed": "matthew", "session": "sess-123",
              "definitionCid": None, "source": "claim"}
    epr_meta.witness(_wroot, runtime="claude", gate="pre-tool-use", subject="f.py",
                      decision="permit", cls="inject", actor=_actor)
    epr_meta.witness(_wroot, runtime="claude", gate="pre-tool-use", subject="g.py",
                      decision="permit", cls="inject")
    _lines = (_wroot / epr_meta.FINDINGS_LEDGER_REL).read_text().splitlines()
    _with_actor = _json.loads(_lines[0])
    _without_actor = _json.loads(_lines[1])
    check("witness(actor=...) writes an entry containing the actor dict",
          _with_actor.get("actor") == _actor)
    check("witness() without actor= omits the actor key entirely",
          "actor" not in _without_actor)

print(f"\n{_passed} checks passed")
