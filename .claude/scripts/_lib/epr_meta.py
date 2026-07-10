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
MANIFEST_FILE_NAME = "manifest.md"
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


def manifest_path(path: Path) -> Path:
    """Resolve an authored epr-meta path. Directory form wins when `path` is `.epr-meta/`."""
    path = Path(path)
    if path.is_dir():
        directory_manifest = path / MANIFEST_FILE_NAME
        if directory_manifest.is_file():
            return directory_manifest
    return path


def manifest_for_dir(directory: Path) -> Path | None:
    """Return the manifest for a directory: `.epr-meta/manifest.md`, then legacy `.epr-meta`."""
    base = Path(directory) / MANIFEST_NAME
    directory_manifest = base / MANIFEST_FILE_NAME
    if directory_manifest.is_file():
        return directory_manifest
    if base.is_file():
        return base
    return None


def is_manifest_path(path: Path) -> bool:
    """True if `path` names an authored governance manifest — the legacy flat `.epr-meta` file OR
    the directory form `.epr-meta/manifest.md`. Purely lexical (no disk touch) so it classifies a
    to-be-written path. Adapters use this to hold the invariant "editing an .epr-meta is never
    blocked so the fix is never bricked" across BOTH forms."""
    path = Path(path)
    if path.name == MANIFEST_NAME:
        return True
    return path.name == MANIFEST_FILE_NAME and path.parent.name == MANIFEST_NAME


def load_meta(path: Path) -> dict:
    """Parse one .epr-meta (frontmatter block -> nested dict). {} on any failure (caller fails open).
    Size + flow-depth capped so cascade/merge never parse a poisoned manifest. For the precise
    REASON a manifest is unusable (for the resolver's advisory), use check_meta()."""
    path = manifest_path(path)
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
        meta = manifest_for_dir(here)
        if meta is not None:
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
import re
from collections import namedtuple

ENFORCEMENT_CLASSES = ("deny", "ask", "inject", "measure", "dispatch")
_SEVERITY = {"deny": 3, "ask": 2, "inject": 1, "measure": 0, "dispatch": 0}
Verdict = namedtuple("Verdict", ["cls", "reason", "rule_id"])

# A rule of an enforcing class must carry at least one of these actionable predicate keys, else it
# fires on nothing (a "deny everything here" that silently allows — the M2 footgun).
_ACTIONABLE_KEYS = ("require-frontmatter", "allowed-types", "route-to", "no-new-subdirs",
                    "require-sibling", "dedupe-of", "validator")
_KNOWN_RULE_KEYS = {"id", "class", "why", "when", "max-files", "measure",
                    "policy", "params"} | set(_ACTIONABLE_KEYS)

# Policy-binding rules (`policy: <id>@<version>`) carry ONLY placement + local variance; the
# registry policy owns class/predicates/measure-defaults/why. The version pin is REQUIRED —
# which version applies is a DECLARED dependency, never recency.
_POLICY_REF_RE = re.compile(r"^[a-z0-9][a-z0-9-]*@\d+$")
_BINDING_KEYS = {"id", "policy", "params", "when", "why"}


