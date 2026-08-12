"""The .epr-meta cascade + merge — the reuse of subject_routing's root-first/nearest-wins
ancestor walk, retargeted to the `.epr-meta` manifest with nested-YAML parsing."""
from __future__ import annotations
import hashlib
import json
import time
from pathlib import Path

from _lib import frontmatter as fm

try:
    import yaml  # PyYAML — needed for the nested rule config
except Exception:  # pragma: no cover
    yaml = None

try:
    import fcntl
except ImportError:  # pragma: no cover — non-POSIX
    fcntl = None

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
# `refer_reason` is the ceiling-law vocabulary tag (constraint 3): rule-fired (default) |
# unresolvable-validator | governance-manifest-malformed | policy-pin-mismatch |
# escalation-requires-ratification. Defaulted so every existing 3-arg Verdict(...) call site
# (this module + _lib/epr_meta_git.py) keeps working unchanged.
# `evidence` (algedonic phase-1 Task 4) is a small optional dict of STRUCTURED numbers a verdict
# carries alongside its prose `reason` — today just the measure class's {"stock": <measured LoC>,
# "limit": <ceiling>} — so a consumer (the resolver's architecture-finding mint) threads real
# values instead of regex-parsing the reason string. Additive + defaulted: every existing 2-4-arg
# Verdict(...) call site keeps working unchanged, and it is NEVER an input to any fingerprint.
Verdict = namedtuple("Verdict", ["cls", "reason", "rule_id", "refer_reason", "evidence"],
                      defaults=[None, None])

# A rule of an enforcing class must carry at least one of these actionable predicate keys, else it
# fires on nothing (a "deny everything here" that silently allows — the M2 footgun).
_ACTIONABLE_KEYS = ("require-frontmatter", "allowed-types", "route-to", "no-new-subdirs",
                    "require-sibling", "dedupe-of", "validator")
# `parameters` is dispatch-only config (dispatch-agent/dispatch-prompt) — NOT counted as a
# competing actionable predicate (a dispatch rule may legitimately pair it with a real predicate
# like require-frontmatter, exactly as a measure rule does; `parameters` alone is also a valid
# fallback trigger — see _eval_rule's final branch). Recognized so it never trips the unknown-key
# check, but excluded from _ACTIONABLE_KEYS so it never trips the >1-actionable-predicate check.
_KNOWN_RULE_KEYS = {"id", "class", "why", "when", "max-files", "measure",
                    "policy", "params", "parameters", "retire-when"} | set(_ACTIONABLE_KEYS)

# Policy-binding rules (`policy: <id>@<version>`) carry ONLY placement + local variance; the
# registry policy owns class/predicates/measure-defaults/why. The version pin is REQUIRED —
# which version applies is a DECLARED dependency, never recency.
_POLICY_REF_RE = re.compile(r"^[a-z0-9][a-z0-9-]*@\d+$")
_BINDING_KEYS = {"id", "policy", "params", "when", "why"}

# Measure ontology (elohim/epr/src/measure.rs MeasureKind, `#[serde(tag = "kind",
# rename_all = "lowercase")]`): a `class: measure` rule's `kind` lives INSIDE its `measure:`
# block, never at the rule top level — `measure:` is the policy-owned semantics block (merged
# through `expand_policies`'s `measure`/`params` merge step below) and a top-level `kind:` would
# be undeclarable by a policy-binding rule (`_BINDING_KEYS` above admits only
# id/policy/params/when/why). Nested, it is inherited through the existing merge like every other
# measure default, with `params` for local variance. A `rate` cannot forget its denominator — it
# must carry `per:`.
MEASURE_KIND_VOCAB = {"level", "rate", "ratio"}

# The intervenor's removal condition (Meadows, *Thinking in Systems* ch.5, "Shifting the Burden
# to the Intervenor"): *"If you are the intervenor, work in such a way as to restore or enhance
# the system's own ability to solve its problems, THEN REMOVE YOURSELF."*
#
# Every rule in every `.epr-meta` is an intervenor — it exists because the system did not
# reliably do something on its own. A rule with no stated removal condition can only accrete:
# there is no state of the world that retires it, so the governance layer grows monotonically
# and the trap closes. `retire-when:` is the exit, and it is deliberately a CONDITION, never a
# date — "when the habit holds" is checkable; "in six months" is a wish.
#
# `never` is a legitimate answer and the whole point of admitting it is that it becomes
# COUNTABLE rather than silent. A constitutional floor genuinely never retires. But a bare
# `never` is indistinguishable from not having thought about it, so the honest form carries its
# reason: `never: <why this is a floor>`. That is the same discipline `status: unwired` applies
# in habits.yaml — make the uncomfortable state declarable and counted instead of papered over.
_RETIRE_NEVER_RE = re.compile(r"^never\b", re.IGNORECASE)


def _validate_retire_when(value, label: str) -> list[str]:
    """Shared check for `retire-when:` on both the manifest-rule path and the registry-policy
    path — one vocabulary, two gates, exactly as MEASURE_KIND_VOCAB is shared. [] = valid.

    Note what is NOT checked: whether the condition is TRUE, or ever becomes true. That is the
    census's job (and ultimately a human's). This gate only refuses the two shapes that carry
    no information — an empty condition, and a bare `never` with no reason."""
    if not isinstance(value, str) or not value.strip():
        return [f"{label}: `retire-when` must be a non-empty condition string — "
                f"the state of the world under which this intervenor is REMOVED"]
    text = value.strip()
    if _RETIRE_NEVER_RE.match(text) and len(text.split()) < 2:
        return [f"{label}: a bare `retire-when: never` is indistinguishable from not having "
                f"asked — an honest never carries its reason (`never: <why this is a floor>`)"]
    return []


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
        if cls == "measure":
            m = rule.get("measure") if isinstance(rule.get("measure"), dict) else {}
            kind = m.get("kind")
            if kind is None:
                errs.append(
                    f"rules[{i}] (`{rid}`): a measure rule must declare kind: "
                    f"level|rate|ratio (L6) — an undeclared kind is how a rate gets "
                    f"compared to a cumulative level")
            elif kind not in MEASURE_KIND_VOCAB:
                errs.append(f"rules[{i}] (`{rid}`): unknown kind {kind!r}; expected one of "
                            f"{sorted(MEASURE_KIND_VOCAB)}")
            elif kind == "rate" and not m.get("per"):
                errs.append(f"rules[{i}] (`{rid}`): kind: rate requires per: "
                            f"(second|minute|hour|day|week|month|year)")
        if "retire-when" in rule:
            errs.extend(_validate_retire_when(rule["retire-when"], f"rules[{i}] (`{rid}`)"))
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

    if rule.get("pin-mismatch"):
        # Synthesized by expand_policies when a bound policy's contentHash no longer matches its
        # registry row — fires unconditionally once `when` matches (the binding itself is suspect).
        return Verdict(cls, why, rid, rule.get("refer-reason"))

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
        if ref in RUNTIME_SCOPED_VALIDATORS:
            # Declared runtime-scoped (e.g. rust-only eprfs-meta validators) — Unavailable-by-
            # declaration, NOT unresolvable: skip clean, never downgrade the rule.
            return None
        if ref not in REFERENCE_VALIDATORS:
            # An unresolvable reference must never SOFTEN a rule that was declared to gate
            # (the pre-slice soundness inversion): a `class: ask|deny` rule whose validator
            # cannot be resolved ROUTES to review rather than guessing.
            #
            # It must not HARDEN an advisory one either. A `class: inject` rule is declared
            # non-blocking by its author — escalating it to `ask` stops the agent and waits
            # on a human for a rule whose own text says it never blocks (the `p2p/**` rules:
            # peer-fallback-invariant, heal-fills-never-moves, dataplane-guide-star,
            # interface-first-reuse-*, all `class: inject`, every one of them gating every
            # edit to elohim-storage). That is the same soundness inversion pointing the
            # other way, and it costs agency instead of safety. The advisory text still
            # reaches the agent — it warns without blocking, which is the rule's whole intent.
            #
            # Clamp to the DECLARED class: route only what was already meant to gate.
            if _SEVERITY.get(cls, 0) >= _SEVERITY["ask"]:
                return Verdict("ask", f"validator `{ref}` not registered — unresolvable "
                                      f"reference routes to review rather than guessing. {why}",
                               rid, "unresolvable-validator")
            return Verdict(cls, f"validator `{ref}` not registered — advisory only, not "
                                f"evaluated. {why}", rid, "unresolvable-validator")
        if REFERENCE_VALIDATORS[ref](write):
            return Verdict(cls, f"validator `{ref}` flagged this write. {why}", rid,
                           VALIDATOR_REFER_REASONS.get(ref))
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
                                      f"LoC ceiling. {why}", rid,
                            evidence={"stock": loc, "limit": hard})
        if isinstance(soft, int) and loc >= soft:
            return Verdict("inject", f"`{name}` is {loc} lines — over the {soft}-line soft LoC "
                                     f"ceiling (hard ceiling: {hard}). {why}", rid)
        return None

    if "max-files" in rule:
        return None

    if cls == "dispatch" and "parameters" in rule:
        # Fallback trigger: a dispatch rule with NO other actionable predicate fires
        # unconditionally once `when` matches (exactly like `dedupe-of`) — the sentinel-style
        # directive text is assembled by the caller (resolver.py) from rule.parameters
        # (dispatch-agent / dispatch-prompt), never blocks. Only reached when none of the real
        # actionable predicates above matched, so a dispatch rule that ALSO carries e.g.
        # require-frontmatter is gated by that predicate first — `parameters` alone never competes
        # with it for the single-predicate-discipline check (validate_meta excludes it).
        return Verdict(cls, f"dispatch rule fired. {why}", rid)
    return None


