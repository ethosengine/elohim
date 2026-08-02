"""seam_census.py — the decision-point census (seam-concern-contract-architecture plan,
task P3.2): the DERIVED read-model over the four (today) `seam-registry.yaml` files plus
the two concern-canon homes (`.claude/epr-meta/policies.yaml` enforcement rows +
`.claude/epr-meta/concerns.yaml` Precedent rows). Wired thin into
`placement-audit.py --epr-meta`; this module owns all the logic, the CLI just prints
`render_census(census_data(ROOT))`.

WHAT IT CHECKS (both directions, per the plan's own P3.2 task text):
  (a) point -> concern: every registered decision point's `contractTests` citations are
      checked for EXISTENCE (the cited file exists) and CONTAINMENT (a `fn <testName>`
      definition is actually present in it) — never for PASSING. Running the suite is the
      Rust gate's job, not this audit's. `contractTests: null` is an honest, schema-required
      gap (paired with `gapNote`), never a finding by itself.
  (b) concern -> points: every one of the 16 canon classes (C0-C14, C6 split into C6a/C6b)
      is checked against every decisionPoint's `concernIds` across every loaded registry,
      in EITHER canon home (policies.yaml's enforcement rows or concerns.yaml's Precedent
      rows). A class with zero referencing decision points anywhere is an `unbound_concern`
      — REPORTED, never a failure (several classes are legitimately unbound today; the plan
      says so explicitly). Structural problems fail LOUD instead: an unparseable registry
      file, a schema-invalid decisionPoint row, or a citation to a nonexistent test.

PER-ROW DEGRADATION (the EprRouter poisoned-scope lesson — a fail-closed `collect` over
rows degrades a whole scope to silence; never do that here): degradation happens at TWO
independent granularities, both `try`/`continue`, never `try`/`raise`:
  - FILE level: one `seam-registry.yaml` that fails to parse, or fails its TOP-LEVEL schema
    shape (missing `crate`/`crateRoot`/`seamLocus`, `decisionPoints` not a non-empty array,
    etc.), is skipped and reported as a `registry-unparseable` finding — every OTHER
    registry still loads in full.
  - ROW level, within one otherwise-good file: one `decisionPoints[i]` entry that fails the
    schema's own `decisionPoint` shape (bad `kind` enum, missing `concernIds`, a `status`
    other than `answered` with no `justification`, `contractTests: null` with no `gapNote`,
    ...) is skipped and reported as a `decision-point-invalid` finding — every OTHER row in
    the SAME file still loads.

VALIDATION STRATEGY: `jsonschema` (if importable — the repo already treats PyYAML as an
optional import with the identical try/except shape, see `_lib.epr_meta.yaml_available`)
validates the top-level document shape and each `decisionPoints[i]` item independently
against `seam-registry.schema.json`'s own `$defs.decisionPoint` (the item schema is
extracted with `$defs` folded in, so its internal `$ref`s resolve with zero external
resolver plumbing — the whole schema is small enough that a `json.dumps`/`json.loads`
round-trip copy is cheap). Item-level and top-level validation are DELIBERATE SEPARATE
passes (item schema loosened to `{}` in the top-level pass) — this is what makes row-level
degradation possible at all; a single jsonschema.validate() over the whole document would
fail atomically on one bad decisionPoint and (per the poisoned-scope lesson) empty the
loaded set. When `jsonschema` is unavailable, `_manual_top_level`/`_manual_decision_point`
re-implement the schema's essential constraints by hand (same graceful-degrade shape as
`epr_meta.load_policies` — a missing optional dependency degrades precision, never crashes).

ANTI-MIRROR CHECK — DELIBERATELY PRECISION-BIASED, READ THIS BEFORE TRUSTING A CLEAN REPORT:
the 2026-07-24 `BlobCid::verifies` incident (concern C5's own `why:` field cites it) is the
type specimen this check targets: a test computed BOTH the actual and the expected value by
calling the identical (wrong) codec path, so the test measured its own mirror and stayed
green forever. This module flags a possible mirror ONLY when, inside the cited test's own
body: (1) the decision point's own symbol is called two or more times, AND (2) at least one
of those calls is the right-hand side of a `let <name-containing-"expect"> = ...` binding.
That is ONE narrow, auditable textual pattern — it WILL miss a real mirror written any other
way (an inline `assert_eq!(f(x), f(x))` with no intermediate `expected` binding, a mirror one
call removed through a helper, a mirror in a *different* file than the one cited here) and it
intentionally declines to analyze decision points whose registry `name` isn't a clean
symbol/`Type::method` form (several rows use natural-language names like
`"contest failure classes (metrics::inc_contest_failed)"` — analyzing THOSE would require
guessing which token is the real callable, which is exactly the imprecision this check is
built to avoid). Every hit is WARN-level (`possible-mirror`), never a hard failure — human
judgment decides. A citation's own author-declared `mirrorRisk` field is honored on both
sides: `true` always surfaces as a finding (self-flagged, no heuristic needed); `false` is an
explicit human clearance and SUPPRESSES the heuristic for that citation entirely (several
rows in the live registries already carry `mirrorRisk: false` after review — respect that
review, don't re-litigate it every census run).

NOT THIS MODULE'S JOB: running `cargo test` (existence + containment only); computing pin
content-hashes for concerns.yaml rows (P0's `epr-meta-pin.py --write` owns that; this module
only needs each row's `id`/`concern`/`version` to build the concern -> home index); the
concern x seam matrix or the calibration ledger (P4.3/P4.4 — this module is their read-model
source, not their renderer).
"""
from __future__ import annotations