def validate_meta(cfg: dict) -> list[str]:
    """Hand-rolled, stdlib-only check against the schema contract. [] = valid.
    Also surfaces footguns: an enforcing rule with no actionable predicate, a rule carrying MORE
    THAN ONE actionable predicate (the evaluator fires on only the first — silent partial
    enforcement), duplicate rule ids within one manifest (a later rule silently overrides an
    earlier one on merge), and unknown rule keys (a typo'd key would otherwise be silently ignored
    by _eval_rule)."""
    errs: list[str] = []
    if not isinstance(cfg, dict):
        return ["`.epr-meta` is not a mapping"]
    if cfg.get("epr-meta-version") != 1:
        errs.append("missing/invalid `epr-meta-version` (must be 1)")
    if "covers" in cfg and cfg["covers"] not in ("subtree", "dir-only"):
        errs.append(f"`covers` value `{cfg['covers']}` not in ('subtree', 'dir-only') "
                    f"— a typo here would silently leave the subtree unclaimed")
    seen_ids: set[str] = set()
    for i, rule in enumerate(cfg.get("rules", []) or []):
        if not isinstance(rule, dict):
            errs.append(f"rules[{i}] is not a mapping"); continue
        rid = rule.get("id", "?")
        if "id" not in rule:
            errs.append(f"rules[{i}] missing `id`")
        elif rid in seen_ids:
            errs.append(f"rules[{i}] (`{rid}`) duplicate rule id — ids must be unique within a "
                        f"manifest (a later rule silently overrides an earlier one on merge)")
        else:
            seen_ids.add(rid)
        if "policy" in rule:
            # Policy BINDING: the registry policy owns semantics; the binding owns placement.
            ref = rule["policy"]
            if not isinstance(ref, str) or not _POLICY_REF_RE.match(ref):
                errs.append(f"rules[{i}] (`{rid}`) `policy` must be `<id>@<version>` — the version "
                            f"pin is a declared dependency, never recency — got `{ref}`")
            extra = sorted(set(rule) - _BINDING_KEYS)
            if extra:
                errs.append(f"rules[{i}] (`{rid}`) policy-binding must not redeclare {extra} — "
                            f"class/predicates/measure come from the registry policy; local "
                            f"variance goes in `params` / `when`")
            if "params" in rule and not isinstance(rule["params"], dict):
                errs.append(f"rules[{i}] (`{rid}`) `params` must be a mapping")
            continue
        cls = rule.get("class")
        if cls not in ENFORCEMENT_CLASSES:
            errs.append(f"rules[{i}] (`{rid}`) class `{cls}` not in {ENFORCEMENT_CLASSES}")
        unknown = set(rule) - _KNOWN_RULE_KEYS
        if unknown:
            errs.append(f"rules[{i}] (`{rid}`) unknown key(s) {sorted(unknown)} (typo?)")
        actionable = [k for k in _ACTIONABLE_KEYS if k in rule]
        if cls in ("deny", "ask", "inject") and not actionable:
            errs.append(f"rules[{i}] (`{rid}`) class `{cls}` has no actionable predicate "
                        f"(require-frontmatter/route-to/no-new-subdirs/…) — it fires on nothing")
        if len(actionable) > 1:
            errs.append(f"rules[{i}] (`{rid}`) carries multiple actionable predicates {actionable} "
                        f"— only one predicate per rule is evaluated (first-match wins); split into "
                        f"separate rules")
    for i, v in enumerate(cfg.get("validators", []) or []):
        if not isinstance(v, dict) or "ref" not in v:
            errs.append(f"validators[{i}] missing `ref`")
    return errs


def check_meta(path: Path) -> list[str]:
    """Health-check one .epr-meta FOR THE RESOLVER: [] = healthy. Reports the precise reason a
    manifest is unusable (size cap / too-deep / unparseable / empty / schema errors) so the hook can
    advise specifically. Distinct from load_meta(), which just returns {} on any failure for the
    cascade/merge fast-path."""
    path = manifest_path(path)
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
    return set(fm.parse(content).fields.keys())


def _eval_rule(rule: dict, write: dict) -> Verdict | None:
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

    if "measure" in rule:
        # Observation tier — never blocks. LoC ceilings: at/over loc-hard emits a `measure` verdict
        # (the resolver files it as a fingerprinted architecture finding + dispatch directive);
        # over loc-soft only emits an `inject` advisory (edit-time nudge; the resolver debounces).
        m = rule["measure"] if isinstance(rule["measure"], dict) else {}
        content = write.get("content")
        if content is None:
            return None  # content unresolved (unreadable disk / failing Edit) — measure abstains
        loc = content.count("\n") + (1 if content and not content.endswith("\n") else 0)
        hard, soft = m.get("loc-hard"), m.get("loc-soft")
        name = Path(write["path"]).name
        if isinstance(hard, int) and loc >= hard:
            return Verdict("measure", f"`{name}` is {loc} lines — at/over the {hard}-line HARD "
                                      f"LoC ceiling. {why}", rid)
        if isinstance(soft, int) and loc >= soft:
            return Verdict("inject", f"`{name}` is {loc} lines — over the {soft}-line soft LoC "
                                     f"ceiling (hard ceiling: {hard}). {why}", rid)
        return None

    if "max-files" in rule:
        return None
    return None