# Named reference validators (the escape hatch). v1 ships one, cloned from p2p-plan-audit's detector.
def _p2p_design_gate(write: dict) -> bool:
    c = (write.get("content") or "")
    return any(s in c for s in ("GET /api/v1", "PRIMARY KEY", "uuid"))


# ── Brand vocabulary stays at the brand boundary (`epr:validator-brand-vocabulary-boundary`).
# Domain/project names are useful in architecture and product prose, but an unfamiliar engineer
# should not need that glossary to understand a route, type, function, table, or wire value. This
# advisory therefore scans only source/config artifacts, only NET-NEW lines, and ignores comments +
# schema prose. There is deliberately no compatibility suppression while the protocol is in
# development: internal role/zome/package/wire literals should be renamed too. This is `inject`,
# never a blocker.
_BRAND_CODE_SUFFIXES = frozenset({
    ".bash", ".c", ".cc", ".cjs", ".cpp", ".cs", ".css", ".go", ".gql", ".graphql",
    ".groovy", ".h", ".hpp", ".html", ".java", ".js", ".json", ".jsonc", ".jsx", ".kt",
    ".kts", ".mjs", ".proto", ".py", ".rs", ".scss", ".sh", ".sql", ".svelte", ".swift",
    ".toml", ".ts", ".tsx", ".vue", ".xml", ".yaml", ".yml",
})
_BRAND_CODE_BASENAMES = frozenset({"dockerfile", "jenkinsfile", "justfile", "makefile"})
_BRAND_LINT_INTERNAL_SUFFIXES = (
    "/.claude/scripts/_lib/epr_meta.py",
    "/.claude/scripts/_lib/__tests__/brand_vocabulary_guard_test.py",
    "/elohim/eprfs/epr-cli/src/repository_validators.rs",
)
_BRAND_VOCABULARY = {
    "imagodei": "identity, presence, or stewardship-of-self",
    "lamad": "learning, teaching, content, path, assessment, or mastery",
    "avodah": "work, service, contribution, project, story, or flow",
    "qahal": "community, collective, assembly, consent, or social governance",
    "shefa": "economy, value flow, mutual credit, resource, or stewardship",
    "mishpat": "governance, judgment, decision, policy, commitment, authority, or recovery quorum",
}
_BRAND_TERM_RES = {
    term: re.compile(rf"(?<![A-Za-z0-9]){re.escape(term)}", re.IGNORECASE)
    for term in _BRAND_VOCABULARY
}
_BRAND_PROSE_FIELD_RE = re.compile(
    r"^\s*[\"']?(?:description|\$comment|why|purpose)[\"']?\s*[:=]",
    re.IGNORECASE,
)
_BRAND_PACKAGE_BODY_RE = re.compile(r"^\s*[\"']?body[\"']?\s*[:=]", re.IGNORECASE)
_BRAND_BLOCK_PROSE_RE = re.compile(
    r"^(\s*)[\"']?(?:description|\$comment|why|purpose)[\"']?\s*[:=]\s*[>|]",
    re.IGNORECASE,
)


def _brand_comment_only(line: str, suffix: str, basename: str) -> bool:
    stripped = line.lstrip()
    if stripped.startswith(("//", "/*", "*", "*/", "<!--", "-->")):
        return True
    if suffix == ".sql" and stripped.startswith("--"):
        return True
    if suffix in (".py", ".sh", ".bash", ".yaml", ".yml", ".toml") or basename in (
            "dockerfile", "makefile"):
        return stripped.startswith("#")
    return False


def _brand_prose_lines(lines: list[str], suffix: str, package_body_is_prose: bool = False) -> set[int]:
    """Line indexes that are documentation inside a code/config artifact. JSON Schema
    `description`/`$comment` values and YAML/TOML folded prose are not compilable references."""
    out: set[int] = set()
    block_indent: int | None = None
    for i, line in enumerate(lines):
        stripped = line.strip()
        indent = len(line) - len(line.lstrip())
        if block_indent is not None:
            if stripped and indent <= block_indent:
                block_indent = None
            else:
                out.add(i)
                continue
        if _BRAND_PROSE_FIELD_RE.match(line) or (
                package_body_is_prose and _BRAND_PACKAGE_BODY_RE.match(line)):
            out.add(i)
        block = _BRAND_BLOCK_PROSE_RE.match(line)
        if block and suffix in (".yaml", ".yml", ".toml"):
            block_indent = len(block.group(1))
    return out


def _brand_vocabulary_boundary(write: dict) -> bool:
    import sys as _sys

    path = Path(write.get("path") or "")
    normalized_path = path.as_posix().lower()
    rooted_path = f"/{normalized_path.lstrip('/')}"  # repo-relative and absolute paths compare alike
    if any(rooted_path.endswith(suffix) for suffix in _BRAND_LINT_INTERNAL_SUFFIXES):
        return False  # the validator and its fixtures necessarily enumerate the vocabulary
    if "/.claude/memory-kit/" in rooted_path:
        return False  # generated observation ledgers are neither code nor authored configuration
    suffix = path.suffix.lower()
    basename = path.name.lower()
    if suffix not in _BRAND_CODE_SUFFIXES and basename not in _BRAND_CODE_BASENAMES:
        return False  # docs, feature narratives, images, and other non-code artifacts stay brand-safe
    post = write.get("content")
    if post is None:
        return False
    pre = _prior_text(write)
    added = set(_added_lines(pre if pre is not None else "", post))
    if not added:
        return False

    lines = post.splitlines()
    package_body_is_prose = (
        suffix == ".json" and "/.epr-meta/elohim/packages/" in rooted_path
    )
    prose_lines = _brand_prose_lines(lines, suffix, package_body_is_prose)
    hits: list[tuple[int, str, str]] = []
    for i, line in enumerate(lines):
        if line not in added or i in prose_lines or _brand_comment_only(line, suffix, basename):
            continue
        for term, pattern in _BRAND_TERM_RES.items():
            if pattern.search(line):
                hits.append((i + 1, term, line.strip()[:120]))

    if not hits:
        return False
    print(f"  brand-vocabulary-boundary — net-new brand vocabulary in compilable artifact "
          f"{_repo_rel(str(path))}:", file=_sys.stderr)
    for lineno, term, source in hits:
        print(f"    · line {lineno} [{term}] {source}", file=_sys.stderr)
        print(f"      prefer: {_BRAND_VOCABULARY[term]}", file=_sys.stderr)
    print("    FIX: name the capability in symbols, routes, schemas, tables, and wire values. "
          "Keep brand vocabulary in product/architecture prose. Internal role names, zome names, "
          "package ids, discriminators, and wire literals are not compatibility boundaries during "
          "development—rename them too. This advisory never blocks the write.", file=_sys.stderr)
    return True