import json
import os
import re
from pathlib import Path

from _lib import epr_meta as _em  # reuse the YAML size/depth guards + policies.yaml loader —
                                   # the interface-first-reuse rule: never re-derive YAML/pin loading.

try:
    import yaml  # PyYAML — same optional-import shape as _lib.epr_meta
except Exception:  # pragma: no cover
    yaml = None

try:
    import jsonschema
except Exception:  # pragma: no cover — degrade to the manual validators, never crash
    jsonschema = None

SEAM_REGISTRY_SCHEMA_REL = "elohim/sdk/schemas/v1/manifest/seam-registry.schema.json"
CONCERNS_REGISTRY_REL = ".claude/epr-meta/concerns.yaml"
REGISTRY_FILENAME = "seam-registry.yaml"

# Discovery bounds: heavy/vendor dirs os.walk must never descend into, plus the two
# explicitly-excluded trees the P3.2 task names (genesis/research; any `held/`).
_SKIP_DIR_NAMES = {".git", "node_modules", "target", "dist", "build", "__pycache__",
                   ".venv", "venv", ".cargo-target-pool"}

ALL_CONCERN_IDS = ("C0", "C1", "C2", "C3", "C4", "C5", "C6a", "C6b",
                   "C7", "C8", "C9", "C10", "C11", "C12", "C13", "C14")
_CONCERN_ID_SET = set(ALL_CONCERN_IDS)
_STATUS_VALUES = {"answered", "partial", "unbound", "n-a"}
_KIND_VALUES = {"pure-decision-predicate", "verdict-fn", "boundary-answer-type", "reason-outcome-enum"}
_TEST_KIND_VALUES = {"unit", "sweettest", "integration"}
_TESTNAME_RE = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*$")
_CLEAN_SYMBOL_RE = re.compile(r"^([A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)(?:\s*\(.*\))?$")

_MAX_SOURCE_SCAN_BYTES = 2 * 1024 * 1024  # 2MB — generous for a Rust source file; a guard, not a wall


# ───────────────────────────── discovery ─────────────────────────────