# Named reference validators (the escape hatch). v1 ships one, cloned from p2p-plan-audit's detector.
def _p2p_design_gate(write: dict) -> bool:
    c = (write.get("content") or "")
    return any(s in c for s in ("GET /api/v1", "PRIMARY KEY", "uuid"))


REFERENCE_VALIDATORS = {"epr:validator-p2p-design-gate": _p2p_design_gate}


# ── Policy registry: define-once-bind-many rules (the Mishpat::Precedent shape, dev-tooling tier).
# The registry YAML holds Precedent-shaped policy objects; a manifest rule binds one with
# `policy: <id>@<version>` (+ optional `params`/`when` local variance). Expansion happens at
# resolve time (after merge_rules, before evaluate) so `evaluate` stays pure and unchanged.
# Graduated home: mishpat_integrity::Precedent entries (CID = entry_hash), bindings become cites. ──
POLICY_REGISTRY_REL = ".claude/epr-meta/policies.yaml"


def load_policies(repo_root: Path) -> tuple[dict, list[str]]:
    """Load the policy registry → ({'id@version': policy}, errors). Missing registry is a
    legitimate state → ({}, []). Unreadable/invalid → ({}, [reason]) so bindings fail LOUD
    (dropped-with-advisory), never silent."""
    p = Path(repo_root) / POLICY_REGISTRY_REL
    if not p.is_file():
        return {}, []
    if yaml is None:
        return {}, [f"PyYAML unavailable — policy registry {POLICY_REGISTRY_REL} not loaded"]
    try:
        if p.stat().st_size > MAX_MANIFEST_BYTES:
            return {}, [f"{POLICY_REGISTRY_REL} exceeds {MAX_MANIFEST_BYTES // 1024}KB size cap"]
        text = p.read_text()
        if not _flow_depth_ok(text):
            return {}, [f"{POLICY_REGISTRY_REL} nesting too deep — refusing to parse"]
        data = yaml.safe_load(text) or {}
    except Exception as e:  # noqa: BLE001
        return {}, [f"{POLICY_REGISTRY_REL} unreadable/invalid: {e}"]
    if not isinstance(data, dict) or data.get("epr-meta-policies-version") != 1:
        return {}, [f"{POLICY_REGISTRY_REL} missing/invalid `epr-meta-policies-version` (must be 1)"]
    out: dict = {}
    errs: list[str] = []
    for i, pol in enumerate(data.get("policies", []) or []):
        if not isinstance(pol, dict) or not pol.get("id") or not isinstance(pol.get("version"), int):
            errs.append(f"policies[{i}] needs `id` + integer `version`")
            continue
        key = f"{pol['id']}@{pol['version']}"
        if key in out:
            errs.append(f"duplicate policy `{key}` in registry")
            continue
        cls = pol.get("class")
        if cls not in ENFORCEMENT_CLASSES:
            errs.append(f"policy `{key}` class `{cls}` not in {ENFORCEMENT_CLASSES}")
            continue
        # The M2 footgun applies to the registry too: an enforcing policy with no actionable
        # predicate would expand into a rule that fires on nothing — a silently-allowed deny.
        # An invalid policy is NOT loaded, so its bindings drop LOUD via expand_policies'
        # unknown-policy path (advisory reaches the model) instead of silently un-enforcing.
        actionable = [k for k in _ACTIONABLE_KEYS if k in pol]
        if cls in ("deny", "ask", "inject") and not actionable:
            errs.append(f"policy `{key}` class `{cls}` has no actionable predicate — it would "
                        f"expand into a rule that fires on nothing (silent allow); NOT loaded")
            continue
        if len(actionable) > 1:
            errs.append(f"policy `{key}` carries multiple actionable predicates {actionable} — "
                        f"only the first is evaluated; NOT loaded")
            continue
        if cls == "measure":
            m = pol.get("measure")
            bad = [k for k in ("loc-soft", "loc-hard")
                   if isinstance(m, dict) and k in m and not isinstance(m[k], int)]
            if not isinstance(m, dict) or bad or not any(
                    isinstance(m.get(k), int) for k in ("loc-soft", "loc-hard")):
                errs.append(f"policy `{key}` class `measure` needs a `measure:` block with "
                            f"integer loc-soft/loc-hard"
                            + (f" (non-integer: {bad})" if bad else "")
                            + " — it would measure nothing; NOT loaded")
                continue
        out[key] = pol
    return out, errs