# ── Sovereignty ontology guard (a validator-EPR). Fires an `ask` when a write NEWLY INTRODUCES
# apex-assertion sovereignty framing — "self-sovereign" as the top identity/agency tier, "true/full
# data sovereignty" celebrated as a goal — the crypto ontology the protocol rejects. It is a GUARD,
# not a ban (`ask`, one-keystroke override); the `why:` teaches the fix and names the three LEGITIMATE
# frames it deliberately does NOT fire on. Canon: genesis/docs/architecture/stewardship-over-sovereignty.md
# ("we do not consider sovereignty itself to be the right framing") + values-forward.md Stance II.4
# ("sovereignty is not forbidden — it is made mechanically expensive").
#   Legitimate (guard stays silent): (1) ADVERSARY frame — what's resisted / external actors
#   ("nation-states near-sovereign yet subject to the protocol"); (2) BOUNDED frame — "sovereign within
#   higher bounds", the collective human floor sovereign over the machine; (3) BRIDGE-LEGIBILITY frame —
#   a self-sovereign wallet making a participant legible to EXTERNAL sovereigns (a credit union/mutual),
#   authority/head/ceiling still flowing from the commons governance pool.
#   Drift (guard fires): self-sovereign as the protocol's OWN apex identity value; "true data
#   sovereignty" as an implementation goal.
# The guard fires only on NET-NEW apex framing (post-edit count > pre-edit count), so editing a file to
# CLEAN sovereignty — or any maintenance edit to a surface already carrying it — is never trapped.
_SOV_APEX_PHRASES = (
    "self-sovereign", "self sovereign", "self-sovereignty", "self sovereignty",
    "true data sovereignty", "full data sovereignty", "sovereign identity",
    "digital sovereignty", "fully sovereign",
)
# A co-located frame declaration suppresses the guard where the frame is already adjudicated (the wallet
# bridge crate declares this once, in-content or via a nearest-wins `.epr-meta` override on the rule id).
_SOV_FRAME_MARKER = "sovereignty-frame:"


def _sov_apex_count(text: str) -> int:
    t = text.lower()
    return sum(t.count(p) for p in _SOV_APEX_PHRASES)


# ── Ownership ontology guard. The sibling drift to sovereignty, and the same shape: OWNERSHIP is
# the enclosure-flavoured apex the protocol subordinates to STEWARDSHIP/CUSTODY. ValueFlows already
# splits the two — `primaryAccountable` (who holds the rights) vs `custodianScope` (who physically
# stewards), with distinct `transferCustody` / `transferAllRights` actions — and collapsing them is
# precisely the enclosure failure the Georgist common-inheritance framing exists to prevent: a holon
# that "owns" the commons it stewards has enclosed it. Phrases are narrow on purpose (the bare words
# "own"/"owner" are far too common to gate); these are the ones that assert ownership as the RIGHT
# relationship to a resource. Escape hatch mirrors sovereignty's: declare `stewardship-frame:`.
# PRECISION NOTE (audit 2026-08-05): the discriminator is PROPERTY vs RESPONSIBILITY. English uses
# "ownership" for both — "take full ownership of this bug", "the full ownership matrix" (already in
# 5 files here: timeline/CONVENTIONS.md + the deliver skill and its 3 projections), "without sole
# ownership burden". Those are accountability idioms, not enclosure claims, so `full ownership` and
# `sole ownership` were REMOVED after a corpus grep proved the collision. What remains is
# property-flavoured only.
_OWN_APEX_PHRASES = (
    "data ownership", "own your data", "owns their data", "owns your data",
    "true ownership", "outright ownership",
    "ownership rights", "ownership of the commons", "owns the commons",
)
_OWN_FRAME_MARKER = "stewardship-frame:"


def _own_apex_count(text: str) -> int:
    t = text.lower()
    return sum(t.count(p) for p in _OWN_APEX_PHRASES)


def _sovereignty_ontology_guard(write: dict) -> bool:
    post = write.get("content") or ""
    if _SOV_FRAME_MARKER in post.lower():
        return False  # frame explicitly declared/adjudicated for this surface
    post_n = _sov_apex_count(post)
    if post_n == 0:
        return False
    if write.get("is_new"):
        return True  # brand-new file: all apex framing is net-new
    try:
        pre = Path(write["path"]).read_text(errors="replace")
    except OSError:
        return True  # can't read prior state — fail toward surfacing (it's only an `ask`)
    return post_n > _sov_apex_count(pre)  # net-new only; cleaning/maintenance never fires


def _ownership_ontology_guard(write: dict) -> bool:
    """Fires on NET-NEW apex-ownership framing. Same contract as the sovereignty guard: an
    affirmation prompt, never a ban — confirm the frame is legitimate (adversary / bounded /
    external-legibility) or reframe toward custody+stewardship."""
    post = write.get("content") or ""
    low = post.lower()
    if _OWN_FRAME_MARKER in low or _SOV_FRAME_MARKER in low:
        return False  # frame explicitly declared/adjudicated for this surface
    post_n = _own_apex_count(post)
    if post_n == 0:
        return False
    if write.get("is_new"):
        return True
    try:
        pre = Path(write["path"]).read_text(errors="replace")
    except OSError:
        return True  # can't read prior state — fail toward surfacing (it's only an `ask`)
    return post_n > _own_apex_count(pre)  # net-new only


# ── Archetype resource alignment (a validator-EPR). Fires an `ask` when deployments.json carries
# INTRA-ARCHETYPE resource drift: a consolidated human whose edgenode* budget != its
# deviceArchetype's CANONICAL value in archetype-resource-budgets.json, WITHOUT an explicit
# `resourceOverride: {<field>, justification}`. Inverts control: the archetype budget is the
# SOURCE, a human derives it, and a role-justified exception (e.g. adam, the genesis anchor)
# declares an override ONCE. Catches the "bump the one crashlooping instance, leave the rest of
# the class behind" drift that the floor-only >= check cannot see. Details -> stderr on fire.
_ARCH_FIELDS = (
    ("edgenodeCpuRequest", "cpuRequest"),
    ("edgenodeCpuLimit", "cpuLimit"),
    ("edgenodeMemoryRequest", "memoryRequest"),
    ("edgenodeMemoryLimit", "memoryLimit"),
)
_ARCH_BUDGETS_PATH = "genesis/data/devices/archetype-resource-budgets.json"


def _archetype_resource_alignment(write: dict) -> bool:
    import json as _json
    import sys as _sys
    if Path(write["path"]).name != "deployments.json":
        return False
    content = write.get("content")
    if content is None:
        return False
    try:
        humans = _json.loads(content).get("humans", [])
    except Exception:
        return False  # unparseable pending edit -> abstain (never block on a syntax error)
    try:
        budgets = _json.loads(Path(_ARCH_BUDGETS_PATH).read_text()).get("budgets", {})
    except Exception:
        budgets = {}
    drifts = []
    for h in humans:
        if h.get("pattern") != "consolidated":
            continue
        arch = h.get("deviceArchetype")
        ov = h.get("resourceOverride") or {}
        justified = bool(ov.get("justification"))
        canon = budgets.get(arch, {})
        for dep_field, budget_key in _ARCH_FIELDS:
            val = h.get(dep_field)
            if val is None:
                continue
            if justified and budget_key in ov:
                if val != ov[budget_key]:
                    drifts.append((h.get("name"), arch, budget_key, val, f"override declares {ov[budget_key]}"))
                continue
            expected = canon.get(budget_key)
            if expected is not None and val != expected:
                drifts.append((h.get("name"), arch, budget_key, val, f"archetype canonical {expected}"))
    if not drifts:
        return False
    print("  archetype-resource-alignment — intra-class drift (one-off instead of the class):", file=_sys.stderr)
    for name, arch, field, val, exp in drifts:
        print(f"    · {name} [{arch}] {field}={val} != {exp}", file=_sys.stderr)
    print("    FIX: move the archetype's budget (realigns the whole class) OR add "
          "resourceOverride:{<field>,justification} to the exceptional human.", file=_sys.stderr)
    return True


