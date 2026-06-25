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
MAX_MANIFEST_BYTES = 64 * 1024   # refuse to parse an oversized manifest (parse-DoS guard)
MAX_FLOW_DEPTH = 64              # reject deeply-nested flow YAML before PyYAML can RecursionError


def yaml_available() -> bool:
    return yaml is not None


def _flow_depth_ok(text: str) -> bool:
    """Cheap O(n) guard: reject a manifest whose flow-collection nesting (`[`/`{`) exceeds
    MAX_FLOW_DEPTH, so a `[[[[…]]]]` parse-bomb never reaches PyYAML (which RecursionErrors ~2s,
    against a 3000ms hook budget). Quoted brackets only ever over-count → conservative-safe."""
    depth = 0
    for ch in text:
        if ch in "[{":
            depth += 1
            if depth > MAX_FLOW_DEPTH:
                return False
        elif ch in "]}":
            depth = depth - 1 if depth > 0 else 0
    return True


def _read_block(path: Path):
    """Size-capped, depth-capped read of the frontmatter block. None if oversized / too-deep /
    unreadable — so the cascade/merge never hand a poisoned manifest to the YAML parser."""
    try:
        if path.stat().st_size > MAX_MANIFEST_BYTES:
            return None
        block = fm.parse_file(path).raw_block
    except Exception:
        return None
    return block if _flow_depth_ok(block) else None


def load_meta(path: Path) -> dict:
    """Parse one .epr-meta (frontmatter block -> nested dict). {} on any failure (caller fails open).
    Size + flow-depth capped so cascade/merge never parse a poisoned manifest. For the precise
    REASON a manifest is unusable (for the resolver's advisory), use check_meta()."""
    block = _read_block(path)
    if block is None or yaml is None:
        return {}
    try:
        data = yaml.safe_load(block) or {}
    except Exception:
        return {}
    return data if isinstance(data, dict) else {}


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


# ── Task 3: hand-rolled schema-validate + pure-guard evaluator ──
import fnmatch
from collections import namedtuple

from _lib import frontmatter as _fm

ENFORCEMENT_CLASSES = ("deny", "ask", "inject", "measure", "dispatch")
_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}
Verdict = namedtuple("Verdict", ["cls", "reason", "rule_id"])

# A rule of an enforcing class must carry at least one of these actionable predicate keys, else it
# fires on nothing (a "deny everything here" that silently allows — the M2 footgun).
_ACTIONABLE_KEYS = ("require-frontmatter", "allowed-types", "route-to", "no-new-subdirs",
                    "require-sibling", "dedupe-of", "validator")
_KNOWN_RULE_KEYS = {"id", "class", "why", "when", "max-files", "measure"} | set(_ACTIONABLE_KEYS)


def validate_meta(cfg: dict) -> list[str]:
    """Hand-rolled, stdlib-only check against the schema contract. [] = valid.
    Also surfaces footguns: an enforcing rule with no actionable predicate, and unknown rule keys
    (a typo'd key would otherwise be silently ignored by _eval_rule)."""
    errs: list[str] = []
    if not isinstance(cfg, dict):
        return ["`.epr-meta` is not a mapping"]
    if cfg.get("epr-meta-version") != 1:
        errs.append("missing/invalid `epr-meta-version` (must be 1)")
    for i, rule in enumerate(cfg.get("rules", []) or []):
        if not isinstance(rule, dict):
            errs.append(f"rules[{i}] is not a mapping"); continue
        rid = rule.get("id", "?")
        if "id" not in rule:
            errs.append(f"rules[{i}] missing `id`")
        cls = rule.get("class")
        if cls not in ENFORCEMENT_CLASSES:
            errs.append(f"rules[{i}] (`{rid}`) class `{cls}` not in {ENFORCEMENT_CLASSES}")
        unknown = set(rule) - _KNOWN_RULE_KEYS
        if unknown:
            errs.append(f"rules[{i}] (`{rid}`) unknown key(s) {sorted(unknown)} (typo?)")
        if cls in ("deny", "ask", "inject") and not any(k in rule for k in _ACTIONABLE_KEYS):
            errs.append(f"rules[{i}] (`{rid}`) class `{cls}` has no actionable predicate "
                        f"(require-frontmatter/route-to/no-new-subdirs/…) — it fires on nothing")
    for i, v in enumerate(cfg.get("validators", []) or []):
        if not isinstance(v, dict) or "ref" not in v:
            errs.append(f"validators[{i}] missing `ref`")
    return errs


def check_meta(path: Path) -> list[str]:
    """Health-check one .epr-meta FOR THE RESOLVER: [] = healthy. Reports the precise reason a
    manifest is unusable (size cap / too-deep / unparseable / empty / schema errors) so the hook can
    advise specifically. Distinct from load_meta(), which just returns {} on any failure for the
    cascade/merge fast-path."""
    try:
        size = path.stat().st_size
    except Exception:
        return []  # missing — collect_cascade only includes is_file() metas, so not our concern
    if size > MAX_MANIFEST_BYTES:
        return [f"exceeds {MAX_MANIFEST_BYTES // 1024}KB size cap — refusing to parse"]
    try:
        block = fm.parse_file(path).raw_block
    except Exception:
        return ["unreadable"]
    if not _flow_depth_ok(block):
        return ["nesting too deep (possible parse-bomb) — refusing to parse"]
    if yaml is None:
        return []  # PyYAML-absent is surfaced as its own advisory by the hook, not a per-manifest error
    try:
        data = yaml.safe_load(block)
    except Exception:
        return ["not valid YAML"]
    if data is None:
        return ["empty manifest (no frontmatter / no `epr-meta-version`)"]
    if not isinstance(data, dict):
        return ["top-level must be a mapping"]
    return validate_meta(data)


def _matches_when(when: dict, write: dict) -> bool:
    if not when:
        return True
    name = Path(write["path"]).name
    pat = when.get("write")
    # case-insensitive: fnmatch uses os.path.normcase (a no-op on Linux), so `*.md` would miss
    # `FILE.MD` — fold both sides so extension case never bypasses a rule (M1).
    if pat and not fnmatch.fnmatch(name.lower(), pat.lower()):
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
        # not yet exist IS the new-subdir signal (the hook sets is_new_subdir).
        if write.get("is_new_subdir"):
            return Verdict(cls, f"new subdirectories are not allowed here. {why}", rid)
        return None

    if "require-sibling" in rule:
        sibling = rule["require-sibling"]
        if write.get("is_new_subdir") and Path(write["path"]).name != sibling:
            return Verdict(cls, f"a new subtree must carry its own `{sibling}`. {why}", rid)
        return None

    if "dedupe-of" in rule:
        return Verdict(cls, f"this concern already lives at {rule['dedupe-of']}. {why}", rid)

    if "validator" in rule:
        ref = rule["validator"]
        if ref not in REFERENCE_VALIDATORS:
            return Verdict("inject", f"validator `{ref}` not registered (advisory). {why}", rid)
        if REFERENCE_VALIDATORS[ref](write):
            return Verdict(cls, f"validator `{ref}` flagged this write. {why}", rid)
        return None

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
