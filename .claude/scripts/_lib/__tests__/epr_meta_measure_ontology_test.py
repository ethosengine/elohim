"""Measure-tier ontology: a `class: measure` rule must declare `kind: level|rate|ratio` inside
its `measure:` block (the field is POLICY-owned, nested exactly where `measure:` already merges
through policy bindings — see `_BINDING_KEYS` / `expand_policies` in _lib/epr_meta.py), and
`kind: rate` must carry `per:`. Wire parity with the Rust `MeasureKind` serde vocabulary
(elohim/epr/src/measure.rs) is law — the last test reads the real serde config, not a copied
constant, so it cannot pass on a drifted wire name.

The `validate_meta` tests below cover INLINE manifest rules. `load_policies` is a SEPARATE
gate covering the `.claude/epr-meta/policies.yaml` REGISTRY — the path that actually feeds the
evaluator (resolve() -> merge_rules -> load_policies -> expand_policies -> evaluate). A manifest
rule that BINDS a registry policy (`policy: <id>@<version>`) carries no `class`/`measure` of its
own, so `validate_meta`'s `cls == "measure"` branch never sees it; only `load_policies`'s own
measure-class check protects the registry path. The `test_registry_*` tests below cover that
gate directly, built the way `load_policies` actually consumes it (a real
`.claude/epr-meta/policies.yaml` file under a repo root, read via `load_policies(repo_root)`).

Run: python3 -m pytest .claude/scripts/_lib/__tests__/epr_meta_measure_ontology_test.py -v
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (here / ".claude" / "scripts" / "_lib").is_dir():
        REPO = here
        sys.path.insert(0, str(here / ".claude" / "scripts"))
        break
    here = here.parent
assert REPO is not None, "could not locate repo root (.claude/scripts/_lib) from test file"

from _lib import epr_meta as _epr_meta  # noqa: E402
from _lib.epr_meta import validate_meta  # noqa: E402


def _registry_repo(tmp: Path, registry_yaml: str) -> Path:
    """Write `registry_yaml` to `<tmp>/.claude/epr-meta/policies.yaml` — the exact path
    `load_policies(repo_root)` reads (`POLICY_REGISTRY_REL`), mirroring the `_repo()` helper in
    epr_meta_policy_test.py so the registry-path tests build their fixture the established way."""
    reg = tmp / _epr_meta.POLICY_REGISTRY_REL
    reg.parent.mkdir(parents=True, exist_ok=True)
    reg.write_text(registry_yaml)
    return tmp


def test_measure_rule_without_kind_is_refused():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "loc", "class": "measure", "when": {"write": "*.rs"}}]}
    errs = validate_meta(meta)
    assert any("kind" in e for e in errs), f"expected a kind advisory, got {errs}"


def test_rate_without_period_is_refused():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "churn", "class": "measure", "measure": {"kind": "rate"},
                       "when": {"write": "*.rs"}}]}
    errs = validate_meta(meta)
    assert any("per" in e for e in errs), f"expected a period advisory, got {errs}"


def test_unknown_kind_is_refused():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "loc", "class": "measure", "measure": {"kind": "velocity"},
                       "when": {"write": "*.rs"}}]}
    errs = validate_meta(meta)
    assert any("velocity" in e for e in errs), f"expected an unknown-kind advisory, got {errs}"


def test_level_with_kind_is_accepted():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "loc", "class": "measure", "measure": {"kind": "level"},
                       "when": {"write": "*.rs"}}]}
    assert validate_meta(meta) == []


def test_rate_with_period_is_accepted():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "churn", "class": "measure",
                       "measure": {"kind": "rate", "per": "week"},
                       "when": {"write": "*.rs"}}]}
    assert validate_meta(meta) == []


def test_ratio_is_accepted():
    meta = {"epr-meta-version": 1,
            "rules": [{"id": "coverage", "class": "measure", "measure": {"kind": "ratio"},
                       "when": {"write": "*.rs"}}]}
    assert validate_meta(meta) == []


def test_kind_vocabulary_matches_the_rust_serde_names():
    # Wire parity is law. Read the REAL serde configuration off `MeasureKind` — not a copied
    # constant and not doc-comment prose (a `f'"{name}"' in src or name in src` check would pass
    # even if the wire names drifted, since the plain english word still appears in comments).
    src = (REPO / "elohim/epr/src/measure.rs").read_text()
    m = re.search(
        r'#\[serde\(tag\s*=\s*"kind",\s*rename_all\s*=\s*"lowercase"\)\]\s*'
        r'pub enum MeasureKind\s*\{(.*?)\n\}',
        src, re.DOTALL,
    )
    assert m, (
        "MeasureKind must stay #[serde(tag = \"kind\", rename_all = \"lowercase\")] — that "
        "attribute is what makes the wire discriminant field literally `kind` and each variant's "
        "wire value its lowercased name, matching the YAML `kind:` vocabulary structurally rather "
        "than by convention."
    )
    body = m.group(1)
    variant_names = set(re.findall(r'^\s*(\w+)\s*[,{]', body, re.MULTILINE))
    assert variant_names, f"could not parse any variants out of MeasureKind body: {body!r}"
    # `rename_all = "lowercase"` IS the transform under test: the wire value is the variant name
    # lowercased, so deriving wire_names this way (not hand-copying "level"/"rate"/"ratio") is
    # what makes this assertion fail if the Rust enum's variant set ever changes.
    wire_names = {v.lower() for v in variant_names}
    assert wire_names == {"level", "rate", "ratio"}, (
        f"Rust MeasureKind wire vocabulary {sorted(wire_names)} != YAML `kind:` vocabulary "
        f"{{'level', 'rate', 'ratio'}} — wire parity broken"
    )


# ── Registry path (load_policies) — the gate `validate_meta` cannot reach, because a
# `policy:`-binding manifest rule carries no `class`/`measure` of its own. ──

def test_registry_measure_policy_without_kind_is_refused(tmp_path):
    _registry_repo(tmp_path, """\
epr-meta-policies-version: 1
policies:
  - id: no-kind-ceiling
    version: 1
    class: measure
    scope: { write: "*.rs" }
    measure: { loc-soft: 10, loc-hard: 20 }
    why: too big.
""")
    pols, errs = _epr_meta.load_policies(tmp_path)
    assert "no-kind-ceiling@1" not in pols, "a kind-less measure policy must NOT load"
    assert any("kind" in e for e in errs), f"expected a kind advisory, got {errs}"


def test_registry_measure_policy_rate_without_period_is_refused(tmp_path):
    _registry_repo(tmp_path, """\
epr-meta-policies-version: 1
policies:
  - id: churn-rate
    version: 1
    class: measure
    scope: { write: "*.rs" }
    measure: { kind: rate, loc-soft: 10, loc-hard: 20 }
    why: churn.
""")
    pols, errs = _epr_meta.load_policies(tmp_path)
    assert "churn-rate@1" not in pols, "a rate measure policy with no per: must NOT load"
    assert any("per" in e for e in errs), f"expected a period advisory, got {errs}"


def test_registry_measure_policy_well_formed_loads(tmp_path):
    _registry_repo(tmp_path, """\
epr-meta-policies-version: 1
policies:
  - id: loc-ceiling
    version: 1
    class: measure
    scope: { write: "*.rs" }
    measure: { kind: level, loc-soft: 10, loc-hard: 20 }
    why: too big.
""")
    pols, errs = _epr_meta.load_policies(tmp_path)
    assert errs == []
    assert "loc-ceiling@1" in pols
    assert pols["loc-ceiling@1"]["measure"]["kind"] == "level"