# ── Alpha test-bench aggregate capacity (a validator-EPR). The Rakia ledger is an
# operator-promoted Category-C observation, not a live-cluster oracle. This validator replaces the
# ledger's previously observed human allocation with the prospective active deployments, retains the
# observed non-human commitments, and checks both requests and limits against allocatable CPU/memory.
# It evaluates from BOTH directions: a deployments.json edit uses the ledger on disk; a promoted
# compute-capacity.json edit uses deployments.json on disk. No kubectl/Prometheus dependency.
_CAPACITY_LEDGER_PATH = "genesis/data/rakia/compute-capacity.json"
_DEPLOYMENTS_PATH = "genesis/orchestrator/data/deployments.json"


def _cpu_m(value) -> int | None:
    if isinstance(value, int):
        return value
    if not isinstance(value, str):
        return None
    try:
        if value.endswith("m"):
            return int(value[:-1])
        return int(float(value) * 1000)
    except (TypeError, ValueError):
        return None


def _memory_mi(value) -> int | None:
    if isinstance(value, int):
        return value
    if not isinstance(value, str):
        return None
    units = (("Ki", 1 / 1024), ("Mi", 1), ("Gi", 1024), ("Ti", 1024 * 1024))
    try:
        for suffix, factor in units:
            if value.endswith(suffix):
                return int(float(value[:-len(suffix)]) * factor)
        return int(value)
    except (TypeError, ValueError):
        return None


def _bundle(obj: dict | None, cpu_key: str = "cpu_m", memory_key: str = "memory_Mi") -> tuple[int, int]:
    obj = obj or {}
    return (_cpu_m(obj.get(cpu_key)) or 0, _memory_mi(obj.get(memory_key)) or 0)


def _planned_human_resources(deployments: dict, kind: str) -> tuple[int, int]:
    cpu_key = "edgenodeCpuRequest" if kind == "requests" else "edgenodeCpuLimit"
    memory_key = "edgenodeMemoryRequest" if kind == "requests" else "edgenodeMemoryLimit"
    cpu = memory = 0
    for human in deployments.get("humans", []):
        if human.get("pattern") != "consolidated" or human.get("suspended") is True:
            continue
        cpu += _cpu_m(human.get(cpu_key)) or 0
        memory += _memory_mi(human.get(memory_key)) or 0
    return cpu, memory


def _observed_human_resources(ledger: dict, kind: str) -> tuple[int, int]:
    cpu = memory = 0
    for human in (ledger.get("elohimHumans") or {}).values():
        if human.get("suspended") is True:
            continue
        for deployment in (human.get("deployments") or {}).values():
            resource = deployment.get(kind) or deployment.get(f"{kind}_sum") or {}
            dcpu, dmemory = _bundle(resource)
            cpu += dcpu
            memory += dmemory
    return cpu, memory


def _observed_total_limits(ledger: dict) -> tuple[int, int]:
    cpu = memory = 0
    node_types = ((ledger.get("cluster") or {}).get("nodeTypes") or {}).values()
    for node_type in node_types:
        for node in node_type.get("nodes", []):
            if node.get("ready") is not True:
                continue
            ncpu, nmemory = _bundle(node.get("limits"))
            cpu += ncpu
            memory += nmemory
    return cpu, memory


def _aggregate_capacity_violations(deployments: dict, ledger: dict) -> list[tuple[str, str, int, int]]:
    cluster = ledger.get("cluster") or {}
    alloc_cpu, alloc_memory = _bundle(cluster.get("totalAllocatable"))
    committed_cpu, committed_memory = _bundle(cluster.get("totalCommitted"))
    observed_req_cpu, observed_req_memory = _observed_human_resources(ledger, "requests")
    planned_req_cpu, planned_req_memory = _planned_human_resources(deployments, "requests")
    observed_lim_cpu, observed_lim_memory = _observed_human_resources(ledger, "limits")
    planned_lim_cpu, planned_lim_memory = _planned_human_resources(deployments, "limits")
    total_lim_cpu, total_lim_memory = _observed_total_limits(ledger)

    projected = {
        ("requests", "cpu_m"): max(0, committed_cpu - observed_req_cpu) + planned_req_cpu,
        ("requests", "memory_Mi"): max(0, committed_memory - observed_req_memory) + planned_req_memory,
        ("limits", "cpu_m"): max(0, total_lim_cpu - observed_lim_cpu) + planned_lim_cpu,
        ("limits", "memory_Mi"): max(0, total_lim_memory - observed_lim_memory) + planned_lim_memory,
    }
    ceilings = {"cpu_m": alloc_cpu, "memory_Mi": alloc_memory}
    return [
        (kind, resource, value, ceilings[resource])
        for (kind, resource), value in projected.items()
        if ceilings[resource] > 0 and value > ceilings[resource]
    ]


def _test_bench_aggregate_capacity(write: dict) -> bool:
    import json as _json
    import sys as _sys

    name = Path(write["path"]).name
    if name not in ("deployments.json", "compute-capacity.json"):
        return False
    try:
        if name == "deployments.json":
            deployments = _json.loads(write.get("content") or "")
            ledger = _json.loads(Path(_CAPACITY_LEDGER_PATH).read_text())
        else:
            ledger = _json.loads(write.get("content") or "")
            deployments = _json.loads(Path(_DEPLOYMENTS_PATH).read_text())
    except Exception:
        return False  # syntax/IO diagnostics belong to their dedicated validators

    violations = _aggregate_capacity_violations(deployments, ledger)
    if not violations:
        return False
    print("  test-bench-aggregate-capacity — prospective portfolio exceeds promoted envelope:",
          file=_sys.stderr)
    for kind, resource, value, ceiling in violations:
        print(f"    · aggregate {kind}.{resource}={value} > allocatable {ceiling}",
              file=_sys.stderr)
    print("    FIX: reconcile active budgets, promote a newer operator-observed Rakia ledger, OR "
          "acknowledge the intentional overcommit at reach.", file=_sys.stderr)
    return True


# ══ Concern-canon validators (seam-concern-contract plan, design surface 4 / task P4.1) ═══════
# Three concrete detectors so standing prose nags become EVALUATED gates. Each is
# precision-biased by construction: every one of them is bound at `class: inject` (advisory —
# the escalation ladder's agent-self-grant tier), so a false positive costs one advisory line,
# never a blocked write. That asymmetry is deliberate and load-bearing — a fuzzy heuristic must
# never deny, and the 2026-07-25 policy-family lesson (an unresolvable validator may not HARDEN
# an advisory rule) points the same way.
#
# Shared shape helpers first: `_prior_text` (the sovereignty-guard's pre-image read, factored)
# and `_added_lines` (multiset line difference — "what did this write INTRODUCE"). Net-new
# counting is what keeps a maintenance edit to an already-offending file from being trapped:
# cleaning or merely touching a file never fires, only ADDING the shape does.

def _prior_text(write: dict) -> str | None:
    """Pre-edit content of the written path. `""` for a brand-new file (everything is net-new);
    None when the prior state is unreadable — callers decide which way to fail, and for these
    advisory validators the honest choice is to surface (the 2026-07-25 clause cuts the other
    way only for rules that GATE)."""
    if write.get("is_new"):
        return ""
    try:
        return Path(write["path"]).read_text(errors="replace")
    except OSError:
        return None


def _added_lines(pre: str, post: str) -> list[str]:
    """Lines `post` carries MORE times than `pre` — a multiset difference, not a real diff.
    Order-free and O(n), which is all a "what did this write introduce" check needs; a moved
    line is correctly NOT reported as added, and a duplicated one correctly is."""
    from collections import Counter
    remaining = Counter(pre.splitlines())
    out: list[str] = []
    for line in post.splitlines():
        if remaining.get(line, 0) > 0:
            remaining[line] -= 1
        else:
            out.append(line)
    return out


def _repo_rel(path: str) -> str:
    """Repo-relative POSIX path for `path` (validators receive absolute tool paths). Falls back
    to the input when the path sits outside the resolved repo root."""
    p = Path(path)
    try:
        return p.resolve().relative_to(find_repo_root(p).resolve()).as_posix()
    except Exception:  # noqa: BLE001
        return p.as_posix()