def expand_policies(merged: dict, policies: dict) -> list[str]:
    """Resolve `policy:` bindings in merged rules to concrete rules, in place. The policy owns
    class / predicates / measure defaults / why; the binding owns `when` override + `params`
    (merged over the policy's `measure`). An unknown or unpinned ref DROPS the rule and reports —
    fail-loud-not-silent (a deny that silently vanishes is silent-allow). An inline rule whose id
    shadows a registry policy id gets a dedupe advisory. Idempotent: already-expanded rules
    (carrying `policy-ref`) pass through untouched."""
    errs: list[str] = []
    ids = {k.split("@", 1)[0] for k in policies}
    for rid, rule in list(merged["rules"].items()):
        ref = rule.get("policy")
        if not ref:
            if "policy-ref" not in rule and rid in ids:
                errs.append(f"rule `{rid}` is inlined but registry policy `{rid}` exists — bind "
                            f"`policy: {rid}@<version>` instead of redefining")
            continue
        if not isinstance(ref, str) or not _POLICY_REF_RE.match(ref):
            errs.append(f"rule `{rid}` policy ref `{ref}` must be `<id>@<version>` (explicit "
                        f"version pin) — rule NOT enforced")
            del merged["rules"][rid]
            continue
        pol = policies.get(ref)
        if pol is None:
            errs.append(f"rule `{rid}` binds unknown policy `{ref}` — rule NOT enforced "
                        f"(registry: {POLICY_REGISTRY_REL})")
            del merged["rules"][rid]
            continue
        exp = {"id": rid, "class": pol.get("class", "inject"),
               "why": rule.get("why") or pol.get("why", ""),
               "when": rule.get("when") or pol.get("scope") or {},
               "policy-ref": ref}
        for k in _ACTIONABLE_KEYS:
            if k in pol:
                exp[k] = pol[k]
        m = dict(pol.get("measure") or {})
        m.update(rule.get("params") or {})
        if m:
            exp["measure"] = m
            # A binding's params can poison a loaded-clean policy (e.g. loc-hard: "9000") —
            # the ceiling would silently stop firing. Advise loud; keep the rule (other keys
            # may still be live).
            bad = [k for k in ("loc-soft", "loc-hard")
                   if k in m and not isinstance(m[k], int)]
            if exp.get("class") == "measure" and bad:
                errs.append(f"rule `{rid}` merged measure key(s) {bad} are not integers — "
                            f"those ceilings are inert; fix the binding's `params`")
        merged["rules"][rid] = exp
    return errs


def evaluate(merged: dict, write: dict) -> list[Verdict]:
    """PURE: read rules + write, return fired verdicts. No writes, no side-effects."""
    out: list[Verdict] = []
    for rule in merged.get("rules", {}).values():
        v = _eval_rule(rule, write)
        if v is not None:
            out.append(v)
    return out


def combine(verdicts: list[Verdict]) -> Verdict | None:
    blocking = [v for v in verdicts if v.cls in ("deny", "ask", "inject")]
    if not blocking:
        return None
    return max(blocking, key=lambda v: _SEVERITY[v.cls])


# ── Subtree-coverage walk: the `.epr-meta` self-responsibility claim + the deterministic coverage
# signal that placement-audit.py reads as a stasis dimension (the downward dual of claude-md-audit's
# missing-CLAUDE.md census). An `.epr-meta` that declares `covers: subtree` is FULLY RESPONSIBLE for
# everything beneath it — the walk terminates there (integrity by construction; a claimed subtree's
# internals are never re-audited, exactly as the core never re-validates an app-manifest's vocabulary).
# A structurally-substantial directory reached with no such covering ancestor is an unclaimed GAP. ──
DEFAULT_COMPLEXITY_EXTS = {
    ".ts", ".tsx", ".js", ".jsx", ".mjs", ".py", ".rs", ".go", ".java", ".rb",
    ".html", ".scss", ".css", ".vue", ".svelte", ".feature", ".sql", ".graphql", ".proto",
}
# Directories the walk never descends into (build output, vendored, VCS, agent worktrees). The
# `.claude/worktrees/wf_*` trees carry shadow `.epr-meta` files that must NOT count as governance.
DEFAULT_SKIP_DIRS = {
    ".git", "node_modules", "target", "dist", "build", ".angular", ".pnpm-store", "__pycache__",
    ".cargo", ".venv", "venv", ".next", "coverage", "worktrees", ".worktrees", ".pytest_cache",
}


