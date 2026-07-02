"""Policy registry + define-once-bind-many expansion + measure evaluation + census test. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_policy_test.py  (exit 0 = pass)"""
import json
import subprocess
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

from _lib import epr_meta  # noqa: E402

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


REGISTRY = """\
epr-meta-policies-version: 1
policies:
  - id: loc-ceiling
    version: 1
    class: measure
    scope: { write: "*.rs" }
    measure: { loc-soft: 10, loc-hard: 20 }
    why: too big.
  - id: fm-at-birth
    version: 1
    class: deny
    scope: { write: "*.md", new: true }
    require-frontmatter: [name]
    why: unrecallable.
"""


def _repo(tmp: Path, registry: str = REGISTRY) -> Path:
    (tmp / ".git").mkdir()
    reg = tmp / epr_meta.POLICY_REGISTRY_REL
    reg.parent.mkdir(parents=True)
    reg.write_text(registry)
    return tmp


def _merged(rules):
    return {"rules": {r["id"]: r for r in rules}, "validators": {}, "sources": ["x"]}


with tempfile.TemporaryDirectory() as td:
    tmp = _repo(Path(td))

    # ── load_policies ──
    pols, errs = epr_meta.load_policies(tmp)
    check("load_policies keys by id@version, no errors",
          set(pols) == {"loc-ceiling@1", "fm-at-birth@1"} and errs == [])
    check("load_policies: missing registry is legitimate → ({}, [])",
          epr_meta.load_policies(Path(td) / "nowhere") == ({}, []))
    (tmp / epr_meta.POLICY_REGISTRY_REL).write_text(
        "epr-meta-policies-version: 1\npolicies:\n"
        "  - id: dup\n    version: 1\n    class: ask\n    dedupe-of: x\n"
        "  - id: dup\n    version: 1\n    class: ask\n    dedupe-of: x\n"
        "  - id: x\n    version: nope\n    class: deny\n"
        "  - id: y\n    version: 1\n    class: bogus\n")
    p2, e2 = epr_meta.load_policies(tmp)
    check("load_policies flags duplicate / bad-version / bad-class, keeps the good one",
          set(p2) == {"dup@1"} and len(e2) == 3)
    (tmp / epr_meta.POLICY_REGISTRY_REL).write_text("not: a registry\n")
    p3, e3 = epr_meta.load_policies(tmp)
    check("load_policies: wrong version marker fails LOUD", p3 == {} and len(e3) == 1)
    # M2 footgun applies to the registry: enforcing policy with no actionable predicate is a
    # latent silent-allow — it must NOT load (so bindings drop LOUD via unknown-policy).
    (tmp / epr_meta.POLICY_REGISTRY_REL).write_text(
        "epr-meta-policies-version: 1\npolicies:\n"
        "  - id: nuke\n    version: 1\n    class: deny\n    scope: { write: '*.md' }\n"
        "  - id: multi\n    version: 1\n    class: deny\n    require-frontmatter: [a]\n"
        "    dedupe-of: x\n"
        "  - id: strhard\n    version: 1\n    class: measure\n"
        "    measure: { loc-soft: 10, loc-hard: '20' }\n"
        "  - id: nomeasure\n    version: 1\n    class: measure\n")
    p4, e4 = epr_meta.load_policies(tmp)
    check("load_policies: predicate-less enforcing policy NOT loaded (fires-on-nothing footgun)",
          "nuke@1" not in p4 and any("fires on nothing" in e for e in e4))
    check("load_policies: multi-predicate policy NOT loaded",
          "multi@1" not in p4 and any("multiple actionable predicates" in e for e in e4))
    check("load_policies: non-integer / absent ceilings NOT loaded",
          "strhard@1" not in p4 and "nomeasure@1" not in p4
          and sum("measure" in e for e in e4) >= 2)
    m = _merged([{"id": "b", "policy": "nuke@1"}])
    errs = epr_meta.expand_policies(m, p4)
    check("binding to a rejected policy drops LOUD (unknown-policy advisory)",
          "b" not in m["rules"] and any("unknown policy" in e for e in errs))
    (tmp / epr_meta.POLICY_REGISTRY_REL).write_text(REGISTRY)  # restore
    pols, _ = epr_meta.load_policies(tmp)
    m = _merged([{"id": "b", "policy": "loc-ceiling@1", "params": {"loc-hard": "99"}}])
    errs = epr_meta.expand_policies(m, pols)
    check("params-poisoned ceiling (non-int) → rule kept, LOUD inert-ceiling advisory",
          "b" in m["rules"] and any("are not integers" in e for e in errs))

    # ── expand_policies ──
    m = _merged([{"id": "rs-ceiling", "policy": "loc-ceiling@1"}])
    errs = epr_meta.expand_policies(m, pols)
    r = m["rules"]["rs-ceiling"]
    check("binding expands: class/scope/measure/why from policy, policy-ref stamped",
          errs == [] and r["class"] == "measure" and r["when"] == {"write": "*.rs"}
          and r["measure"]["loc-hard"] == 20 and r["policy-ref"] == "loc-ceiling@1"
          and r["why"] == "too big.")
    m = _merged([{"id": "b", "policy": "loc-ceiling@1", "params": {"loc-hard": 99},
                  "when": {"write": "*.py"}}])
    epr_meta.expand_policies(m, pols)
    r = m["rules"]["b"]
    check("binding params merge over measure defaults; when overrides scope",
          r["measure"] == {"loc-soft": 10, "loc-hard": 99} and r["when"] == {"write": "*.py"})
    m = _merged([{"id": "fm", "policy": "fm-at-birth@1"}])
    epr_meta.expand_policies(m, pols)
    check("binding carries the policy's actionable predicate (require-frontmatter)",
          m["rules"]["fm"]["require-frontmatter"] == ["name"]
          and m["rules"]["fm"]["class"] == "deny")
    m = _merged([{"id": "ghost", "policy": "no-such@1"}])
    errs = epr_meta.expand_policies(m, pols)
    check("unknown policy → rule DROPPED + loud error",
          "ghost" not in m["rules"] and any("unknown policy" in e for e in errs))
    m = _merged([{"id": "unpinned", "policy": "loc-ceiling"}])
    errs = epr_meta.expand_policies(m, pols)
    check("unpinned ref → rule DROPPED + version-pin error",
          "unpinned" not in m["rules"] and any("version" in e for e in errs))
    m = _merged([{"id": "loc-ceiling", "class": "measure", "measure": {"loc-hard": 5}}])
    errs = epr_meta.expand_policies(m, pols)
    check("inline rule shadowing a registry id → dedupe advisory, rule kept",
          "loc-ceiling" in m["rules"] and any("inlined but registry policy" in e for e in errs))
    errs2 = epr_meta.expand_policies(m, pols)  # once more on the same merged dict
    m2 = _merged([{"id": "rs-ceiling", "policy": "loc-ceiling@1"}])
    epr_meta.expand_policies(m2, pols)
    check("idempotent: expanded rules (policy-ref) pass through with no collision advisory",
          epr_meta.expand_policies(m2, pols) == [])

    # ── validate_meta binding shape ──
    check("validate_meta: clean binding passes",
          epr_meta.validate_meta({"epr-meta-version": 1, "rules": [
              {"id": "b", "policy": "loc-ceiling@1", "params": {"loc-hard": 5}}]}) == [])
    check("validate_meta: missing version pin flagged",
          any("declared dependency" in e for e in epr_meta.validate_meta(
              {"epr-meta-version": 1, "rules": [{"id": "b", "policy": "loc-ceiling"}]})))
    check("validate_meta: binding redeclaring class/predicates flagged",
          any("must not redeclare" in e for e in epr_meta.validate_meta(
              {"epr-meta-version": 1, "rules": [
                  {"id": "b", "policy": "loc-ceiling@1", "class": "deny",
                   "require-frontmatter": ["x"]}]})))
    check("validate_meta: non-mapping params flagged",
          any("`params` must be a mapping" in e for e in epr_meta.validate_meta(
              {"epr-meta-version": 1, "rules": [
                  {"id": "b", "policy": "loc-ceiling@1", "params": [1]}]})))

    # ── measure evaluation (pure) ──
    rule = {"id": "m", "class": "measure", "when": {"write": "*.rs"},
            "measure": {"loc-soft": 10, "loc-hard": 20}}
    big = "\n".join(f"l{i}" for i in range(25))
    mid = "\n".join(f"l{i}" for i in range(15))
    sml = "\n".join(f"l{i}" for i in range(5))
    vs = epr_meta.evaluate(_merged([rule]), {"path": "/r/a.rs", "content": big})
    check("at/over loc-hard → measure verdict", vs and vs[0].cls == "measure" and "25 lines" in vs[0].reason)
    vs = epr_meta.evaluate(_merged([rule]), {"path": "/r/a.rs", "content": mid})
    check("over loc-soft only → inject verdict", vs and vs[0].cls == "inject")
    check("under both ceilings → silent",
          epr_meta.evaluate(_merged([rule]), {"path": "/r/a.rs", "content": sml}) == [])
    check("content unresolved → measure abstains",
          epr_meta.evaluate(_merged([rule]), {"path": "/r/a.rs", "content": None}) == [])
    check("measure verdicts never block (combine ignores them)",
          epr_meta.combine(epr_meta.evaluate(_merged([rule]), {"path": "/r/a.rs", "content": big})) is None)
    check("non-matching glob → silent",
          epr_meta.evaluate(_merged([rule]), {"path": "/r/a.py", "content": big}) == [])

    # ── measure_census (descending batch dual) ──
    (tmp / ".epr-meta").write_text(
        "---\nepr-meta-version: 1\nid: r\nroot: true\nrules:\n"
        "  - id: rs-ceiling\n    policy: loc-ceiling@1\n---\n")
    src = tmp / "src"; src.mkdir()
    (src / "huge.rs").write_text("x\n" * 25)
    (src / "mid.rs").write_text("x\n" * 15)
    (src / "ok.rs").write_text("x\n" * 5)
    (src / "huge.py").write_text("x\n" * 25)          # non-matching glob
    sub = tmp / "vendored"; sub.mkdir(); (sub / ".git").mkdir()
    (sub / "vendor.rs").write_text("x\n" * 400)        # submodule boundary → skipped
    exd = tmp / "excluded"; exd.mkdir()
    (exd / "ex.rs").write_text("x\n" * 400)            # exclude_globs → skipped
    c = epr_meta.measure_census(tmp, exclude_globs=("excluded", "excluded/**"))
    check("census: hard row for huge.rs only",
          [r["path"] for r in c["hard"]] == ["src/huge.rs"] and c["hard"][0]["loc"] == 25)
    check("census: soft row for mid.rs only", [r["path"] for r in c["soft"]] == ["src/mid.rs"])
    check("census: submodule + excluded + non-glob files never counted",
          not any("vendor" in r["path"] or "excluded" in r["path"] or r["path"].endswith(".py")
                  for r in c["hard"] + c["soft"]))

    # deeper-manifest override: a region re-binds with looser params → nearest wins
    deep = src / "harness"; deep.mkdir()
    (deep / ".epr-meta").write_text(
        "---\nepr-meta-version: 1\nid: h\ncovers: subtree\nrules:\n"
        "  - id: rs-ceiling\n    policy: loc-ceiling@1\n    params: { loc-hard: 1000, loc-soft: 900 }\n---\n")
    (deep / "table.rs").write_text("x\n" * 200)
    c = epr_meta.measure_census(tmp, exclude_globs=("excluded", "excluded/**"))
    check("census: nearest-wins re-bind raises the ceiling for its region",
          not any(r["path"] == "src/harness/table.rs" for r in c["hard"] + c["soft"]))

    # ── resolver end-to-end in the temp repo: ledger + debounce ──
    resolver = REPO / ".claude/hooks/epr-meta-resolver.py"
    def run_resolver(payload):
        p = subprocess.run([sys.executable, str(resolver)], input=json.dumps(payload),
                           capture_output=True, text=True)
        return json.loads(p.stdout)["hookSpecificOutput"] if p.stdout.strip() else {}

    w = {"tool_name": "Write", "tool_input": {
        "file_path": str(src / "huge.rs"), "content": "x\n" * 30}}
    out = run_resolver(w)
    ctx = out.get("additionalContext", "")
    check("resolver: hard ceiling → NEW architecture finding + dispatch directive",
          "NEW architecture finding" in ctx and "DISPATCH NOW" in ctx)
    ledger = tmp / ".claude/data/architecture-findings.jsonl"
    rows = [json.loads(ln) for ln in ledger.read_text().splitlines()]
    check("resolver: ledger row carries fp/rule/policy/path/status",
          len(rows) == 1 and rows[0]["rule"] == "rs-ceiling"
          and rows[0]["policy"] == "loc-ceiling@1" and rows[0]["path"] == "src/huge.rs"
          and rows[0]["status"] == "open" and len(rows[0]["fp"]) == 12)
    out = run_resolver(w)
    ctx2 = out.get("additionalContext", "")
    check("resolver: re-encounter → no duplicate row, debounced citation not re-NEW",
          "NEW architecture finding" not in ctx2
          and len(ledger.read_text().splitlines()) == 1)
    w_soft = {"tool_name": "Write", "tool_input": {
        "file_path": str(src / "mid.rs"), "content": "x\n" * 15}}
    ctx = run_resolver(w_soft).get("additionalContext", "")
    check("resolver: soft ceiling → inject advisory", "soft LoC ceiling" in ctx)
    ctx2 = run_resolver(w_soft).get("additionalContext", "")
    check("resolver: soft advisory debounced on repeat", "soft LoC ceiling" not in ctx2)

    # ── ledger race: N threads filing the SAME fingerprint concurrently → exactly one row ──
    import importlib.util
    import threading
    spec = importlib.util.spec_from_file_location(
        "epr_meta_resolver", REPO / ".claude/hooks/epr-meta-resolver.py")
    rmod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(rmod)
    race_root = Path(td) / "race"
    (race_root / ".git").mkdir(parents=True)
    verdicts = [epr_meta.Verdict("measure", "`big.rs` is 9999 lines — at/over the ceiling.", "r1")]
    rmerged = {"rules": {"r1": {"id": "r1", "class": "measure", "policy-ref": "p@1",
                                "measure": {"loc-hard": 20, "dispatch-agent": "x"}}},
               "validators": {}, "sources": []}
    barrier = threading.Barrier(8)
    results: list = []

    def worker():
        barrier.wait()
        results.append(rmod._handle_measures(
            verdicts, rmerged, race_root, str(race_root / "big.rs")))

    threads = [threading.Thread(target=worker) for _ in range(8)]
    for t in threads:
        t.start()
    for t in threads:
        t.join()
    rows = [json.loads(ln) for ln in
            (race_root / ".claude/data/architecture-findings.jsonl").read_text().splitlines()]
    check("ledger race: 8 concurrent filers of one fp → exactly 1 row",
          len(rows) == 1 and rows[0]["rule"] == "r1")
    check("ledger race: exactly one NEW directive across all racers",
          sum(1 for notes in results for n in notes if "NEW architecture finding" in n) == 1)

print(f"  {_passed} assertions passed ✅")