# ── C2 / heal-fills-never-moves (`epr:validator-heal-fills-never-moves`, bound by
# c2-monotonic-authority@2). Two arms, both narrow:
#   (a) a NET-NEW `StampMode::Declare` call site in a file that is not a canonical channel.
#       `Declare` may move a declared head anywhere (including a deliberate revert), so widening
#       its call-site set is exactly how the 2026-07-11 resurrection class returns — 2,838 rows
#       restamped backwards, serially, after every restart.
#   (b) a write whose ADDED lines touch the stamp guard while the file cites NONE of the contract
#       tests the seam registry registers for it — the "blind first fix" shape (defect #3: the
#       guard was re-keyed on WHO writes and froze forward adoption; the tests are what named the
#       difference). Arm (b) is skipped entirely when the registry registers no test for the file:
#       an honest, census-owned gap is not this validator's finding to make.
# The word-boundary regex matters: `CollectiveStampMode::Declare` (the collectives plane, a
# different enum) contains `StampMode::Declare` as a substring and must NOT be counted.
_HEAL_CANONICAL_CHANNEL_FILES = (
    # The stamp implementation itself: the `stamp_declared_head` deliberate-declare wrapper, the
    # `StampMode::Declare => {}` guard arm, and their in-file contract tests.
    "elohim/elohim-storage/src/db/content_diesel.rs",
    # The own-conductor deliberate canonical act after `adopt_peer` mints the link.
    "elohim/elohim-storage/src/services/head_adoption.rs",
)
_DECLARE_CALL_RE = re.compile(r"(?<![A-Za-z0-9_])StampMode::Declare(?![A-Za-z0-9_])")
_STAMP_GUARD_SYMBOLS = (
    "canonical_move_verdict", "StampMode::HealCanonical", "StampMode::GapFill",
    "moving_declared_row", "SkippedStale", "stamp_declared_head_mode",
)


def _registered_contract_tests(path: str) -> set[str]:
    """Contract-test names the nearest `seam-registry.yaml` registers for this source file.
    Delegates to the census read-model (imported lazily — seam_census imports THIS module, so a
    top-level import would cycle); the registry reader stays in exactly one place."""
    try:
        from _lib import seam_census
        return seam_census.contract_test_names_for(Path(path))
    except Exception:  # noqa: BLE001 — a missing//broken registry must never break a gate
        return set()


def _heal_fills_never_moves(write: dict) -> bool:
    import sys as _sys
    path = write.get("path") or ""
    if not path.endswith(".rs"):
        return False
    post = write.get("content")
    if post is None:
        return False  # content unresolved (unreadable disk / failing Edit) — abstain
    pre = _prior_text(write)
    rel = _repo_rel(path)

    # (a) net-new Declare outside the canonical-channel list
    post_n = len(_DECLARE_CALL_RE.findall(post))
    pre_n = len(_DECLARE_CALL_RE.findall(pre)) if pre is not None else 0
    if post_n > pre_n and rel not in _HEAL_CANONICAL_CHANNEL_FILES:
        print(f"  heal-fills-never-moves — net-new `StampMode::Declare` call site in {rel} "
              f"({pre_n} -> {post_n}), which is NOT a canonical channel:", file=_sys.stderr)
        for ch in _HEAL_CANONICAL_CHANNEL_FILES:
            print(f"    · canonical channel: {ch}", file=_sys.stderr)
        print("    FIX: a heal carrying a canonical answer stamps `HealCanonical` (move only with "
              "proof of forward ordering); a fallback resolve stamps `GapFill` (fill an empty row "
              "only). Widen `Declare` only by adding a deliberate canonical channel here.",
              file=_sys.stderr)
        return True

    # (b) stamp-guard edit that cites none of its registered contract tests
    added = _added_lines(pre if pre is not None else "", post)
    touched = sorted({sym for line in added for sym in _STAMP_GUARD_SYMBOLS if sym in line})
    if touched:
        tests = _registered_contract_tests(path)
        if tests and not any(t in post for t in tests):
            print(f"  heal-fills-never-moves — {rel} edits the stamp guard ({', '.join(touched)}) "
                  f"while citing none of its registered contract tests:", file=_sys.stderr)
            for t in sorted(tests):
                print(f"    · {t}", file=_sys.stderr)
            print("    FIX: keep (or name in the predicate's doc comment) the contract test that "
                  "proves the ordering — a guard edit with no cited proof is how the blind "
                  "who-writes fix froze forward adoption.", file=_sys.stderr)
            return True
    return False


# ── C6a / bounded work (`epr:validator-bounded-work`, bound by c6a-bounded-work@2). The class's
# teeth are in its SCOPE clause: the expensive instances had no `loop` token in the diff at all
# (a retry policy against an uncancellable call, 74fbdf2d7) or asserted a budget in the wrong
# direction. This detector is deliberately narrow — it fires only on ADDED lines carrying an
# unambiguous unbounded-work shape, inside a crate that actually registers decision points (a
# `seam-registry.yaml` at some ancestor), with NO budget vocabulary in the surrounding window.
# It will miss real instances; it is built not to invent false ones, because the honest signal
# for a fuzzy shape is an advisory and an advisory that cries wolf is deleted, not obeyed.
_BOUNDED_WORK_SHAPES = (
    "loop {",           # a bare infinite loop
    "max_retries", "max_attempts", "retry_count", "retries", "retry_forever",
    "backoff", ".retry(", "RetryPolicy", "RetryConfig",
)
_BOUNDED_WORK_BUDGET_TOKENS = (
    "budget", "deadline", "timeout", "Duration::from", "max_per", "batch_size", "limit",
    ".take(", "_cap", "cap:", "MAX_", "_MAX", "drain_publish_queue", "tokio::select",
)
# A one-line author declaration naming the budget suppresses the shape (the sovereignty-guard's
# `sovereignty-frame:` idiom): the guarantee is a DECLARED budget, so declaring it satisfies it.
_BOUNDED_WORK_MARKER = "bounded-work:"
_BOUNDED_WORK_WINDOW = 12  # lines either side of the shape searched for budget vocabulary


def _has_registered_seam(path: Path) -> bool:
    """True iff some ancestor directory carries a `seam-registry.yaml` (a REGISTERED seam).
    Restricting the detector to registered seams is the precision half of the bias: those are
    the crates whose decision points are enumerated, so a budget question there has a home."""
    try:
        cur = path.resolve().parent
    except Exception:  # noqa: BLE001
        return False
    for _ in range(MAX_CASCADE_DEPTH):
        if (cur / "seam-registry.yaml").is_file():
            return True
        if cur.parent == cur:
            break
        cur = cur.parent
    return False


def _bounded_work(write: dict) -> bool:
    import sys as _sys
    path = write.get("path") or ""
    if not path.endswith(".rs"):
        return False
    post = write.get("content")
    if post is None:
        return False
    if _BOUNDED_WORK_MARKER in post:
        return False  # budget declared in-content by the author
    if not _has_registered_seam(Path(path)):
        return False
    pre = _prior_text(write)
    added = set(_added_lines(pre if pre is not None else "", post))
    if not added:
        return False
    lines = post.splitlines()
    hits: list[tuple[int, str, str]] = []
    for i, line in enumerate(lines):
        if line not in added:
            continue
        stripped = line.strip()
        if stripped.startswith(("//", "/*", "*", "#")):
            # A COMMENT naming a retry token is documentation of a budget, not a new ladder —
            # and it is the single largest false-positive source measured against the live
            # storage crate (every `max_retries` mentioned in a doc comment). Skipping comments
            # is what keeps this advisory worth reading.
            continue
        shape = next((s for s in _BOUNDED_WORK_SHAPES if s in line), None)
        if shape is None:
            continue
        lo = max(0, i - _BOUNDED_WORK_WINDOW)
        window = "\n".join(lines[lo:i + _BOUNDED_WORK_WINDOW + 1])
        if any(tok in window for tok in _BOUNDED_WORK_BUDGET_TOKENS):
            continue
        hits.append((i + 1, shape, line.strip()[:100]))
    if not hits:
        return False
    print(f"  bounded-work — new unbounded-work shape(s) in {_repo_rel(path)} with no budget "
          f"within ±{_BOUNDED_WORK_WINDOW} lines:", file=_sys.stderr)
    for lineno, shape, text in hits:
        print(f"    · line {lineno} [{shape}] {text}", file=_sys.stderr)
    print("    FIX: declare the budget the loop/ladder respects (a per-tick cap, a deadline, a "
          "bounded `take`), or bind the existing drain kernel (`drain_publish_queue` + "
          "wait-for-drain) rather than minting a fourth pacing vocabulary. A retry policy "
          f"against an UNCANCELLABLE call is a loop even with no loop token. Declaring the "
          f"budget in a `{_BOUNDED_WORK_MARKER} <budget>` comment satisfies this advisory.",
          file=_sys.stderr)
    return True


