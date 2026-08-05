---
title: "`.epr-meta` Compose-Gate — Implementation Plan (P1)"
id: epr-meta-compose-gate-plan
status: Landed
created: 2026-06-25
landed: 2026-08-05
verified_by: |
  Landed-state verified against the tree 2026-08-05 during the worktree commit sweep, not by
  ticking the steps: .claude/hooks/epr-meta-resolver.py (20.8KB) is registered at
  .claude/settings.json:144; .claude/scripts/_lib/epr_meta.py (92.8KB) carries the cascade,
  schema check and pure-guard evaluator; elohim/sdk/schemas/v1/objects/epr-meta.schema.json is
  the contract; 42 .epr-meta manifests are deployed and observably enforcing (deny/ask/inject
  all fired during this sweep). The checkbox body below is a historical authoring record, NOT a
  work queue — it was never ticked as the work landed.
class: process-meta
process_subdomain: doc-lifecycle
topic: [epr-meta, compose-gate, resolver, cascade, hook, schema, recursion-guard, plan]
context-tier: disclosed
steward: cartographer
informed-by:
  - genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
cites:
  - epr-meta-compose-gate | `.epr-meta` | sha256:42f61de93a17196f | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - doc-lifecycle-as-epr-development-substrate | Doc-Lifecycle as EPR | sha256:4b87bca1eb683441 | path: genesis/docs/superpowers/specs/2026-06-25-doc-lifecycle-as-epr-development-substrate-design.md
---

# `.epr-meta` Compose-Gate Implementation Plan (P1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `.epr-meta` compose-gate — one generic PreToolUse resolver that enforces directory-local, cascading, declarative governance rules over document authoring, validated against a JSON-Schema contract and evaluated as pure guards.

**Architecture:** A thin Python PreToolUse hook (`epr-meta-resolver.py`) delegates to a testable `_lib/epr_meta.py` module. The module walks the directory cascade (ancestor `.epr-meta` files, root-first / nearest-wins, bounded depth — the recursion guard), validates each `.epr-meta` against a hand-rolled schema check, merges the rule sets, and evaluates the merged rules as **pure functions** against the proposed write — returning a most-severe `deny` / `ask` / `inject` / silent verdict that the hook emits as stdout JSON. Rules are declarative (a closed vocabulary); a `validator:` escape delegates to a named reference validator. The `.epr-meta` JSON Schema (draft-2020-12, `elohim/sdk/schemas/v1/objects/`) is the authoritative contract.

**Tech Stack:** Python 3 (stdlib + PyYAML for nested frontmatter), JSON Schema draft-2020-12, the repo's `.claude/scripts/_lib` hook utilities (`frontmatter`, `subject_routing` pattern).

## Scope — this plan is P1a (the author-time gate), not the whole P1 spec

The P1 spec (`2026-06-25-epr-meta-compose-gate-design`) lists six acceptance criteria spanning **two independent subsystems**. Per the writing-plans Scope Check they get **two plans**:

- **P1a — the compose-gate resolver (THIS PLAN).** A pure-Python, author-time PreToolUse gate. Ships value the moment it lands; needs no Rust, no conductor, no `elohim-epr`. Covers spec criteria **#1** (schema), **#3** (resolver: cascade → validate → merge → pure-guard → verdict, wired on `Write`/`Edit` + new-subdir), **#5** (seeded examples deny/ask), and the **cascade-depth + pure-guard** half of **#4** (recursion guard).
- **P1b — the canonical-envelope projector (SIBLING PLAN, not yet written).** Spec criterion **#2**: compile a source `.epr-meta` → DAG-CBOR envelope → CID via `elohim/epr` (`canonical_bytes`/`compute_cid`), so reformatting the source body leaves the atom's CID unchanged. This is the **graduation / seed-import** leg ("graduation into the protocol is a seeder import"). It is Rust + content-addressing — a distinct testable deliverable. **Out of scope here.**

Also deferred out of P1a (mapped to their spec criteria in the closing Self-Review notes): the **fuel / `extends:`-by-CID visited-set** half of criterion #4 (needs real validator-EPR atoms — the CID-hardening slice), and criterion **#6** self-hardening (override-counter + agent-authored-`deny` approval loop — a fast-follow). P1a ships the rule *vocabulary* that names `measure`/`dispatch`, not the downstream economy.