def discover_registry_paths(repo_root: Path) -> list[Path]:
    """Bounded walk for every `seam-registry.yaml` under repo_root. Heavy/vendor dirs are
    PRUNED before descending (os.walk's in-place dirnames mutation — never rglob-then-filter,
    which would still pay the cost of descending into node_modules first). Excludes any
    directory literally named `held` (per the substrate-scope convention: docs/specs held
    for an unavailable capability are out of the planner/runner scan path) and the
    `genesis/research` tree specifically. Sorted for determinism."""
    root = Path(repo_root)
    out: list[Path] = []
    for dirpath, dirnames, filenames in os.walk(root):
        rel_dir = Path(dirpath).relative_to(root).as_posix()
        pruned = []
        for d in dirnames:
            rel_child = d if rel_dir == "." else f"{rel_dir}/{d}"
            if d in _SKIP_DIR_NAMES or d == "held" or rel_child == "genesis/research":
                continue
            pruned.append(d)
        dirnames[:] = pruned
        if REGISTRY_FILENAME in filenames:
            out.append(Path(dirpath) / REGISTRY_FILENAME)
    return sorted(out)


# ───────────────────────────── schema ─────────────────────────────

def load_schema(repo_root: Path) -> tuple[dict | None, str | None]:
    p = Path(repo_root) / SEAM_REGISTRY_SCHEMA_REL
    if not p.is_file():
        return None, f"schema not found at {SEAM_REGISTRY_SCHEMA_REL}"
    try:
        return json.loads(p.read_text()), None
    except Exception as e:  # noqa: BLE001
        return None, f"schema unreadable/invalid JSON: {e}"


def _top_level_schema(full_schema: dict) -> dict:
    """A copy of the full schema with `decisionPoints` items loosened to `{}` (accept
    anything) — isolates TOP-LEVEL structural validity (required keys, types) from
    per-item validity, which `_build_item_validator` checks separately. THIS is what makes
    row-level degradation possible: a malformed item can never make the document ITSELF
    look invalid."""
    doc = json.loads(json.dumps(full_schema))  # cheap deep-copy; schema is ~12KB
    doc["properties"] = dict(doc.get("properties", {}))
    doc["properties"]["decisionPoints"] = {"type": "array", "minItems": 1, "items": {}}
    return doc


def _build_top_level_validator(full_schema: dict):
    if jsonschema is None:
        return None
    try:
        return jsonschema.Draft202012Validator(_top_level_schema(full_schema))
    except Exception:  # noqa: BLE001
        return None


def _build_item_validator(full_schema: dict):
    """A standalone validator for one `decisionPoints[i]` item: the `decisionPoint` $def
    with the schema's `$defs` folded IN (so its internal `#/$defs/...` refs resolve against
    THIS document with zero external resolver/registry plumbing)."""
    if jsonschema is None:
        return None
    try:
        item_schema = dict(full_schema["$defs"]["decisionPoint"])
        item_schema["$defs"] = full_schema["$defs"]
        item_schema["$schema"] = full_schema.get("$schema")
        return jsonschema.Draft202012Validator(item_schema)
    except Exception:  # noqa: BLE001
        return None


def _manual_top_level(data: dict) -> list[str]:
    """Fallback structural check when `jsonschema` is unavailable — re-states the schema's
    essential top-level constraints by hand (same graceful-degrade shape as
    `epr_meta.load_policies` under a missing PyYAML)."""
    errs = []
    if not isinstance(data.get("seamRegistryVersion"), int) or data.get("seamRegistryVersion", 0) < 1:
        errs.append("`seamRegistryVersion` must be an integer >= 1")
    for k in ("crate", "crateRoot", "seamLocus"):
        v = data.get(k)
        if not isinstance(v, str) or not v.strip():
            errs.append(f"`{k}` must be a non-empty string")
    dps = data.get("decisionPoints")
    if not isinstance(dps, list) or not dps:
        errs.append("`decisionPoints` must be a non-empty array")
    return errs


