#!/usr/bin/env python3
"""Regression contract for the repo-wide brand-vocabulary compose-gate advisory.

Run: python3 .claude/scripts/_lib/__tests__/brand_vocabulary_guard_test.py
"""
import sys
import tempfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[2]))
from _lib import epr_meta  # noqa: E402

ROOT = epr_meta.find_repo_root(Path(__file__).resolve())
POLICIES, POLICY_ERRORS = epr_meta.load_policies(ROOT)
POLICY_REF = "brand-vocabulary-boundary@1"
RULE_ID = "brand-vocabulary-boundary"
GUARD = epr_meta.REFERENCE_VALIDATORS["epr:validator-brand-vocabulary-boundary"]

_failures: list[str] = []
_passed = 0


def check(label: str, condition: bool, detail: str = "") -> None:
    global _passed
    if condition:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(label)
        print(f"  ❌ {label}{(' — ' + detail) if detail else ''}")


def bound_rule() -> dict:
    merged = {
        "rules": {RULE_ID: {"id": RULE_ID, "policy": POLICY_REF}},
        "validators": {},
        "sources": [str(ROOT)],
    }
    errors = epr_meta.expand_policies(merged, POLICIES)
    if errors:
        _failures.append(f"expand policy: {errors}")
    return merged


def verdict(merged: dict, write: dict):
    return epr_meta.combine(epr_meta.evaluate(merged, write))


def tmp_write(tmp: Path, rel: str, pre: str | None, post: str) -> dict:
    target = tmp / rel
    target.parent.mkdir(parents=True, exist_ok=True)
    if pre is not None:
        target.write_text(pre)
    return {"path": str(target), "content": post, "is_new": pre is None}


check("policy registry loads without errors", not POLICY_ERRORS, str(POLICY_ERRORS))
policy = POLICIES.get(POLICY_REF) or {}
check(
    "policy is an active inject advisory backed by the registered validator",
    policy.get("class") == "inject"
    and policy.get("status") == "active"
    and policy.get("validator") == "epr:validator-brand-vocabulary-boundary"
    and "epr:validator-brand-vocabulary-boundary" in epr_meta.REFERENCE_VALIDATORS,
    str(policy),
)
check(
    "repo root binds the pinned policy",
    "policy: brand-vocabulary-boundary@1" in (ROOT / ".epr-meta/manifest.md").read_text(),
)
root_probe = epr_meta.resolve_write(
    ROOT / "brand-vocabulary-probe.ts",
    {
        "path": str(ROOT / "brand-vocabulary-probe.ts"),
        "content": "export class MishpatSignal {}\n",
        "is_new": True,
    },
    ROOT,
)
check(
    "real root cascade permits with an inject advisory",
    root_probe.get("decision") == "permit"
    and root_probe.get("cls") == "inject"
    and root_probe.get("rule_id") == RULE_ID,
    str(root_probe),
)

with tempfile.TemporaryDirectory() as td:
    tmp = Path(td)
    (tmp / ".git").mkdir()
    merged = bound_rule()

    # Every opaque domain brand is covered, including PascalCase and identifier suffixes.
    brand_symbols = {
        "imagodei": "ImagodeiIdentityClient",
        "lamad": "LamadLearningClient",
        "avodah": "AvodahWorkService",
        "qahal": "QahalCollectiveStore",
        "shefa": "ShefaEconomicEvent",
        "mishpat": "MishpatSignal",
    }
    for term, symbol in brand_symbols.items():
        v = verdict(merged, tmp_write(tmp, f"src/{term}.ts", None, f"export class {symbol} {{}}\n"))
        check(
            f"net-new {term} symbol fires at inject",
            v is not None and v.cls == "inject" and v.rule_id == RULE_ID,
            f"got {v}",
        )

    # A coded route/string literal is part of the contract even when its surrounding service is
    # semantically named.
    route = "const path = '/api/v1/mishpat/recognition/participation';\n"
    v = verdict(merged, tmp_write(tmp, "src/governance-api.ts", None, route))
    check("brand literal in a route fires", v is not None and v.cls == "inject", f"got {v}")

    # Product/architecture prose is explicitly outside the code lint.
    prose = "# Mishpat\n\nMishpat is the branded judgment substrate.\n"
    v = verdict(merged, tmp_write(tmp, "docs/governance.md", None, prose))
    check("Markdown brand prose stays silent", v is None, f"got {v}")

    # Comments and schema prose are not compilable references.
    comment = "/// Mishpat is the architecture project name.\npub enum GovernanceSignal {}\n"
    v = verdict(merged, tmp_write(tmp, "src/signals.rs", None, comment))
    check("source comments stay silent", v is None, f"got {v}")

    schema = "{\n  \"description\": \"Mishpat project provenance.\",\n  \"title\": \"CommitmentView\"\n}\n"
    v = verdict(merged, tmp_write(tmp, "schemas/commitment.schema.json", None, schema))
    check("JSON Schema description prose stays silent", v is None, f"got {v}")

    package_body = f'{{\n  "body": "# Project {term} reference\\n"\n}}\n'
    v = verdict(
        merged,
        tmp_write(tmp, ".epr-meta/elohim/packages/skills/reference.json", None, package_body),
    )
    check("agent-package documentation body stays silent", v is None, f"got {v}")

    generated_report = f'{{"last_changed": "src/{term}.rs"}}\n'
    v = verdict(
        merged,
        tmp_write(tmp, ".claude/memory-kit/generated-report.json", None, generated_report),
    )
    check("generated memory-kit observation stays silent", v is None, f"got {v}")

    # The validator and its fixtures enumerate the vocabulary by necessity, so they are exempt —
    # whether the hook hands the write an absolute path or a repo-relative one.
    self_source = 'MISHPAT_ROLE_NAME = "mishpat"\n'
    for rel in (
        ".claude/scripts/_lib/epr_meta.py",
        ".claude/scripts/_lib/__tests__/brand_vocabulary_guard_test.py",
        "elohim/eprfs/epr-cli/src/repository_validators.rs",
    ):
        for label, path in (("repo-relative", rel), ("absolute", str(ROOT / rel))):
            fired = GUARD({"path": path, "content": self_source, "is_new": True})
            check(f"{label} {rel} is self-exempt", not fired, f"fired on {path}")

    # Internal compatibility is not an exception while the protocol is still in development.
    marked = (
        "// brand-boundary: mishpat — stable Holochain role identifier\n"
        "const GOVERNANCE_ROLE_NAME: &str = \"mishpat\";\n"
    )
    v = verdict(merged, tmp_write(tmp, "src/role.rs", None, marked))
    check("compatibility marker does not suppress coded vocabulary", v is not None and v.cls == "inject", f"got {v}")

    # Existing debt does not toll unrelated maintenance; only newly introduced lines are inspected.
    legacy = "pub enum MishpatSignal {}\n"
    maintained = legacy + "pub enum GovernanceDecision {}\n"
    v = verdict(merged, tmp_write(tmp, "src/legacy.rs", legacy, maintained))
    check("untouched legacy vocabulary stays silent during semantic maintenance", v is None, f"got {v}")

    # Elohim is the protocol name, and doorway is ordinary descriptive language; neither is an
    # opaque domain-brand lint target.
    allowed = "export class ElohimDoorwayClient {}\n"
    v = verdict(merged, tmp_write(tmp, "src/protocol.ts", None, allowed))
    check("protocol and ordinary descriptive names stay silent", v is None, f"got {v}")

if _failures:
    print(f"\n{len(_failures)} failure(s): {', '.join(_failures)}")
    raise SystemExit(1)

print(f"\n{_passed} checks passed ✅")