def claims_subtree(cfg: dict) -> bool:
    """True iff this manifest declares full responsibility for its subtree (`covers: subtree`).
    The self-contained coverage-walk terminator — the downward dual of `root: true` (which stops the
    upward cascade). Opt-in by design: an incidental manifest (a `ci-trigger` config, one local rule)
    does NOT claim its subtree, so the repo-root manifest never trivially 'covers' the whole tree."""
    return isinstance(cfg, dict) and cfg.get("covers") == "subtree"


def _is_substantial(n_files: int, n_subdirs: int, n_exts: int,
                    min_files: int, min_subdirs: int, min_exts: int) -> bool:
    """A directory worth a governance decision: enough direct files OR subdirs to be architecturally
    real, AND enough distinct complexity-extensions that it isn't pure data/assets. Mirrors
    claude-md-audit's MISSING_TUNABLES substantiality predicate."""
    return (n_files >= min_files or n_subdirs >= min_subdirs) and n_exts >= min_exts


def subtree_coverage(root: Path, *, min_files: int = 15, min_subdirs: int = 4, min_exts: int = 1,
                     complexity_exts=DEFAULT_COMPLEXITY_EXTS, skip_dirs=DEFAULT_SKIP_DIRS,
                     exclude_globs=()) -> dict:
    """Walk the file graph under `root`; classify each region as COVERED (claimed by a *valid*
    `covers: subtree` manifest) or a GAP (structurally-substantial, no covering ancestor). Both
    claim-points and gap-roots TERMINATE descent — one manifest at the right altitude resolves a whole
    subtree, so nested substantial dirs are never double-counted. Deliberate v1 simplification: a
    `covers` claim placed *below* a gap-root is NOT reached, so the gap-root counts AGAINST the ratio
    until a claim is declared at/above it (ownership is altitude-first, not leaf-first). The repo root is
    never itself a gap (mirrors claude-md-audit). `min_exts` defaults to 1 (NOT claude-md's 2): a
    single-language code dir is a prime governance target here. Returns {covered, gaps, covered_count,
    gap_count, ratio}; ratio = covered / (covered + gaps), and 1.0 when there is nothing governable."""
    root = root.resolve()
    covered: list[str] = []
    gaps: list[dict] = []

    def rel(d: Path) -> str:
        return d.relative_to(root).as_posix()

    def excluded(d: Path) -> bool:
        r = rel(d)
        return any(fnmatch.fnmatch(r, g) or fnmatch.fnmatch(r + "/", g) for g in exclude_globs)

    def visit(d: Path, is_root: bool):
        meta = manifest_for_dir(d)
        if meta is not None:
            cfg = load_meta(meta)
            # only a VALID claim owns the subtree — a schema-invalid manifest the resolver would reject
            # must NOT be credited as coverage (else the census and the enforcing resolver disagree, and a
            # broken/unenforced manifest inflates the ratio + hides a real gap).
            if claims_subtree(cfg) and not validate_meta(cfg):
                covered.append(rel(d))
                return  # fully responsible — terminate
        try:
            entries = list(d.iterdir())
        except OSError:
            return
        # not c.is_symlink(): never follow a symlinked dir — it would double-count a target reached two
        # ways and could pull a dir OUTSIDE root into the census under an in-root relpath.
        subdirs = [c for c in entries if c.is_dir() and not c.is_symlink()
                   and c.name not in skip_dirs and not excluded(c)]
        if not is_root:  # the repo root is never itself a gap
            files = [c for c in entries if c.is_file() and c.name != MANIFEST_NAME]
            exts = {c.suffix for c in files if c.suffix in complexity_exts}
            if _is_substantial(len(files), len(subdirs), len(exts), min_files, min_subdirs, min_exts):
                gaps.append({"path": rel(d), "files": len(files),
                             "subdirs": len(subdirs), "exts": sorted(exts)})
                return  # gap-root — a claim at/above here resolves it; don't descend
        for c in sorted(subdirs):
            visit(c, False)

    visit(root, True)
    denom = len(covered) + len(gaps)
    return {"covered": covered, "gaps": gaps, "covered_count": len(covered),
            "gap_count": len(gaps), "ratio": 1.0 if denom == 0 else len(covered) / denom}