def _manual_decision_point(item) -> list[str]:
    """Fallback per-item structural check mirroring `$defs.decisionPoint` by hand."""
    if not isinstance(item, dict):
        return ["decisionPoint row is not a mapping"]
    errs = []
    if not isinstance(item.get("name"), str) or not item["name"].strip():
        errs.append("`name` must be a non-empty string")
    if item.get("kind") not in _KIND_VALUES:
        errs.append(f"`kind` must be one of {sorted(_KIND_VALUES)}")
    sl = item.get("sourceLocation")
    if not isinstance(sl, dict) or not isinstance(sl.get("file"), str) or not sl.get("file", "").endswith(".rs"):
        errs.append("`sourceLocation.file` must be a string ending in .rs")
    cids = item.get("concernIds")
    if not isinstance(cids, list) or not cids:
        errs.append("`concernIds` must be a non-empty array")
    else:
        for j, cb in enumerate(cids):
            if not isinstance(cb, dict) or cb.get("id") not in _CONCERN_ID_SET:
                errs.append(f"concernIds[{j}].id must be one of {ALL_CONCERN_IDS}")
                continue
            status = cb.get("status")
            if status not in _STATUS_VALUES:
                errs.append(f"concernIds[{j}].status must be one of {sorted(_STATUS_VALUES)}")
            elif status != "answered" and not (
                    isinstance(cb.get("justification"), str) and cb["justification"].strip()):
                errs.append(f"concernIds[{j}] status={status} requires non-empty `justification`")
    tests = item.get("contractTests", "__absent__")
    if tests is None:
        if not isinstance(item.get("gapNote"), str) or not item["gapNote"].strip():
            errs.append("`contractTests: null` requires a non-empty `gapNote`")
    elif isinstance(tests, list):
        if not tests:
            errs.append("`contractTests` array must be non-empty (use null, never [])")
        for j, ct in enumerate(tests):
            if (not isinstance(ct, dict) or not ct.get("path") or not ct.get("testName")
                    or ct.get("kind") not in _TEST_KIND_VALUES):
                errs.append(f"contractTests[{j}] needs path/testName/kind "
                            f"(kind in {sorted(_TEST_KIND_VALUES)})")
            elif not _TESTNAME_RE.match(ct["testName"]):
                errs.append(f"contractTests[{j}].testName `{ct['testName']}` is not a valid identifier")
    else:
        errs.append("`contractTests` must be an array or null")
    return errs


def _validate_top_level(data: dict, validator) -> list[str]:
    if validator is not None:
        try:
            return sorted(
                f"{'/'.join(str(p) for p in e.path) or '<root>'}: {e.message}"
                for e in validator.iter_errors(data))
        except Exception as e:  # noqa: BLE001 — a validator bug must never kill the census
            return _manual_top_level(data) + [f"(schema validator error, used manual fallback: {e})"]
    return _manual_top_level(data)


def _validate_decision_point(item, validator) -> list[str]:
    if validator is not None:
        try:
            return sorted(
                f"{'/'.join(str(p) for p in e.path) or '<root>'}: {e.message}"
                for e in validator.iter_errors(item))
        except Exception as e:  # noqa: BLE001
            return _manual_decision_point(item) + [f"(schema validator error, used manual fallback: {e})"]
    return _manual_decision_point(item)


# ───────────────────────────── registry file loading ─────────────────────────────

def _load_registry_file(path: Path) -> tuple[dict | None, list[str]]:
    """FILE-level load: read + parse only (no schema check yet — that's a separate pass so
    the caller can attribute top-level vs. per-row failures distinctly)."""
    if yaml is None:
        return None, ["PyYAML unavailable — registry not loaded"]
    try:
        if path.stat().st_size > _em.MAX_MANIFEST_BYTES:
            return None, [f"exceeds {_em.MAX_MANIFEST_BYTES // 1024}KB size cap"]
        text = path.read_text()
        if not _em._flow_depth_ok(text):
            return None, ["nesting too deep — refusing to parse"]
        data = yaml.safe_load(text)
    except Exception as e:  # noqa: BLE001
        return None, [f"unreadable/invalid YAML: {e}"]
    if not isinstance(data, dict):
        return None, ["registry root is not a mapping"]
    return data, []