# ── DNA hash neutrality (`epr:validator-dna-hash-neutrality`) — the one registered validator with
# NO canon class behind it. The ceremony tested it as a class and declined: "DNA-hash-neutrality
# classification (integrity vs coordinator diffs) has no runtime predicate — it becomes the third
# registered validator, not a class." So it enters as policy MACHINERY only: no concerns.yaml row,
# no policies.yaml row, just a detector attached where the resolver evaluates DNA diffs
# (`elohim/holochain/dna/.epr-meta`).
#
# What it classifies: the Holochain DNA hash covers INTEGRITY zomes + modifiers ONLY. A
# coordinator-zome-only change moves no hash and heals via `update_coordinators` hot-swap (cheap,
# no re-key, no DHT churn); an integrity-zome or modifiers change moves the hash and therefore
# rides the reinstall path — which mints a new agent key and, applied to SOME peers and not all,
# partitions the namespace into two DHTs. The validator LABELS the hash-moving class (that is the
# trap) and stays silent on the coordinator-only class (the cheap path needs no warning) — it
# never blocks: its rule is `class: inject`.
_DNA_TREE_PREFIX = "elohim/holochain/dna/"
# Only files that end up INSIDE the zome wasm can move the hash. A README or a `.md` note living
# in an integrity zome's directory changes no compiled byte — firing on those would train the
# reader to ignore the advisory that matters.
_DNA_WASM_BEARING_SUFFIXES = (".rs", "Cargo.toml", "Cargo.lock", ".wasm")


def _dna_modifier_block(text: str) -> str:
    """The hash-relevant slice of a `dna.yaml`: the `integrity:` section (whose zome list and
    the `origin_time`/`network_seed`/`properties` modifiers under it are what the DNA hash is
    computed over). Everything below the `coordinator:` key is hot-swappable and excluded."""
    out, keeping = [], False
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("integrity:"):
            keeping = True
        elif stripped.startswith("coordinator:"):
            keeping = False
        if keeping:
            out.append(line.rstrip())
    return "\n".join(out)


def _dna_hash_neutrality(write: dict) -> bool:
    import sys as _sys
    rel = _repo_rel(write.get("path") or "")
    if not rel.startswith(_DNA_TREE_PREFIX):
        return False
    tail = rel[len(_DNA_TREE_PREFIX):]
    parts = tail.split("/")
    post = write.get("content")

    kind = None  # "integrity-zome" | "modifiers" | "coordinator-zome"
    if "zomes" in parts:
        zome_dir = parts[parts.index("zomes") + 1] if len(parts) > parts.index("zomes") + 1 else ""
        if not rel.endswith(_DNA_WASM_BEARING_SUFFIXES):
            return False  # docs/config beside a zome compile into nothing — neutral by construction
        kind = "integrity-zome" if "integrity" in zome_dir else "coordinator-zome"
    elif parts and parts[-1] == "dna.yaml":
        if post is None:
            return False
        pre = _prior_text(write)
        if pre is not None and _dna_modifier_block(pre) == _dna_modifier_block(post):
            return False  # coordinator-side / cosmetic edit — the hashed slice is untouched
        kind = "modifiers"

    if kind in (None, "coordinator-zome"):
        # Coordinator-only (or not a DNA-hash-bearing surface at all): NEUTRAL. Silent by design —
        # the hot-swap path carries no trap, and an advisory on every coordinator edit would be
        # the nag this whole registry exists to avoid.
        return False

    label = ("an INTEGRITY zome" if kind == "integrity-zome"
             else "the hashed `integrity:` slice of dna.yaml (modifiers)")
    print(f"  dna-hash-neutrality — HASH-MOVING: {rel} is {label}.", file=_sys.stderr)
    print("    · integrity zomes + modifiers ARE the DNA hash; coordinator zomes are NOT.",
          file=_sys.stderr)
    print("    · consequence: this diff cannot land by `update_coordinators` hot-swap. It needs a "
          "deliberate DNA-lineage event (ALLOW_DNA_REINSTALL), which mints a NEW agent key — and "
          "reinstalling SOME peers in a namespace but not all splits one DHT into two.",
          file=_sys.stderr)
    print("    FIX: if the change can be expressed coordinator-side, do that (hot-swap, no re-key, "
          "no churn). If it genuinely belongs in integrity, ride a planned lineage event with the "
          "whole namespace — the alpha genesis PAIR must both get the flag.", file=_sys.stderr)
    return True


# Declared runtime-scoped validator refs: NOT unresolvable — Unavailable-by-declaration, skips
# clean without downgrading the rule (constraint 6). Value names the runtime that owns them.
#
# DERIVED, never hand-written. The scope map lives in ONE place —
# `elohim/sdk/schemas/v1/registries/governance-validators.json` — which the Rust provider
# (epr-cli's ElohimRepositoryValidators) reads for the mirror-image question. Two hand-maintained
# lists that must agree IS the fork this whole surface exists to close; a shared registry makes
# disagreement unrepresentable rather than merely tested-for.
#
# An unreadable/absent registry yields an EMPTY map here — the same degradation the Rust side
# takes — so the two hosts stay in correspondence even in failure. Empty means "nothing is
# declared elsewhere", which routes unknown refs rather than silently skipping them: the safe
# direction, and identical on both sides.
_VALIDATOR_SCOPE_REL = "elohim/sdk/schemas/v1/registries/governance-validators.json"


def _load_runtime_scoped() -> dict:
    """Refs the SHARED registry assigns to a runtime other than Python."""
    try:
        import json as _json
        root = Path(__file__).resolve().parents[3]
        with open(root / _VALIDATOR_SCOPE_REL, encoding="utf-8") as fh:
            registry = _json.load(fh)
        return {v["ref"]: f"{v['runtime']}-only"
                for v in registry.get("validators", [])
                if isinstance(v, dict) and v.get("ref") and v.get("runtime") not in (None, "python", "both")}
    except Exception:  # noqa: BLE001 — absent/unreadable registry degrades to "nothing declared"
        return {}


RUNTIME_SCOPED_VALIDATORS = _load_runtime_scoped()

# A validator that FLAGS a write can tag the fired Verdict with a specific refer_reason (the
# ceiling-law vocabulary, constraint 3). Absent here -> decision_for() defaults to "rule-fired".
VALIDATOR_REFER_REASONS = {
    "epr:validator-escalation-ladder": "escalation-requires-ratification",
}


# ── Agency-charter validator (the ladder, constraint 4). Agents self-grant measure/inject (+
# dispatch with a named agent); ask/deny authorship or promotion requires an operator-ratified
# policy pin. Fires on a `.epr-meta`/manifest write that INTRODUCES an ask|deny rule (new file:
# any ask|deny rule; existing file: compare against prior on-disk content, so unrelated edits to an
# already-governed manifest never retrigger) not backed by a `policy:` binding whose registry row's
# `established_by` starts with `operator-` (the pending-ratification convention counts as backed).
def _frontmatter_or_whole(text: str) -> str:
    """Mirror of the Rust `frontmatter_or_whole` (repository_validators.rs): real .epr-meta
    files are `---`-fenced frontmatter + body — raw safe_load on them fails/abstains, which
    would make any content-parsing validator silently never fire. Fenced → the block; bare
    YAML → the whole text. Cross-runtime parity requires both sides to accept both shapes."""
    if text.startswith("---\n"):
        rest = text[4:]
        end = rest.find("\n---")
        if end != -1:
            return rest[:end]
    return text


def _rule_ask_deny_classes(content: str | None, policies: dict) -> dict:
    """Parse manifest YAML `content` -> {rule_id: effective_class} for rules whose EFFECTIVE class
    (inline `class:`, or the bound registry policy's `class:`) is ask or deny. Best-effort: any
    parse failure abstains ({})."""
    if not content or yaml is None:
        return {}
    try:
        data = yaml.safe_load(_frontmatter_or_whole(content))
    except Exception:
        return {}
    if not isinstance(data, dict):
        return {}
    out: dict = {}
    for rule in data.get("rules", []) or []:
        if not isinstance(rule, dict):
            continue
        rid = rule.get("id")
        if not rid:
            continue
        ref = rule.get("policy")
        cls = policies.get(ref, {}).get("class") if isinstance(ref, str) else rule.get("class")
        if cls in ("ask", "deny"):
            out[rid] = cls
    return out


