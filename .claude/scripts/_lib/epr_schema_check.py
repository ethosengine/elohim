"""Stdlib-only JSON-Schema-subset validator — the HOOK-TIER PROJECTION.

AUTHORITY: `elohim/sdk/schemas/v1/registries/*.schema.json`. This file is a projection of
that authority, never a second one. Two rails keep it honest:
  * `pnpm run schema:check-python` (pre-push/CI) revalidates the same corpus with the real
    `jsonschema` lib and fails on ANY disagreement — the correspondence gate;
  * golden vectors in `_lib/__tests__/epr_schema_golden/` are run by both tiers.
Subset supported (all our cards use): type/required/properties/enum/const/pattern/
additionalProperties/items/minItems/minimum/oneOf-by-const. Zero deps: the hook must
never fail to START, and a missing wheel that disables a gate is silent-allow.
"""
from __future__ import annotations

import hashlib
import json
import re
from pathlib import Path

_TYPES = {"object": dict, "array": list, "string": str, "boolean": bool,
          "integer": int, "number": (int, float), "null": type(None)}


def validate(inst, schema: dict, at: str = "$") -> list[str]:
    """[] == valid. Messages are AUTHOR-FACING: they name the path and the fix."""
    errs: list[str] = []
    t = schema.get("type")
    if t and not isinstance(inst, _TYPES.get(t, object)):
        return [f"{at}: expected {t}, got {type(inst).__name__}"]
    if t == "integer" and isinstance(inst, bool):
        return [f"{at}: expected integer, got boolean"]
    if "const" in schema and inst != schema["const"]:
        errs.append(f"{at}: must be {schema['const']!r}")
    if "enum" in schema and inst not in schema["enum"]:
        errs.append(f"{at}: {inst!r} not one of {schema['enum']}")
    if isinstance(inst, str):
        pat = schema.get("pattern")
        if pat and not re.match(pat, inst):
            errs.append(f"{at}: {inst!r} does not match /{pat}/")
    if isinstance(inst, (int, float)) and "minimum" in schema and inst < schema["minimum"]:
        errs.append(f"{at}: {inst} < minimum {schema['minimum']}")
    if isinstance(inst, dict):
        props = schema.get("properties", {})
        for k in schema.get("required", []):
            if k not in inst:
                errs.append(f"{at}.{k}: required field missing")
        if schema.get("additionalProperties") is False:
            for k in sorted(set(inst) - set(props)):
                errs.append(f"{at}.{k}: unknown field (typo? closed vocabulary)")
        for k, sub in props.items():
            if k in inst:
                errs += validate(inst[k], sub, f"{at}.{k}")
    if isinstance(inst, list):
        if len(inst) < schema.get("minItems", 0):
            errs.append(f"{at}: needs >= {schema['minItems']} item(s)")
        item = schema.get("items")
        if item:
            for i, v in enumerate(inst):
                errs += validate(v, item, f"{at}[{i}]")
    return errs


def load_schema(root: Path, name: str) -> dict:
    p = Path(root) / "elohim/sdk/schemas/v1/registries" / f"{name}.schema.json"
    return json.loads(p.read_text())


# ── contentHash: what turns `id@version` from a NAME into a PIN ───────────────────────
def content_hash(row: dict) -> str:
    """Canonical hash of a registry row with its own `contentHash` removed. Same discipline
    as `canonical_bytes` on the Rust side: sorted keys, no insignificant whitespace."""
    body = {k: v for k, v in row.items() if k != "contentHash"}
    canon = json.dumps(body, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    return "sha256:" + hashlib.sha256(canon.encode()).hexdigest()


def check_policy_row(row: dict, root: Path) -> list[str]:
    """A registry row is valid iff it is schema-shaped AND self-pinned. Without the
    self-pin, `policy: foo@1` names a MUTABLE row: editing the body in place silently
    re-semantics every binding — which version applies must be a DECLARED dependency."""
    errs = validate(row, load_schema(root, "policy-registry-row"), f"policies[{row.get('id','?')}]")
    declared = row.get("contentHash")
    if declared and declared != (actual := content_hash(row)):
        errs.append(f"policy `{row.get('id')}@{row.get('version')}`: contentHash is stale "
                    f"(declared {declared[:14]}…, actual {actual[:14]}…). Editing a policy = "
                    f"a NEW version row; never mutate a version that has bindings.")
    return errs


def check_axis_card(card: dict, root: Path) -> list[str]:
    return validate(card, load_schema(root, "axis-card"), f"axis[{card.get('axis','?')}]")


def check_epr_meta(cfg: dict, root: Path) -> list[str]:
    """Schema-shape half of `.epr-meta` validation. The FOOTGUN half (an enforcing rule with
    no actionable predicate, >1 predicate, duplicate ids) stays in `epr_meta.validate_meta`:
    those are cross-field pragmatics no schema language expresses, and their messages are
    better than any generated one."""
    return validate(cfg, load_schema(root, "epr-meta-manifest"), ".epr-meta")