def _dir_is_substantial(d: Path, min_files: int, min_subdirs: int, min_exts: int,
                        complexity_exts, skip_dirs) -> bool:
    """Substantiality of ONE directory (the per-edit ascending check's unit; same predicate the
    descending walk applies to a candidate region)."""
    try:
        entries = list(d.iterdir())
    except OSError:
        return False
    subdirs = [c for c in entries if c.is_dir() and not c.is_symlink() and c.name not in skip_dirs]
    files = [c for c in entries if c.is_file() and c.name != MANIFEST_NAME]
    exts = {c.suffix for c in files if c.suffix in complexity_exts}
    return _is_substantial(len(files), len(subdirs), len(exts), min_files, min_subdirs, min_exts)


def coverage_advice(target: Path, *, repo_root: Path = None, min_files: int = 15, min_subdirs: int = 4,
                    min_exts: int = 1, complexity_exts=DEFAULT_COMPLEXITY_EXTS,
                    skip_dirs=DEFAULT_SKIP_DIRS, exclude_globs=()) -> dict | None:
    """The ASCENDING dual of `subtree_coverage`, for the in-flight signal: given a single edited
    `target`, is it inside an UNCLAIMED substantial region? Walks ancestors from the target's dir up
    to the repo root (bounded). Returns None if any valid `covers: subtree` ancestor already owns it
    (claimed) OR if no substantial ancestor exists (nothing to govern — no nag). Otherwise returns
    {gap_root, covered: False} where gap_root is the SHALLOWEST substantial ancestor — the same
    altitude the `--epr-meta` census reports, so the in-flight nudge and the queue agree on WHERE to
    author the manifest. The repo root itself is never a gap-root."""
    target = target.resolve()
    start = target.parent if (target.is_file() or not target.exists()) else target
    root = (repo_root or find_repo_root(start)).resolve()

    def excluded(d: Path) -> bool:
        try:
            r = d.relative_to(root).as_posix()
        except ValueError:
            return True
        return any(fnmatch.fnmatch(r, g) or fnmatch.fnmatch(r + "/", g) for g in exclude_globs)

    chain: list[Path] = []
    here, depth = start, 0
    while depth < MAX_CASCADE_DEPTH:
        chain.append(here)
        meta = manifest_for_dir(here)
        if meta is not None:
            cfg = load_meta(meta)
            if claims_subtree(cfg) and not validate_meta(cfg):
                return None  # already inside a valid claimed subtree
        if here == root or here.parent == here:
            break
        here = here.parent
        depth += 1

    for d in reversed(chain):  # root-first → first substantial is the shallowest (the gap-root)
        if d == root or d.name in skip_dirs or excluded(d):
            continue
        if _dir_is_substantial(d, min_files, min_subdirs, min_exts, complexity_exts, skip_dirs):
            return {"gap_root": d.relative_to(root).as_posix(), "covered": False}
    return None


def governance_cfg(repo_root: Path) -> dict:
    """The SINGLE config source for epr-meta coverage — coverage tunables + the exclusion globs
    (git submodules, auto-discovered, PLUS the yaml `epr_meta_governance.exclude` list). Shared by the
    descending census (`placement-audit.py --epr-meta`) and the ascending in-flight nudge (the resolver
    hook) so the two can never disagree about which dirs are governable. Fail-open to code defaults if
    `.gitmodules` / PyYAML / the yaml block is absent. Returns {min_files, min_subdirs, min_exts,
    exclude_globs}."""
    repo_root = Path(repo_root)
    cfg = {"min_files": 15, "min_subdirs": 4, "min_exts": 1, "exclude_globs": []}
    ex: list[str] = []
    gm = repo_root / ".gitmodules"
    if gm.is_file():
        try:
            for ln in gm.read_text().splitlines():
                ln = ln.strip()
                if ln.startswith("path") and "=" in ln:
                    p = ln.split("=", 1)[1].strip()
                    if p:
                        ex += [p, p + "/**"]
        except OSError:
            pass
    if yaml is not None:
        try:
            data = yaml.safe_load((repo_root / ".claude/memory-kit/context-coverage.yaml").read_text()) or {}
            blk = data.get("epr_meta_governance", {}) or {}
            for k in ("min_files", "min_subdirs", "min_exts"):
                if isinstance(blk.get(k), int):
                    cfg[k] = blk[k]
            if isinstance(blk.get("exclude"), list):
                ex += [str(x) for x in blk["exclude"]]
        except Exception:  # noqa: BLE001
            pass
    cfg["exclude_globs"] = ex
    return cfg