def _rule_backed_by_ratified_policy(rid: str, content: str | None, policies: dict) -> bool:
    """True iff rule `rid` in `content` binds a `policy:` ref whose registry row's
    `established_by` starts with `operator-`. An INLINE ask|deny (no `policy:` binding) is NEVER
    backed — the charter requires an operator-ratified pin for that class, not agent
    self-declaration."""
    if not content or yaml is None:
        return False
    try:
        data = yaml.safe_load(_frontmatter_or_whole(content))
    except Exception:
        return False
    if not isinstance(data, dict):
        return False
    for rule in data.get("rules", []) or []:
        if not isinstance(rule, dict) or rule.get("id") != rid:
            continue
        ref = rule.get("policy")
        if not isinstance(ref, str):
            return False
        pol = policies.get(ref)
        return bool(
            pol
            and str(pol.get("established_by", "")).startswith(("operator-", "deliberated-"))
        )
    return False


def _escalation_ladder(write: dict) -> bool:
    path = Path(write["path"])
    if not is_manifest_path(path):
        return False
    content = write.get("content")
    root = find_repo_root(path)
    policies, _errs = load_policies(root)
    post = _rule_ask_deny_classes(content, policies)
    if write.get("is_new"):
        prior_ids: set = set()
    else:
        try:
            prior = path.read_text(errors="replace")
        except OSError:
            prior = None
        prior_ids = set(_rule_ask_deny_classes(prior, policies).keys())
    introduced = {rid: c for rid, c in post.items() if rid not in prior_ids}
    return any(not _rule_backed_by_ratified_policy(rid, content, policies) for rid in introduced)


REFERENCE_VALIDATORS = {
    "epr:validator-p2p-design-gate": _p2p_design_gate,
    "epr:validator-brand-vocabulary-boundary": _brand_vocabulary_boundary,
    "epr:validator-sovereignty-ontology-guard": _sovereignty_ontology_guard,
    "epr:validator-ownership-ontology-guard": _ownership_ontology_guard,
    "epr:validator-archetype-resource-alignment": _archetype_resource_alignment,
    "epr:validator-test-bench-aggregate-capacity": _test_bench_aggregate_capacity,
    "epr:validator-escalation-ladder": _escalation_ladder,
    # Concern-canon validators (plan P4.1). The first two realize canon classes
    # (c2-monotonic-authority@2, c6a-bounded-work@2); the third is machinery with no class.
    "epr:validator-heal-fills-never-moves": _heal_fills_never_moves,
    "epr:validator-bounded-work": _bounded_work,
    "epr:validator-dna-hash-neutrality": _dna_hash_neutrality,
}


# ── Policy registry: define-once-bind-many rules (the Mishpat::Precedent shape, dev-tooling tier).
# The registry YAML holds Precedent-shaped policy objects; a manifest rule binds one with
# `policy: <id>@<version>` (+ optional `params`/`when` local variance). Expansion happens at
# resolve time (after merge_rules, before evaluate) so `evaluate` stays pure and unchanged.
# Graduated home: mishpat_integrity::Precedent entries (CID = entry_hash), bindings become cites. ──
POLICY_REGISTRY_REL = ".claude/epr-meta/policies.yaml"

# contentHash pins the row's SEMANTICS (canonical JSON, sorted+compact) minus lifecycle/volatile
# fields — status and supersession lineage change without altering what the row governs, so they
# stay out of the hash; contentHash itself is obviously excluded. Shared by load_policies
# (verify) and epr-meta-pin.py (compute/write) — the SAME canonicalization, or the two would drift.
# `retire-when` joins the lifecycle keys OUTSIDE the semantic hash, and the reason is a
# incentive one, not a convenience one. The contentHash pins what a policy DOES to a diff;
# a removal condition says when the policy stops existing — two different questions, and a
# binding behaves identically with or without one. If declaring an exit required minting a new
# policy version (and re-pinning every hash), exits would be the most expensive thing in the
# registry to add, which is precisely the wrong gradient for the trap this key exists to spring:
# the accretion is already free, so the exit must be free too. Editing a retire condition is
# therefore an amendment, not a supersession.
_HASH_EXCLUDE_KEYS = {"contentHash", "status", "superseded_by", "retire-when"}


def policy_content_hash(pol: dict) -> str:
    """Canonical `sha256:<hex>` contentHash for one policy registry row."""
    body = {k: v for k, v in pol.items() if k not in _HASH_EXCLUDE_KEYS}
    canon = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=True)
    return "sha256:" + hashlib.sha256(canon.encode("utf-8")).hexdigest()


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
            # L6: the registry loader is the path that actually feeds the evaluator
            # (resolve() -> merge_rules -> load_policies -> expand_policies -> evaluate), and a
            # manifest rule that BINDS a registry policy carries no `class`/`measure` of its own
            # (only `_BINDING_KEYS`) — so validate_meta's `cls == "measure"` branch never sees a
            # binding, only an inline rule. Skipping the kind check here would leave the registry
            # as the one path where `kind` isn't required, undermining the very claim that
            # `measure:` is policy-owned. Same failure mode as the loc-soft/loc-hard check above:
            # append + NOT loaded, so a binding drops LOUD via expand_policies' unknown-policy
            # path instead of silently un-enforcing.
            kind = m.get("kind")
            if kind not in MEASURE_KIND_VOCAB:
                errs.append(f"policy `{key}` class `measure` needs `measure.kind` in "
                            f"{sorted(MEASURE_KIND_VOCAB)} (got {kind!r}) — NOT loaded")
                continue
            if kind == "rate" and not m.get("per"):
                errs.append(f"policy `{key}` `measure.kind: rate` requires `measure.per:` "
                            f"(second|minute|hour|day|week|month|year) — NOT loaded")
                continue
        # `retire-when` is SEMANTICS, so it belongs to the policy, never to a binding
        # (`_BINDING_KEYS` admits only id/policy/params/when/why — a binding that tried to
        # declare its own removal condition would be re-deciding the policy's own exit).
        # A malformed one is NOT a load-blocker: an unusable retire condition should surface
        # as an advisory the census counts, not silently un-enforce a live deny.
        if "retire-when" in pol:
            errs.extend(_validate_retire_when(pol["retire-when"], f"policy `{key}`"))
        stored_hash = pol.get("contentHash")
        if stored_hash:
            computed = policy_content_hash(pol)
            if computed != stored_hash:
                errs.append(f"policy `{key}` contentHash mismatch — registry tampered (pinned "
                            f"{stored_hash}, computed {computed}); bindings ROUTE to review "
                            f"(policy-pin-mismatch), not enforced as authored")
                out[key] = {"__pin_mismatch__": True}
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
        if pol.get("__pin_mismatch__"):
            # Tampered registry: the binding is suspect, not simply unknown — route to review
            # rather than enforcing (or silently dropping) possibly-altered semantics.
            errs.append(f"rule `{rid}` binds policy `{ref}` whose contentHash mismatches — "
                        f"routing to review (policy-pin-mismatch)")
            merged["rules"][rid] = {
                "id": rid, "class": "ask", "pin-mismatch": True,
                "why": f"policy `{ref}` contentHash mismatch — registry may be tampered; this "
                       f"binding routes to review rather than enforcing possibly-altered "
                       f"semantics. Re-pin with epr-meta-pin.py --write after confirming the "
                       f"edit was legitimate.",
                "when": rule.get("when") or {}, "policy-ref": ref,
                "refer-reason": "policy-pin-mismatch",
            }
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


# ── The governance correspondence spine: decision_for() + resolve_write() — the shared, PURE
# per-write decision path both the resolver hook and the git-gate CLI drive (the "same _witness
# helper" / "the REAL path, not a reimplementation" the golden-vector parity runner exercises).
# Decision vocabulary: epr:schema:enum:decision (permit|refuse|refer). ──
FINDINGS_LEDGER_REL = ".claude/data/governance-findings.jsonl"