def load_registry(path: Path, top_validator, item_validator) -> tuple[dict | None, list[str], list[str]]:
    """Load + validate ONE `seam-registry.yaml`. Returns (data-with-only-valid-decisionPoints,
    file_errors, row_errors). `data` is None iff the file itself is unparseable or fails its
    TOP-LEVEL schema shape (file-level degradation: this file is dropped, callers are
    unaffected). When `data` is not None, its `decisionPoints` list holds ONLY the rows that
    passed item-level validation — `row_errors` names what was dropped and why (row-level
    degradation: the file still loads, just minus its bad rows)."""
    data, errs = _load_registry_file(path)
    if data is None:
        return None, errs, []
    top_errs = _validate_top_level(data, top_validator)
    if top_errs:
        return None, top_errs, []
    raw_points = data.get("decisionPoints") or []
    good_points = []
    row_errors: list[str] = []
    for i, item in enumerate(raw_points):
        item_errs = _validate_decision_point(item, item_validator)
        if item_errs:
            label = item.get("name", f"<row {i}>") if isinstance(item, dict) else f"<row {i}>"
            row_errors.append(f"decisionPoints[{i}] (`{label}`): " + "; ".join(item_errs))
            continue
        good_points.append(item)
    out = dict(data)
    out["decisionPoints"] = good_points
    return out, [], row_errors


# ───────────────────────────── concerns.yaml (the sibling canon home) ─────────────────────────────

def load_concerns(repo_root: Path) -> tuple[dict, list[str]]:
    """Load `.claude/epr-meta/concerns.yaml` -> ({'id@version': row}, errors). Mirrors
    `_lib.epr_meta.load_policies`'s conventions exactly (same size/depth guards reused
    directly from that module, same per-row degrade-don't-abort discipline) for the sibling
    non-enforcement canon home policies.yaml doesn't cover. Missing file is legitimate
    (`({}, [])` — concerns.yaml is optional infrastructure, not yet universal); unreadable
    or structurally invalid is `({}, [reason])` so the census fails LOUD rather than quietly
    reporting zero concerns from a broken file."""
    p = Path(repo_root) / CONCERNS_REGISTRY_REL
    if not p.is_file():
        return {}, []
    if yaml is None:
        return {}, [f"PyYAML unavailable — {CONCERNS_REGISTRY_REL} not loaded"]
    try:
        if p.stat().st_size > _em.MAX_MANIFEST_BYTES:
            return {}, [f"{CONCERNS_REGISTRY_REL} exceeds {_em.MAX_MANIFEST_BYTES // 1024}KB size cap"]
        text = p.read_text()
        if not _em._flow_depth_ok(text):
            return {}, [f"{CONCERNS_REGISTRY_REL} nesting too deep — refusing to parse"]
        data = yaml.safe_load(text) or {}
    except Exception as e:  # noqa: BLE001
        return {}, [f"{CONCERNS_REGISTRY_REL} unreadable/invalid: {e}"]
    if not isinstance(data, dict) or data.get("concern-canon-version") != 1:
        return {}, [f"{CONCERNS_REGISTRY_REL} missing/invalid `concern-canon-version` (must be 1)"]
    out: dict = {}
    errs: list[str] = []
    for i, row in enumerate(data.get("concerns", []) or []):
        if not isinstance(row, dict) or not row.get("id") or not isinstance(row.get("version"), int):
            errs.append(f"concerns[{i}] needs `id` + integer `version`")
            continue
        key = f"{row['id']}@{row['version']}"
        if key in out:
            errs.append(f"duplicate concern row `{key}` in {CONCERNS_REGISTRY_REL}")
            continue
        cid = row.get("concern")
        if cid not in _CONCERN_ID_SET:
            errs.append(f"concern `{key}` has invalid/missing `concern` id (got {cid!r})")
            continue
        out[key] = row
    return out, errs


# ───────────────────────────── contract-test citation checks ─────────────────────────────

def _finding(severity: str, kind: str, crate: str, point: str, detail: str) -> dict:
    return {"severity": severity, "kind": kind, "crate": crate, "point": point, "detail": detail}