> **Status (executed 2026-06-25).** All five tasks landed + committed (`258ddb461`→`0ffca3df3`), sandbox-proven then real-repo-verified. An independent 3-lens review panel (spec/compliance ✅ · blast-radius ✅ · adversarial-security ⚠) then drove a hardening pass committed as `d42ac0dc1`: a malformed `.epr-meta` is now **strict-but-recoverable** (operator decision — never bricks the subtree, the manifest is always editable, broken governance downgrades `deny → ask`); a 64KB + flow-depth cap kills the YAML parse-DoS; `route-plans-out`/`p2p-design-gate` gate at creation (`new: true`); case-insensitive `*.md`; `validate_meta` warns on predicate-less rules + unknown keys. **The code blocks below show the original per-task structure; the committed (hardened) source is the source of truth.**

## Global Constraints

- **Verdict emission:** all decisions are a JSON object printed to **stdout** with `sys.exit(0)`. Block = `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":<str>}}`. Prompt = same with `"ask"`. Advisory = `{"hookSpecificOutput":{"hookEventName":"PreToolUse","additionalContext":<str>}}` (no `permissionDecision`). Silent allow = print nothing. **Exit 2 is NEVER used; exit 1 = internal hook error only.** (Source: `seed-data-validation.py:303-321`.)
- **Fail-open (mandatory):** wrap `main()` in `try/except`; on internal exception → print to stderr, `sys.exit(1)`. On malformed stdin JSON → `sys.exit(0)` silently. A guard bug must never block dev.
- **Frontmatter/YAML:** split block/body with `_lib.frontmatter`; parse the nested rule config with `yaml.safe_load(fm.raw_block)`. **If PyYAML is unavailable, emit a LOUD advisory and fail-open — never silently no-op a governance gate.**
- **Schema:** `elohim/sdk/schemas/v1/objects/epr-meta.schema.json`, draft-2020-12, **no `$ref`** (self-contained leaf), `"additionalProperties": false`, description contains `"Source of truth:"` and `"Category C (operational/config; not notarized)"`.
- **Reuse, don't reinvent:** cascade + merge mirror `_lib.subject_routing`'s root-first/nearest-wins pattern; frontmatter via `_lib.frontmatter` (NOT `placement-audit.py`'s inline parser).
- **Tests (repo convention — NOT pytest):** one runnable script per task at `.claude/scripts/_lib/__tests__/<name>_test.py` (template: `.claude/scripts/_lib/__tests__/subject_routing_test.py`) — self-contained `_lib` bootstrap header, a `check(label, cond)` assert-helper, `tempfile.TemporaryDirectory()` (no fixtures / no `tmp_path` / no `monkeypatch`), run via `python3 <file>` (exit 0 = pass; prints `N assertions passed ✅`). **pytest is not installed.**
- **Pure guards:** rule evaluation reads and returns a verdict; it performs **no writes**. Side-effects are the host's job, to a non-governed path.
- **Enforcement-class enum:** `deny | ask | inject | measure | dispatch`.
- **Git:** commit on the working branch; do NOT push (integrator owns pushes).

---

### Task 1: The `.epr-meta` JSON-Schema contract

**Files:**
- Create: `elohim/sdk/schemas/v1/objects/epr-meta.schema.json`
- Test: `.claude/scripts/_lib/__tests__/epr_meta_schema_test.py`

**Interfaces:**
- Produces: the schema file at the path above; the canonical valid-instance shape every later task validates against.

- [ ] **Step 1: Write the failing test**

```python
# .claude/scripts/_lib/__tests__/epr_meta_schema_test.py
"""Schema-contract test for .epr-meta. Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_schema_test.py  (exit 0 = pass)"""
import json, sys
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        break
    here = here.parent
REPO = here  # dir holding .claude/

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

s = json.loads((REPO / "elohim/sdk/schemas/v1/objects/epr-meta.schema.json").read_text())
check("$schema draft 2020-12", s["$schema"] == "https://json-schema.org/draft/2020-12/schema")
check("$id", s["$id"] == "epr:schema:objects:epr-meta")
check("title EprMeta", s["title"] == "EprMeta")
check("additionalProperties false", s["additionalProperties"] is False)
check("declares Source of truth:", "Source of truth:" in s["description"])
check("declares Category C", "Category C" in s["description"])
check("self-contained leaf (no $ref)", "$ref" not in json.dumps(s))
check("requires epr-meta-version", "epr-meta-version" in s["required"])
check("rule class enum is the canonical 5",
      s["properties"]["rules"]["items"]["properties"]["class"]["enum"]
      == ["deny", "ask", "inject", "measure", "dispatch"])

print(f"\n  {_passed} assertions passed ✅")
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_schema_test.py`
Expected: FAIL — `FileNotFoundError` (schema does not exist yet).

- [ ] **Step 3: Write the schema**

```json
// elohim/sdk/schemas/v1/objects/epr-meta.schema.json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "epr:schema:objects:epr-meta",
  "title": "EprMeta",
  "description": "Source of truth: the directory-local compose-gate manifest (.epr-meta). A self-contained leaf config read by the epr-meta-resolver PreToolUse hook; not an HTTP wire shape. P2P design-gate classification: Category C (operational/config; not notarized).",
  "type": "object",
  "additionalProperties": false,
  "required": ["epr-meta-version"],
  "properties": {
    "epr-meta-version": { "type": "integer", "const": 1 },
    "id": { "type": "string" },
    "root": { "type": "boolean" },
    "extends": { "type": "string" },
    "max-cascade-depth": { "type": "integer", "minimum": 1, "maximum": 32 },
    "purpose": { "type": "string" },
    "rules": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["id", "class"],
        "properties": {
          "id": { "type": "string" },
          "class": { "type": "string", "enum": ["deny", "ask", "inject", "measure", "dispatch"] },
          "why": { "type": "string" },
          "when": { "type": "object" },
          "require-frontmatter": { "type": "array", "items": { "type": "string" } },
          "allowed-types": { "type": "array", "items": { "type": "string" } },
          "route-to": { "type": "object" },
          "no-new-subdirs": { "type": "boolean" },
          "require-sibling": { "type": "string" },
          "dedupe-of": { "type": "string" },
          "max-files": { "type": "object" },
          "measure": { "type": "object" },
          "validator": { "type": "string" }
        }
      }
    },
    "validators": {
      "type": "array",
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["ref"],
        "properties": {
          "ref": { "type": "string" },
          "cid": { "type": "string" },
          "fuel": { "type": "integer", "minimum": 1 }
        }
      }
    },
    "cites": { "type": "array", "items": { "type": "string" } }
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_schema_test.py`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add elohim/sdk/schemas/v1/objects/epr-meta.schema.json .claude/scripts/_lib/__tests__/epr_meta_schema_test.py
git commit -m "feat(epr-meta): add .epr-meta JSON-Schema contract (P1 Task 1)"
```

---

### Task 2: The cascade + merge (`_lib/epr_meta.py`)

**Files:**
- Create: `.claude/scripts/_lib/epr_meta.py`
- Test: `.claude/scripts/_lib/__tests__/epr_meta_cascade_test.py`

**Interfaces:**
- Consumes: `_lib.frontmatter.parse_file` / `parse`; PyYAML (`yaml.safe_load`).
- Produces:
  - `MAX_CASCADE_DEPTH: int = 32`
  - `load_meta(path: Path) -> dict` — parse one `.epr-meta` (frontmatter block → nested dict via yaml); `{}` on failure.
  - `find_repo_root(start: Path) -> Path` — `.git`-authoritative, ≤12-level walk.
  - `collect_cascade(target: Path) -> list[Path]` — ancestor `.epr-meta` files, **root-first** (nearest last), bounded by depth and a `root: true` base case.
  - `merge_rules(chain: list[Path]) -> dict` — `{"rules": {id: rule}, "validators": {ref: v}, "sources": [str]}`, nearest-wins.

- [ ] **Step 1: Write the failing test**

```python
# .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py`
Expected: FAIL — `ModuleNotFoundError: No module named '_lib.epr_meta'`.

- [ ] **Step 3: Write the cascade + merge**

```python
# .claude/scripts/_lib/epr_meta.py
"""The .epr-meta cascade + merge — the reuse of subject_routing's root-first/nearest-wins
ancestor walk, retargeted to the `.epr-meta` manifest with nested-YAML parsing."""
from __future__ import annotations
from pathlib import Path

from _lib import frontmatter as fm

try:
    import yaml  # PyYAML — needed for the nested rule config
except Exception:  # pragma: no cover
    yaml = None

MANIFEST_NAME = ".epr-meta"
MAX_CASCADE_DEPTH = 32


def yaml_available() -> bool:
    return yaml is not None


def load_meta(path: Path) -> dict:
    """Parse one .epr-meta (frontmatter block -> nested dict). {} on any failure (caller fails open)."""
    try:
        block = fm.parse_file(path).raw_block
        if yaml is None:
            return {}
        data = yaml.safe_load(block) or {}
        return data if isinstance(data, dict) else {}
    except Exception:
        return {}


def find_repo_root(start: Path) -> Path:
    here = start.resolve()
    if here.is_file():
        here = here.parent
    for _ in range(12):
        if (here / ".git").exists():
            return here
        if here.parent == here:
            break
        here = here.parent
    return start.resolve().parent if start.is_file() else start.resolve()


def collect_cascade(target: Path) -> list[Path]:
    """Ancestor .epr-meta files from `target`'s dir up to a `root: true` base (or repo root),
    bounded by MAX_CASCADE_DEPTH. Returned ROOT-FIRST so nearest wins on merge."""
    target = target.resolve()
    here = target.parent if (target.is_file() or not target.exists()) else target
    root = find_repo_root(here)
    chain: list[Path] = []
    depth = 0
    while depth < MAX_CASCADE_DEPTH:
        meta = here / MANIFEST_NAME
        if meta.is_file():
            chain.append(meta)
            if load_meta(meta).get("root") is True:
                break
        if here == root or here.parent == here:
            break
        here = here.parent
        depth += 1
    return list(reversed(chain))  # root-first


def merge_rules(chain: list[Path]) -> dict:
    """Merge root-first; nearer overrides on rule `id` / validator `ref`."""
    merged: dict = {"rules": {}, "validators": {}, "sources": []}
    for meta in chain:
        cfg = load_meta(meta)
        merged["sources"].append(str(meta))
        for rule in cfg.get("rules", []) or []:
            rid = rule.get("id")
            if rid:
                merged["rules"][rid] = rule
        for v in cfg.get("validators", []) or []:
            ref = v.get("ref")
            if ref:
                merged["validators"][ref] = v
    return merged
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py`
Expected: PASS (both tests).

- [ ] **Step 5: Commit**

```bash
git add .claude/scripts/_lib/epr_meta.py .claude/scripts/_lib/__tests__/epr_meta_cascade_test.py
git commit -m "feat(epr-meta): cascade + merge (root-first/nearest-wins, bounded) (P1 Task 2)"
```

---

### Task 3: Hand-rolled schema-validate + pure-guard evaluator

**Files:**
- Modify: `.claude/scripts/_lib/epr_meta.py`
- Test: `.claude/scripts/_lib/__tests__/epr_meta_eval_test.py`

**Interfaces:**
- Consumes: `merge_rules` output; the proposed-write dict `{"path": str, "content": str | None, "is_new": bool, "is_new_subdir": bool}` (`is_new_subdir` = the write's parent dir does not yet exist; Task 4's hook computes it).
- Produces:
  - `ENFORCEMENT_CLASSES = ("deny", "ask", "inject", "measure", "dispatch")`
  - `Verdict` namedtuple `(cls: str, reason: str, rule_id: str)`
  - `validate_meta(cfg: dict) -> list[str]` — hand-rolled schema check; list of error strings (empty = valid).
  - `evaluate(merged: dict, write: dict) -> list[Verdict]` — pure; returns the fired verdicts.
  - `combine(verdicts: list[Verdict]) -> Verdict | None` — most-severe-wins (`deny > ask > inject`; `measure`/`dispatch` are side-channels).

- [ ] **Step 1: Write the failing test**

```python
# .claude/scripts/_lib/__tests__/epr_meta_eval_test.py
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_eval_test.py`
Expected: FAIL — `AttributeError: module '_lib.epr_meta' has no attribute 'validate_meta'`.

- [ ] **Step 3: Append the validator + evaluator**

```python
# append to .claude/scripts/_lib/epr_meta.py
import fnmatch
from collections import namedtuple

from _lib import frontmatter as _fm

ENFORCEMENT_CLASSES = ("deny", "ask", "inject", "measure", "dispatch")
_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}
Verdict = namedtuple("Verdict", ["cls", "reason", "rule_id"])


def validate_meta(cfg: dict) -> list[str]:
    """Hand-rolled, stdlib-only check against the schema contract. [] = valid."""
    errs: list[str] = []
    if not isinstance(cfg, dict):
        return ["`.epr-meta` is not a mapping"]
    if cfg.get("epr-meta-version") != 1:
        errs.append("missing/invalid `epr-meta-version` (must be 1)")
    for i, rule in enumerate(cfg.get("rules", []) or []):
        if not isinstance(rule, dict):
            errs.append(f"rules[{i}] is not a mapping"); continue
        if "id" not in rule:
            errs.append(f"rules[{i}] missing `id`")
        cls = rule.get("class")
        if cls not in ENFORCEMENT_CLASSES:
            errs.append(f"rules[{i}] (`{rule.get('id','?')}`) class `{cls}` not in {ENFORCEMENT_CLASSES}")
    for i, v in enumerate(cfg.get("validators", []) or []):
        if not isinstance(v, dict) or "ref" not in v:
            errs.append(f"validators[{i}] missing `ref`")
    return errs


def _matches_when(when: dict, write: dict) -> bool:
    if not when:
        return True
    name = Path(write["path"]).name
    pat = when.get("write")
    if pat and not fnmatch.fnmatch(name, pat):
        return False
    if when.get("new") is True and not write.get("is_new", False):
        return False
    content = write.get("content") or ""
    needles = when.get("contains-any") or ([when["contains"]] if "contains" in when else [])
    if needles and not any(n in content for n in needles):
        return False
    return True


def _frontmatter_fields(content: str | None) -> set[str]:
    if not content:
        return set()
    return set(_fm.parse(content).fields.keys())


def _eval_rule(rule: dict, write: dict, merged: dict) -> Verdict | None:
    cls = rule.get("class", "inject")
    rid = rule.get("id", "?")
    why = rule.get("why", "")
    if not _matches_when(rule.get("when", {}), write):
        return None

    if "require-frontmatter" in rule:
        present = _frontmatter_fields(write.get("content"))
        missing = [f for f in rule["require-frontmatter"] if f not in present]
        if missing:
            return Verdict(cls, f"missing required frontmatter {missing}. {why}", rid)
        return None

    if "route-to" in rule:
        dest = rule["route-to"].get("dest", "?")
        return Verdict(cls, f"{Path(write['path']).name} routes to {dest}. {why}", rid)

    if rule.get("no-new-subdirs"):
        # In Claude Code there is no separate dir-create event: a Write whose parent dir does
        # not yet exist IS the new-subdir signal (the hook sets is_new_subdir, §Task 4).
        if write.get("is_new_subdir"):
            return Verdict(cls, f"new subdirectories are not allowed here. {why}", rid)
        return None

    if "require-sibling" in rule:
        sibling = rule["require-sibling"]
        # A new subtree must carry its own .epr-meta (the no-orphan-tree guard the user named:
        # "creating a new tree without a memkit manifest"). The .epr-meta itself is exempt.
        if write.get("is_new_subdir") and Path(write["path"]).name != sibling:
            return Verdict(cls, f"a new subtree must carry its own `{sibling}`. {why}", rid)
        return None

    if "dedupe-of" in rule:
        return Verdict(cls, f"this concern already lives at {rule['dedupe-of']}. {why}", rid)

    if "validator" in rule:
        # v1: the validator escape resolves a NAMED reference validator (CID-pinning is a later
        # hardening). Unknown validators advise rather than block, so a missing validator never
        # hard-blocks dev.
        ref = rule["validator"]
        if ref not in REFERENCE_VALIDATORS:
            return Verdict("inject", f"validator `{ref}` not registered (advisory). {why}", rid)
        if REFERENCE_VALIDATORS[ref](write):
            return Verdict(cls, f"validator `{ref}` flagged this write. {why}", rid)
        return None

    # measure / max-files are side-channels: surfaced as inject in v1 (host wires the counter later)
    if "measure" in rule or "max-files" in rule:
        return None
    return None


# Named reference validators (the escape hatch). v1 ships one, cloned from p2p-plan-audit's detector.
def _p2p_design_gate(write: dict) -> bool:
    c = (write.get("content") or "")
    return any(s in c for s in ("GET /api/v1", "PRIMARY KEY", "uuid"))


REFERENCE_VALIDATORS = {"epr:validator-p2p-design-gate": _p2p_design_gate}


def evaluate(merged: dict, write: dict) -> list[Verdict]:
    """PURE: read rules + write, return fired verdicts. No writes, no side-effects."""
    out: list[Verdict] = []
    for rule in merged.get("rules", {}).values():
        v = _eval_rule(rule, write, merged)
        if v is not None:
            out.append(v)
    return out


def combine(verdicts: list[Verdict]) -> Verdict | None:
    blocking = [v for v in verdicts if v.cls in ("deny", "ask", "inject")]
    if not blocking:
        return None
    return max(blocking, key=lambda v: _SEVERITY[v.cls])
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_eval_test.py`
Expected: PASS (all five tests).

- [ ] **Step 5: Commit**

```bash
git add .claude/scripts/_lib/epr_meta.py .claude/scripts/_lib/__tests__/epr_meta_eval_test.py
git commit -m "feat(epr-meta): hand-rolled validate + pure-guard evaluator (P1 Task 3)"
```

---

### Task 4: The PreToolUse resolver hook

**Files:**
- Create: `.claude/hooks/epr-meta-resolver.py`
- Test: `.claude/scripts/_lib/__tests__/epr_meta_resolver_test.py`

**Interfaces:**
- Consumes: stdin JSON `{"tool_name","tool_input":{"file_path","content"}}`; `_lib.epr_meta`.
- Produces: the hook executable that emits the Global-Constraints verdict JSON; fail-open.

- [ ] **Step 1: Write the failing test**

```python
# .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py
"""Resolver-hook test (subprocess; verdict JSON on stdout). Run:
python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py  (exit 0 = pass)"""
import json, subprocess, sys, tempfile, textwrap
from pathlib import Path

here = Path(__file__).resolve()
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        break
    here = here.parent
REPO = here
HOOK = REPO / ".claude/hooks/epr-meta-resolver.py"

_passed = 0
def check(label, cond):
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")

def _wr(p, body):
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(textwrap.dedent(body).lstrip())

def _hook(payload=None, stdin=None):
    text = stdin if stdin is not None else json.dumps(payload)
    return subprocess.run([sys.executable, str(HOOK)], input=text, capture_output=True, text=True)

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: fm-at-birth
            class: deny
            when: { write: "*.md", new: true }
            require-frontmatter: [id, status]
            why: "no doc without id+status"
        ---
    """)
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "new.md"), "content": "bare"}})
    check("deny exit 0", r.returncode == 0)
    check("deny verdict", json.loads(r.stdout)["hookSpecificOutput"]["permissionDecision"] == "deny")
    r = _hook({"tool_name": "Write", "tool_input":
               {"file_path": str(root / "ok.md"), "content": "---\nid: x\nstatus: Draft\n---\n"}})
    check("silent-allow when frontmatter present", r.returncode == 0 and r.stdout.strip() == "")
    r = _hook({"tool_name": "Write", "tool_input": {"file_path": str(root / "x.py"), "content": "print(1)"}})
    check("silent on non-md", r.returncode == 0 and r.stdout.strip() == "")

r = _hook(stdin="not json")
check("fails open on malformed stdin", r.returncode == 0 and r.stdout.strip() == "")

with tempfile.TemporaryDirectory() as _td:
    root = Path(_td)
    (root / ".git").mkdir()
    _wr(root / ".epr-meta", """
        ---
        epr-meta-version: 1
        root: true
        rules:
          - id: orphan
            class: ask
            require-sibling: .epr-meta
            why: "new tree needs a manifest"
        ---
    """)
    r = _hook({"tool_name": "Write", "tool_input":
               {"file_path": str(root / "brand" / "note.md"), "content": "x"}})
    out = json.loads(r.stdout)
    check("wires is_new_subdir (ask on orphan tree)",
          out["hookSpecificOutput"]["permissionDecision"] == "ask")
    check("orphan-tree reason names .epr-meta",
          ".epr-meta" in out["hookSpecificOutput"]["permissionDecisionReason"])

print(f"\n  {_passed} assertions passed ✅")
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py`
Expected: FAIL — hook file does not exist.

- [ ] **Step 3: Write the hook**

```python
#!/usr/bin/env python3
# .claude/hooks/epr-meta-resolver.py
"""PreToolUse resolver for the .epr-meta compose-gate. Thin: stdin -> _lib.epr_meta -> verdict JSON.
Fail-open: a guard bug never blocks dev."""
import json
import sys
from pathlib import Path

# --- _lib bootstrap (clone of managed-surface-context.py:26-32) ---
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402


def _emit_deny(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "deny",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_ask(reason: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "permissionDecision": "ask",
        "permissionDecisionReason": reason}}))
    sys.exit(0)


def _emit_advise(text: str):
    print(json.dumps({"hookSpecificOutput": {
        "hookEventName": "PreToolUse", "additionalContext": text}}))
    sys.exit(0)


def main():
    raw = sys.stdin.read()
    try:
        data = json.loads(raw)
    except Exception:
        sys.exit(0)  # malformed stdin -> fail open silently

    tool = data.get("tool_name", "")
    if tool not in ("Write", "Edit"):
        sys.exit(0)
    ti = data.get("tool_input", {}) or {}
    fp = ti.get("file_path", "")
    if not fp:
        sys.exit(0)

    target = Path(fp)
    is_new = (tool == "Write") and not target.exists()
    # No separate dir-create event in Claude Code: a Write whose parent dir does not yet exist
    # is the new-subdir signal (drives no-new-subdirs / require-sibling, §Task 3).
    is_new_subdir = (tool == "Write") and not target.parent.exists()
    # Write carries full content; Edit does not -> read on-disk for frontmatter checks.
    content = ti.get("content")
    if content is None and target.exists():
        try:
            content = target.read_text(errors="replace")
        except Exception:
            content = None

    chain = epr_meta.collect_cascade(target)
    if not chain:
        sys.exit(0)  # no governance here

    if not epr_meta.yaml_available():
        _emit_advise("[.epr-meta] PyYAML unavailable — compose-gate rules NOT enforced for this "
                     "write. Install PyYAML to re-enable the gate. (failing open, not silent.)")

    # schema-validate every .epr-meta in the cascade; a malformed manifest is itself a deny.
    metas = [(m, epr_meta.load_meta(m)) for m in chain]
    for meta, cfg in metas:
        errs = epr_meta.validate_meta(cfg)
        if errs:
            _emit_deny(f"malformed `.epr-meta` at {meta}: {'; '.join(errs)}")

    # Recursion-guard surface 6.2: a governed subtree whose cascade reached the repo/depth bound
    # without a `root: true` constitutional base is a misconfiguration. v1 ADVISES (fail-open
    # friendly); the spec's stricter `deny` is a hardening once the root is repo-wide.
    advisories = []
    if not any(cfg.get("root") is True for _, cfg in metas):
        advisories.append("[.epr-meta] no `root: true` constitutional base in this cascade — "
                          "add one (this subtree's governance has no anchor).")

    merged = epr_meta.merge_rules(chain)
    write = {"path": fp, "content": content, "is_new": is_new,
             "is_new_subdir": is_new_subdir}
    verdict = epr_meta.combine(epr_meta.evaluate(merged, write))
    if verdict is None:
        if advisories:
            _emit_advise(" ".join(advisories))
        sys.exit(0)  # silent allow
    src = merged["sources"][-1] if merged["sources"] else "?"
    msg = f"{verdict.reason} [rule `{verdict.rule_id}` from {src}]"
    if verdict.cls == "deny":
        _emit_deny(msg)
    elif verdict.cls == "ask":
        _emit_ask(msg)
    else:
        _emit_advise(" ".join([msg, *advisories]))


if __name__ == "__main__":
    try:
        main()
    except SystemExit:
        raise
    except Exception as e:  # fail-open: internal error, NOT a deny
        print(f"epr-meta-resolver internal error: {e}", file=sys.stderr)
        sys.exit(1)
```

- [ ] **Step 4: Run test to verify it passes**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py`
Expected: PASS (all four tests).

- [ ] **Step 5: Commit**

```bash
git add .claude/hooks/epr-meta-resolver.py .claude/scripts/_lib/__tests__/epr_meta_resolver_test.py
git commit -m "feat(epr-meta): PreToolUse resolver hook (fail-open, stdout-JSON verdict) (P1 Task 4)"
```

---

### Task 5: Register the hook + seed the two example `.epr-meta` files

**Files:**
- Modify: `.claude/settings.json` (append to the existing `PreToolUse` array)
- Create: `genesis/docs/superpowers/.epr-meta` (the base/root-of-subtree manifest)
- Create: `genesis/docs/superpowers/specs/.epr-meta` (nested override demonstrating the cascade)
- Test: `.claude/scripts/_lib/__tests__/epr_meta_examples_test.py`

**Interfaces:**
- Consumes: the resolver from Task 4; the cascade from Task 2.
- Produces: a live, registered gate over `genesis/docs/superpowers/{specs}/`.

- [ ] **Step 1: Write the failing test** (drives the example files, end-to-end through the lib)

```python
# .claude/scripts/_lib/__tests__/epr_meta_examples_test.py
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
```

- [ ] **Step 2: Run test to verify it fails**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_examples_test.py`
Expected: FAIL — the `.epr-meta` example files do not exist yet.

- [ ] **Step 3a: Write the base manifest** `genesis/docs/superpowers/.epr-meta`

```yaml
---
epr-meta-version: 1
id: superpowers-docs-governance
root: true
purpose: >
  Specs and plans — the authored design surface. Born-linked, decomposes to gaps,
  graduates to history; never parked. See the framing spec for the full law.
rules:
  - id: doc-frontmatter-at-birth
    class: deny
    when: { write: "*.md", new: true }
    require-frontmatter: [id, status, cites]
    why: "No doc born without id + status + cites (the .epr-meta law)."
validators: []
max-cascade-depth: 8
cites:
  - 2026-06-25-doc-lifecycle-as-epr-development-substrate-design
---

# superpowers/ — authored design surface

The base governance for specs and plans. Child `.epr-meta` files tighten this per-directory.
```

- [ ] **Step 3b: Write the nested override** `genesis/docs/superpowers/specs/.epr-meta`

```yaml
---
epr-meta-version: 1
id: specs-dir-governance
extends: ../.epr-meta
purpose: "Design specs — the 'what/why'. Plans route to plans/."
rules:
  - id: doc-frontmatter-at-birth
    class: deny
    when: { write: "*.md", new: true }
    require-frontmatter: [id, status, class, context-tier, steward, graduation-trigger, cites]
    why: "Specs need the full lifecycle frontmatter (overrides the base rule, nearest-wins)."
  - id: route-plans-out
    class: ask
    when: { write: "*-plan.md" }
    route-to: { type: "*-plan.md", dest: genesis/docs/superpowers/plans/ }
    why: "Plans live in plans/, not specs/."
  - id: p2p-design-gate
    class: ask
    when: { write: "*.md", contains-any: ["GET /api/v1", "PRIMARY KEY", "uuid"] }
    validator: epr:validator-p2p-design-gate
    why: "Data-entity designs pass the p2p-design-gate."
validators:
  - ref: epr:validator-p2p-design-gate
    fuel: 200
max-cascade-depth: 8
---

# specs/ — the 'what/why'

Born from `/brainstorm`. Decomposes to gaps; graduates or is superseded; never parked.
```

- [ ] **Step 3c: Register the hook** — append to the `PreToolUse` array in `.claude/settings.json`:

```json
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "python3 \"$CLAUDE_PROJECT_DIR/.claude/hooks/epr-meta-resolver.py\"",
            "timeout": 3000
          }
        ]
      }
```

- [ ] **Step 4: Run tests + verify the registration parses**

Run: `python3 .claude/scripts/_lib/__tests__/epr_meta_examples_test.py`
Expected: PASS (all three).
Run: `python3 -c "import json; json.load(open('.claude/settings.json')); print('settings.json OK')"`
Expected: `settings.json OK`.

- [ ] **Step 5: Seal cites + commit**

```bash
python3 .claude/scripts/memory-kit/cite-gen.py --seal genesis/docs/superpowers/.epr-meta || true
git add .claude/settings.json genesis/docs/superpowers/.epr-meta genesis/docs/superpowers/specs/.epr-meta .claude/scripts/_lib/__tests__/epr_meta_examples_test.py
git commit -m "feat(epr-meta): register resolver + seed base/nested example .epr-meta (P1 Task 5)"
```

---

## Self-Review notes (carried into execution)

- **Test convention (resolved in pre-flight):** pytest is NOT installed; tests are plain runnable scripts at `.claude/scripts/_lib/__tests__/<name>_test.py` (modeled on `subject_routing_test.py`), each run via `python3 <file>` (exit 0 = pass). No auto-discovery runner — like its siblings each `_test.py` is run on demand. PyYAML 6.0.3 is present; the `_lib.frontmatter` API (`.raw_block`, `.fields`) and the `.claude/settings.json` `PreToolUse` array shape were both verified against the real files.
- **PyYAML:** the resolver requires PyYAML for nested rules and emits a loud advisory (never a silent no-op) if absent — confirm PyYAML is present in the hook's Python (`subject_routing.py` already imports it).
- **False-positive hooks on THIS plan file:** the P2P-design-audit flags ("New storage schema without source-of-truth declaration") fire on the illustrative `*.schema.json` block quoted in Task 1 — but that schema's `description` literally declares `"Source of truth:"` and Category-C, and a plan is not a live storage schema. No action. The `[ref](write)` EPR-link tip fires on a Python f-string, not a markdown link. No action.

### Spec-criterion coverage map (acceptance criteria of the P1 spec → where they land)

| Spec criterion | Status in P1a (this plan) |
|---|---|
| #1 schema in `v1/` + contract test | **Task 1** (Python wellformedness/enum test; Rust `schema_contract.rs` test deferred per grounding) |
| #2 projector source→envelope→CID, reformat-stable | **P1b sibling plan** (Rust/`elohim-epr`) — out of scope here |
| #3 resolver cascade→validate→merge→pure-guard→verdict, wired Write/Edit + dir-create | **Tasks 2–5** (dir-create realized as the `is_new_subdir` signal) |
| #4 recursion guard (visited-set / missing-root / fuel) | **Partial:** cascade-depth bound + `root:true` base + pure guards (Task 2) + missing-root advisory (Task 4). `extends:`-by-CID visited-set + validator-chain fuel → **CID-hardening slice** (needs real validator-EPR atoms) |
| #5 seeded root + `specs/` examples deny/ask | **Task 5** |
| #6 self-hardening (override-counter + agent-`deny` approval) | **Fast-follow, not this plan** — needs the `measure` side-channel host-write (the same host-write the §6.3 guard isolates to non-governed paths). P1a ships the vocabulary that *names* `measure`/`dispatch`; the economy that consumes it is P3/P5. |