def evaluator_identity() -> dict:
    """Content address of the PYTHON evaluator — this module's own source.

    The Rust twin addresses its compiled binary; here the source IS the artifact,
    so it is what gets hashed. Symmetry matters more than the mechanism: both
    hosts must be able to say WHICH build decided, or a disagreement names two
    moving targets and cannot be re-derived by anyone."""
    try:
        digest = hashlib.sha256(Path(__file__).resolve().read_bytes()).hexdigest()
        return {"id": "_lib.epr_meta", "cid": f"sha256:{digest}"}
    except Exception:  # noqa: BLE001 — an unreadable source is an honest absence, not a fake id
        return {"id": "_lib.epr_meta", "cid": None}


def witness(root: Path, *, runtime: str, gate: str, subject, decision: str, cls: str | None,
            rule_id: str | None = None, policy_ref: str | None = None, refer: dict | None = None,
            checks: list | None = None, evaluator: dict | None = None) -> None:
    """Append one JSON line to governance-findings.jsonl — the keel Verdict projected to JSONL.
    Shared by the resolver hook AND the git-gate CLI so both runtimes write the SAME shape.
    Flock-append idiom cloned from the resolver's _handle_measures (best-effort; NEVER raises —
    witnessing must never break a gate). `decision` in permit|refuse|refer."""
    try:
        entry = {
            "ts": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "runtime": runtime,
            "gate": gate,
            "subject": subject,
            "decision": decision,
            "class": cls,
            "ruleId": rule_id,
            "policyRef": policy_ref,
            "refer": refer,
            "witness": checks or [],
            # WHICH evaluator produced this decision. Defaulted to the Python
            # evaluator because that is who is running when no caller says
            # otherwise; a caller that delegated to `epr` passes its identity.
            "evaluator": evaluator or evaluator_identity(),
        }
        ledger = Path(root) / FINDINGS_LEDGER_REL
        ledger.parent.mkdir(parents=True, exist_ok=True)
        with open(ledger, "a", encoding="utf-8", errors="replace") as fh:
            locked = fcntl is None
            if not locked:
                deadline = time.monotonic() + 0.5
                while time.monotonic() < deadline:
                    try:
                        fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                        locked = True
                        break
                    except OSError:
                        time.sleep(0.02)
            if locked:
                fh.write(json.dumps(entry, ensure_ascii=False) + "\n")
    except Exception:  # noqa: BLE001 — witnessing must never break the gate
        pass


def decision_for(verdict: Verdict | None) -> dict:
    """PURE: map a combined Verdict (deny/ask/inject, or None) to the keel decision shape
    {decision, cls, rule_id, refer}. deny -> refuse; ask -> refer (refer.reason = the verdict's
    refer_reason, defaulting to 'rule-fired'); inject -> permit (advisory). measure/dispatch never
    reach combine() (severity 0, always non-blocking) — callers handle them separately."""
    if verdict is None:
        return {"decision": "permit", "cls": None, "rule_id": None, "refer": None}
    if verdict.cls == "deny":
        return {"decision": "refuse", "cls": "deny", "rule_id": verdict.rule_id, "refer": None}
    if verdict.cls == "ask":
        reason = verdict.refer_reason or "rule-fired"
        return {"decision": "refer", "cls": "ask", "rule_id": verdict.rule_id,
                "refer": {"layer": "operator", "reason": reason}}
    return {"decision": "permit", "cls": verdict.cls, "rule_id": verdict.rule_id, "refer": None}


def resolve_write(target: Path, write: dict, root: Path, *, verdict_filter=None) -> dict:
    """PURE GIVEN a pure (or absent) `verdict_filter` (read-only disk access for manifests/
    policies, no side effects otherwise): the FULL per-write resolver decision — cascade ->
    malformed check -> merge+expand policies -> evaluate -> combine -> decision mapping. Mirrors
    epr-meta-resolver.py's main() minus stdin/stdout/witnessing/coverage-nudge/arch-ledger side
    effects (those stay runtime-specific in the callers). Returns {decision, cls, rule_id, refer,
    directive, advisories, measures, dispatches, witnessed, merged?} — `witnessed` is False ONLY
    for the silent clean-allow paths (no manifest cascade at all, or a cascade that fired nothing
    AND no malformed-manifest advisory): every other outcome (deny/ask/inject/measure/dispatch/
    malformed-manifest/unresolvable-validator) is True, per the polarity law (fail-open, never
    silent).

    `verdict_filter(verdict, merged) -> bool`, when given, is called BEFORE combine() on every
    fired verdict; True EXCLUDES it from the combine input (but never from `measures`/
    `dispatches`, which always see every fired verdict of their class regardless). The extension
    point for the resolver hook's per-session soft-ceiling inject debounce (a stateful, Claude-
    side concern — this function stays pure by construction: the default `None` is EXACTLY the
    golden-vector parity runner's path, since none of the 9 vectors exercise debounce).

    A malformed manifest on a NON-manifest target hard-routes (returns immediately, mirroring the
    original resolver's `_emit_ask` early-exit — "the gate cannot know what governance intended").
    A malformed manifest on the MANIFEST's own edit is advisory-only and does NOT short-circuit —
    editing an `.epr-meta` is never blocked, and independent concerns (root-anchor, other rules
    that may still apply to the manifest file itself) still get evaluated, exactly as before."""
    chain = collect_cascade(target)
    if not chain:
        return {"decision": "permit", "cls": None, "rule_id": None, "refer": None, "reason": None,
                "directive": None, "advisories": [], "measures": [], "dispatches": [],
                "witnessed": False}

    target_is_manifest = is_manifest_path(target)
    problems = [(m, errs) for m in chain if (errs := check_meta(m))]
    advisories: list[str] = []
    malformed_advisory = False
    if problems:
        detail = "; ".join(f"{m}: {', '.join(e)}" for m, e in problems)
        if target_is_manifest:
            advisories.append(f"[.epr-meta] malformed governance manifest(s) — {detail}. "
                              "(editing an .epr-meta is never blocked, so you can fix it.)")
            malformed_advisory = True
        else:
            return {"decision": "refer", "cls": "ask", "rule_id": None,
                    "refer": {"layer": "operator", "reason": "governance-manifest-malformed"},
                    "reason": f"governance manifest malformed — {detail}. Fix the manifest to "
                              "restore full governance here; proceeding now requires "
                              "confirmation.",
                    "directive": None, "advisories": [], "measures": [], "dispatches": [],
                    "witnessed": True}

    merged = merge_rules(chain)
    policies, pol_errs = load_policies(root)
    exp_errs = expand_policies(merged, policies)
    advisories += [f"[.epr-meta] {e}" for e in (*pol_errs, *exp_errs)]

    verdicts = evaluate(merged, write)
    measures = [v for v in verdicts if v.cls == "measure"]
    dispatches = [v for v in verdicts if v.cls == "dispatch"]
    combine_input = (verdicts if verdict_filter is None
                      else [v for v in verdicts if not verdict_filter(v, merged)])
    combined = combine(combine_input)

    if combined is not None:
        info = decision_for(combined)
        src = merged["sources"][-1] if merged.get("sources") else "?"
        info["reason"] = f"{combined.reason} [rule `{combined.rule_id}` from {src}]"
        return {**info, "directive": None, "advisories": advisories, "measures": measures,
                "dispatches": dispatches, "witnessed": True, "merged": merged}

    if dispatches:
        d = dispatches[0]
        return {"decision": "permit", "cls": "dispatch", "rule_id": d.rule_id, "refer": None,
                "reason": d.reason, "directive": "dispatch", "advisories": advisories,
                "measures": measures, "dispatches": dispatches, "witnessed": True,
                "merged": merged}

    if measures:
        m = measures[0]
        return {"decision": "permit", "cls": "measure", "rule_id": m.rule_id, "refer": None,
                "reason": m.reason, "directive": None, "advisories": advisories,
                "measures": measures, "dispatches": dispatches, "witnessed": True,
                "merged": merged}

    return {"decision": "permit", "cls": None, "rule_id": None, "refer": None, "reason": None,
            "directive": None, "advisories": advisories, "measures": measures,
            "dispatches": dispatches, "witnessed": malformed_advisory, "merged": merged}


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