def _symbol_from_name(name: str) -> str | None:
    """Extract a callable symbol from a decisionPoint's `name` for the anti-mirror check —
    ONLY when `name` is (optionally annotated with a trailing parenthetical) a clean
    identifier or `Type::method` path, e.g. `decide_head_action`, `K2Store::put_at
    (MemK2Store)` -> `put_at`. Returns None for natural-language names like `"contest
    failure classes (metrics::inc_contest_failed)"` — deliberately: guessing which token in
    a prose name is "the real symbol" is exactly the imprecision the anti-mirror check is
    built to avoid, so those rows are silently skipped rather than analyzed."""
    m = _CLEAN_SYMBOL_RE.match(name.strip())
    if not m:
        return None
    return m.group(1).rsplit("::", 1)[-1]


def _extract_fn_body(text: str, fn_name: str) -> str | None:
    """Brace-matched extraction of `fn <fn_name>(...) { ... }`'s body from source text.
    Returns None on no-match or unbalanced braces (bail out safely — never over-read)."""
    m = re.search(rf"\bfn\s+{re.escape(fn_name)}\s*\([^)]*\)[^{{]*\{{", text)
    if not m:
        return None
    start = m.end() - 1
    depth = 0
    for i in range(start, len(text)):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return text[start:i + 1]
    return None


def _possible_mirror(body: str, symbol: str) -> bool:
    """See the module docstring's ANTI-MIRROR CHECK section for the full rationale and
    accepted false-negative scope. Fires only when `symbol` is called >=2x in `body` AND at
    least one call is the right-hand side of a `let <...expect...> = ...` binding."""
    call_re = re.compile(rf"\b{re.escape(symbol)}\b\s*(?:::\w+)?\s*\(")
    if len(call_re.findall(body)) < 2:
        return False
    expect_re = re.compile(
        rf"\blet\s+(?:mut\s+)?\w*expect\w*\s*(?::[^=]+)?=\s*[^;]*\b{re.escape(symbol)}\b\s*(?:::\w+)?\s*\(",
        re.IGNORECASE)
    return bool(expect_re.search(body))


def _check_contract_tests(repo_root: Path, crate: str, point: dict) -> tuple[bool, list[dict]]:
    """Existence + containment check for one decisionPoint's `contractTests` citations
    (never runs the suite — that is the Rust gate's job). Returns (any_verified,
    findings). `any_verified` feeds the registry's cited/uncited tally."""
    tests = point.get("contractTests")
    name = point.get("name", "?")
    findings: list[dict] = []
    if not tests:  # None (honest gap, gapNote-required by schema) or [] (shouldn't survive validation)
        return False, findings
    any_verified = False
    for ct in tests:
        path_s, test_name = ct.get("path"), ct.get("testName")
        if not path_s or not test_name:
            findings.append(_finding("error", "missing-contract-test", crate, name,
                                      f"citation missing path/testName: {ct}"))
            continue
        f = Path(repo_root) / path_s
        if not f.is_file():
            findings.append(_finding("error", "missing-contract-test", crate, name,
                                      f"cited test file does not exist: {path_s} (test `{test_name}`)"))
            continue
        try:
            if f.stat().st_size > _MAX_SOURCE_SCAN_BYTES:
                findings.append(_finding("warn", "contract-test-unscanned", crate, name,
                                          f"{path_s} exceeds the scan cap — existence assumed, "
                                          f"containment of `{test_name}` NOT verified"))
                any_verified = True
                continue
            text = f.read_text(errors="replace")
        except OSError as e:
            findings.append(_finding("error", "missing-contract-test", crate, name,
                                      f"cited test file unreadable: {path_s} ({e})"))
            continue
        if not re.search(rf"\bfn\s+{re.escape(test_name)}\s*\(", text):
            findings.append(_finding("error", "missing-contract-test", crate, name,
                                      f"test `{test_name}` not found in {path_s} "
                                      f"(citation to a nonexistent test)"))
            continue
        any_verified = True
        mirror_declared = ct.get("mirrorRisk")
        if mirror_declared is True:
            findings.append(_finding("warn", "possible-mirror", crate, name,
                                      f"`{test_name}` self-declares mirrorRisk: true ({path_s})"))
        elif mirror_declared is False:
            pass  # explicit human clearance after review — do not re-run the heuristic
        else:
            symbol = _symbol_from_name(name)
            if symbol:
                body = _extract_fn_body(text, test_name)
                if body and _possible_mirror(body, symbol):
                    findings.append(_finding(
                        "warn", "possible-mirror", crate, name,
                        f"`{test_name}` calls `{symbol}` >=2x including an 'expect*'-bound "
                        f"call — may be measuring the mirror, not the behaviour ({path_s})"))
    return any_verified, findings


