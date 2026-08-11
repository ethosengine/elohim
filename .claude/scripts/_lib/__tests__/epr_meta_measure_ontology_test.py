"""Measure-tier ontology: a `class: measure` rule must declare `kind: level|rate|ratio` inside
its `measure:` block (the field is POLICY-owned, nested exactly where `measure:` already merges
through policy bindings — see `_BINDING_KEYS` / `expand_policies` in _lib/epr_meta.py), and
`kind: rate` must carry `per:`. Wire parity with the Rust `MeasureKind` serde vocabulary
(elohim/epr/src/measure.rs) is law — the last test reads the real serde config, not a copied
constant, so it cannot pass on a drifted wire name.

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

from _lib.epr_meta import validate_meta  # noqa: E402


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