def measure_census(repo_root: Path, *, skip_dirs=DEFAULT_SKIP_DIRS, exclude_globs=()) -> dict:
    """Descending census for measure rules — the batch dual of the resolver's per-edit measure
    verdict: every on-disk file governed by an (expanded) measure rule with a LoC ceiling, counted
    against the same ceilings the edit-time gate applies. Returns {"hard": rows, "soft": rows,
    "errors": [...]} — hard = at/over loc-hard (the architecture-review queue), soft = over
    loc-soft only. Rules inherit downward nearest-wins (mirrors collect_cascade/merge_rules);
    a directory with its own `.git` is a submodule boundary — its governance is its own.

    Two known divergences from the per-edit path, both benign for glob-scoped source policies:
    the census honors `exclude_globs` while the per-edit cascade does not (an excluded-dir edit
    still measures; the census stays quiet there), and the census inherits rules DOWN past an
    intermediate `root: true` manifest that the per-edit UPWARD cascade would stop at. If a
    measure policy ever targets files under such a subtree, reconcile these first."""
    root = Path(repo_root).resolve()
    policies, errors = load_policies(root)
    hard: list[dict] = []
    soft: list[dict] = []

    def rules_at(d: Path, inherited: dict) -> dict:
        meta = d / MANIFEST_NAME
        if not meta.is_file():
            return inherited
        merged = dict(inherited)
        for rule in load_meta(meta).get("rules", []) or []:
            if rule.get("id"):
                merged[rule["id"]] = rule
        wrap = {"rules": merged, "validators": {}, "sources": []}
        errors.extend(expand_policies(wrap, policies))  # idempotent on inherited expanded rules
        return wrap["rules"]

    def visit(d: Path, inherited: dict, is_root: bool):
        if not is_root and (d / ".git").exists():
            return  # submodule boundary
        rel_d = d.relative_to(root).as_posix()
        if not is_root and any(fnmatch.fnmatch(rel_d, g) or fnmatch.fnmatch(rel_d + "/", g)
                               for g in exclude_globs):
            return
        rules = rules_at(d, inherited)
        mrules = [r for r in rules.values() if isinstance(r.get("measure"), dict)
                  and ("loc-hard" in r["measure"] or "loc-soft" in r["measure"])]
        try:
            entries = list(d.iterdir())
        except OSError:
            return
        if mrules:
            for f in (c for c in entries if c.is_file() and not c.is_symlink()):
                for r in mrules:
                    pat = (r.get("when") or {}).get("write")
                    if not pat or not fnmatch.fnmatch(f.name.lower(), pat.lower()):
                        continue
                    try:
                        data = f.read_bytes()
                    except OSError:
                        continue
                    loc = data.count(b"\n") + (1 if data and not data.endswith(b"\n") else 0)
                    m = r["measure"]
                    h, s = m.get("loc-hard"), m.get("loc-soft")
                    row = {"path": f.relative_to(root).as_posix(), "loc": loc,
                           "rule": r.get("id"), "soft": s, "hard": h}
                    if isinstance(h, int) and loc >= h:
                        hard.append(row)
                    elif isinstance(s, int) and loc >= s:
                        soft.append(row)
                    break  # first matching measure rule wins (single-predicate discipline)
        for c in sorted(c for c in entries
                        if c.is_dir() and not c.is_symlink() and c.name not in skip_dirs):
            visit(c, rules, False)

    visit(root, {}, True)
    hard.sort(key=lambda r: -r["loc"])
    soft.sort(key=lambda r: -r["loc"])
    return {"hard": hard, "soft": soft, "errors": errors}