# ───────────────────────────── the census itself ─────────────────────────────

def census_data(repo_root: Path) -> dict:
    """Compute the full census. Never raises — every failure mode degrades to a finding
    at the narrowest granularity that failed (file or row), per the module docstring."""
    repo_root = Path(repo_root)
    schema, schema_err = load_schema(repo_root)
    top_validator = _build_top_level_validator(schema) if schema else None
    item_validator = _build_item_validator(schema) if schema else None

    findings: list[dict] = []
    if schema_err:
        findings.append(_finding("error", "schema-unavailable", "-", "-", schema_err))

    reg_paths = discover_registry_paths(repo_root)
    registries: list[dict] = []
    concern_bound: dict[str, list[dict]] = {cid: [] for cid in ALL_CONCERN_IDS}

    for path in reg_paths:
        rel = path.relative_to(repo_root).as_posix()
        data, file_errs, row_errs = load_registry(path, top_validator, item_validator)
        for e in row_errs:
            findings.append(_finding("error", "decision-point-invalid",
                                      data.get("crate", "?") if data else "?", "-", f"{rel}: {e}"))
        if data is None:
            findings.append(_finding("error", "registry-unparseable", "?", "-",
                                      f"{rel}: " + "; ".join(file_errs)))
            registries.append({"path": rel, "crate": None, "crateRoot": None,
                                "loaded": False, "n_points": 0, "n_invalid_rows": 0,
                                "n_cited": 0, "n_uncited": 0})
            continue
        crate = data.get("crate", "?")
        n_cited = n_uncited = 0
        for pt in data["decisionPoints"]:
            verified, pt_findings = _check_contract_tests(repo_root, crate, pt)
            findings.extend(pt_findings)
            if verified:
                n_cited += 1
            else:
                n_uncited += 1
            for cb in pt.get("concernIds", []):
                cid = cb.get("id")
                if cid in concern_bound:
                    concern_bound[cid].append({
                        "crate": crate, "name": pt.get("name"), "status": cb.get("status")})
        registries.append({
            "path": rel, "crate": crate, "crateRoot": data.get("crateRoot"),
            "loaded": True, "n_points": len(data["decisionPoints"]),
            "n_invalid_rows": len(row_errs), "n_cited": n_cited, "n_uncited": n_uncited,
        })

    # ── concern -> points, BOTH canon homes ──
    policies, pol_errs = _em.load_policies(repo_root)
    for e in pol_errs:
        findings.append(_finding("error", "policies-registry-error", "-", "-", e))
    concern_home: dict[str, str] = {}
    concern_title: dict[str, str] = {}
    for pol in policies.values():
        if pol.get("__pin_mismatch__"):
            continue
        cid = pol.get("concern")
        if cid in _CONCERN_ID_SET:
            concern_home[cid] = "policies.yaml"
            concern_title[cid] = pol.get("title", "")

    concerns_home, con_errs = load_concerns(repo_root)
    for e in con_errs:
        findings.append(_finding("error", "concerns-registry-error", "-", "-", e))
    for row in concerns_home.values():
        cid = row.get("concern")
        if cid in _CONCERN_ID_SET:
            concern_home.setdefault(cid, "concerns.yaml")
            concern_title.setdefault(cid, row.get("title", ""))

    concern_summary: dict[str, dict] = {}
    unbound_concerns: list[str] = []
    for cid in ALL_CONCERN_IDS:
        bound_points = concern_bound.get(cid, [])
        status_counts: dict[str, int] = {}
        for bp in bound_points:
            s = bp.get("status") or "?"
            status_counts[s] = status_counts.get(s, 0) + 1
        home = concern_home.get(cid)
        concern_summary[cid] = {
            "home": home, "title": concern_title.get(cid, ""),
            "n_bound_points": len(bound_points),
            "status_counts": status_counts,
            "bound_points": bound_points,
        }
        if not bound_points:
            unbound_concerns.append(cid)
        if home is None and bound_points:
            # Only a REAL structural problem when some decisionPoint actually cites this
            # concern id — a concern with zero home AND zero bound points is simply outside
            # today's scope (nothing to report), not a finding.
            findings.append(_finding(
                "error", "concern-missing-home", "-", "-",
                f"{cid} is referenced by a registered decisionPoint but is canonized in "
                f"NEITHER policies.yaml nor concerns.yaml"))

    return {
        "n_registries_discovered": len(reg_paths),
        "n_registries_loaded": sum(1 for r in registries if r["loaded"]),
        "registries": registries,
        "concern_summary": concern_summary,
        "unbound_concerns": unbound_concerns,
        "findings": findings,
    }


# ───────────────────────────── rendering ─────────────────────────────

_SEV_GLYPH = {"error": "⛔", "warn": "⚠", "info": "•"}  # ⛔ ⚠ •


def render_census(data: dict, *, as_json: bool = False) -> str:
    if as_json:
        return json.dumps(data, indent=1)

    lines: list[str] = []
    lines.append("DECISION-POINT CENSUS  (plan P3.2 — the seam-registry.yaml derived read-model)")
    lines.append("=" * 72)
    n_disc, n_loaded = data["n_registries_discovered"], data["n_registries_loaded"]
    lines.append(f"Registries: {n_loaded}/{n_disc} loaded"
                  + ("" if n_loaded == n_disc else "  ⚠ one or more registries failed to load — see findings"))
    for r in sorted(data["registries"], key=lambda r: r["path"]):
        if not r["loaded"]:
            lines.append(f"  ⛔ {r['path']}  — FAILED TO LOAD (see findings)")
            continue
        gap_note = "" if r["n_invalid_rows"] == 0 else f" · ⚠ {r['n_invalid_rows']} invalid row(s) dropped"
        lines.append(f"  ✓ {r['crate']:<20} {r['path']:<62} "
                      f"{r['n_points']:>3} pts · {r['n_cited']:>3} cited · {r['n_uncited']:>3} uncited"
                      f"{gap_note}")

    lines.append("")
    lines.append(f"Concern canon ({len(ALL_CONCERN_IDS)} classes) — bound decision points, both homes:")
    for cid in ALL_CONCERN_IDS:
        cs = data["concern_summary"][cid]
        home = cs["home"] or "⛔ NO HOME"
        n = cs["n_bound_points"]
        counts = ", ".join(f"{k}:{v}" for k, v in sorted(cs["status_counts"].items())) or "—"
        tag = "  ⚠ unbound (zero registered points cite it)" if n == 0 else ""
        lines.append(f"  {cid:<4} {home:<14} {n:>2} bound  [{counts}]{tag}")
    if data["unbound_concerns"]:
        lines.append(f"\n  {len(data['unbound_concerns'])} unbound concern(s): "
                      + ", ".join(data["unbound_concerns"])
                      + "  (reported, not failed — several classes are legitimately unbound today)")

    findings = data["findings"]
    errors = [f for f in findings if f["severity"] == "error"]
    warns = [f for f in findings if f["severity"] == "warn"]
    lines.append("")
    lines.append(f"FINDINGS: {len(errors)} error(s), {len(warns)} warn(s)")
    for f in errors + warns:
        glyph = _SEV_GLYPH.get(f["severity"], "?")
        where = f["crate"] if f["crate"] and f["crate"] != "-" else None
        pt = f["point"] if f["point"] and f["point"] != "-" else None
        loc = f"{where}/{pt}: " if where and pt else (f"{where}: " if where else "")
        lines.append(f"  {glyph} [{f['kind']}] {loc}{f['detail']}")
    if not findings:
        lines.append("  none ✅")
    return "\n".join(lines)
