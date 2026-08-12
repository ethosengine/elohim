"""Validator-registry parity — is the governance plane ONE evaluator, or two that disagree?

WHY THIS EXISTS. The repo runs two implementations of the same governance evaluation:

  * Python — `_lib.epr_meta.REFERENCE_VALIDATORS`, the LIVE one. It backs the PreToolUse hook
    (`.claude/hooks/epr-meta-resolver.py`, wired at `.claude/settings.json`) and the pre-push
    git gate (`.claude/scripts/epr-meta-git-gate.py`).
  * Rust — `ElohimRepositoryValidators::evaluate`'s match arm in
    `elohim/eprfs/epr-cli/src/repository_validators.rs`, reached via `epr check`.

`governance-parity-vectors.json` pins that the two derive the same decisions from the same
cascades, and both runners execute it. What a vector suite cannot do is speak about validators it
has no vector for — so this test asks the structural question instead: is every validator
reference ACCOUNTED FOR in both hosts?

THE LAW, CORRECTED (2026-08-12). The first version of this test asserted that the two registries
hold the SAME SET. That was the wrong law, and asserting it would have driven exactly the wrong
repair. Some validators are inherently single-host — `eprfs-meta-domain-neutrality` is a claim
about the Rust crate's own source; `bounded-work` walks a seam-registry census that lives on the
Python side. Porting each one to both hosts would produce TWO IMPLEMENTATIONS OF ONE PREDICATE,
which is the fork itself, not the cure.

What must not diverge is the MAP. A two-host plane may hold an honest asymmetry, but only while
both hosts can tell "declared elsewhere, not mine to run" (skip clean, never downgrade the rule)
from "no host claims this" (route to judgment). So the map lives in ONE place —
`elohim/sdk/schemas/v1/registries/governance-validators.json` — and this test enforces that each
host implements exactly what the registry scopes to it, no more and no less. Disagreement becomes
unrepresentable rather than merely tested-for.

WHAT IS STILL RED. Correspondence is not collapse. Two evaluators that agree today can drift
tomorrow, and the Rust one is invoked by no gate at all — so it is parity-tested but never RUN,
and a future divergence has no way to surface in normal use. The habit
`governance-plane-single-evaluator` closes when the decision has ONE evaluator: the Rust
`epr` binary, with the Python hook as a client that falls back to REFER — never permit — when
the binary is absent.

Run: python3 .claude/scripts/_lib/__tests__/validator_registry_parity_test.py  (exit 0 = pass)
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed).
"""
import json
import re
import sys
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent
if REPO is None:  # pragma: no cover - only if the tree is moved
    print("FAIL: could not locate repo root", file=sys.stderr)
    sys.exit(1)

from _lib import epr_meta  # noqa: E402

RUST_PROVIDER = REPO / "elohim/eprfs/epr-cli/src/repository_validators.rs"
SCOPE_REGISTRY = REPO / "elohim/sdk/schemas/v1/registries/governance-validators.json"

_passed = 0
_failures: list[str] = []


def check(label: str, cond: bool, detail: str = "") -> None:
    global _passed
    if cond:
        _passed += 1
        print(f"  ✅ {label}")
    else:
        _failures.append(f"{label}\n{detail}" if detail else label)
        print(f"  ❌ {label}")
        if detail:
            for line in detail.splitlines():
                print(f"       {line}")


# ── the shared scope map ──────────────────────────────────────────────────────
scope: dict[str, str] = {}
if SCOPE_REGISTRY.is_file():
    for entry in json.loads(SCOPE_REGISTRY.read_text()).get("validators", []):
        if isinstance(entry, dict) and entry.get("ref"):
            scope[entry["ref"]] = entry.get("runtime", "?")

# ── what each host actually implements ────────────────────────────────────────
python_impl = set(epr_meta.REFERENCE_VALIDATORS)
rust_text = RUST_PROVIDER.read_text()
# Match-arm literals of the form:  "epr:validator-foo" => foo(request),
rust_impl = set(re.findall(r'"(epr:validator-[a-z0-9-]+)"\s*=>', rust_text))

print("Validator accounting")
print(f"  shared scope registry : {len(scope)}  ({SCOPE_REGISTRY.relative_to(REPO)})")
print(f"  python implements     : {len(python_impl)}")
print(f"  rust implements       : {len(rust_impl)}")
print()

check(
    "the shared scope registry is present and non-empty "
    "(without it neither host can tell a declared absence from an unknown reference)",
    bool(scope),
    f"expected {SCOPE_REGISTRY.relative_to(REPO)} to declare each validator's owning runtime",
)

check(
    "both registries were parsed (a zero here means the extractor broke, not that they agree)",
    bool(python_impl) and bool(rust_impl),
    f"python={len(python_impl)} rust={len(rust_impl)}",
)

# ── no implementation may be undeclared ───────────────────────────────────────
undeclared = sorted((python_impl | rust_impl) - set(scope))
check(
    "every implemented validator is declared in the shared scope registry",
    not undeclared,
    "\n".join(
        [
            "These run in some host but no registry row says who owns them, so the OTHER host",
            "cannot distinguish them from a typo and will route every write they gate:",
            *[f"    - {r}" for r in undeclared],
        ]
    ),
)

# ── every declaration must be honoured by the host it names ───────────────────
missing: list[str] = []
for ref, runtime in sorted(scope.items()):
    if runtime in ("python", "both") and ref not in python_impl:
        missing.append(f"{ref}: declared `{runtime}` but NOT implemented in Python")
    if runtime in ("rust", "both") and ref not in rust_impl:
        missing.append(f"{ref}: declared `{runtime}` but NOT implemented in Rust")
    if runtime == "python" and ref in rust_impl:
        missing.append(f"{ref}: declared `python` but ALSO implemented in Rust (two implementations of one predicate IS the fork)")
    if runtime == "rust" and ref in python_impl:
        missing.append(f"{ref}: declared `rust` but ALSO implemented in Python (two implementations of one predicate IS the fork)")

check(
    "each host implements exactly what the registry scopes to it",
    not missing,
    "\n".join(["The scope map and the implementations disagree:", *[f"    - {m}" for m in missing]]),
)

# ── is the parity-tested twin actually reachable from any gate? ───────────────
gate_files = [
    REPO / ".husky/pre-push",
    REPO / ".husky/pre-commit",
    REPO / ".claude/hooks/epr-meta-resolver.py",
    REPO / ".claude/scripts/epr-meta-git-gate.py",
]
invokers = [
    p.relative_to(REPO).as_posix()
    for p in gate_files
    if p.is_file() and re.search(r"\bepr\s+(check|govern)\b", p.read_text())
]
check(
    "the Rust evaluator is invoked by at least one gate",
    bool(invokers),
    "\n".join(
        [
            "No gate invokes it. The Rust evaluator is parity-tested but never RUN, so a future",
            "divergence from the live Python evaluator cannot surface in normal use — which is",
            "how four measured divergences survived until 2026-08-12.",
            "",
            "This is the remaining leg of `governance-plane-single-evaluator`: collapse to ONE",
            "evaluator by making the Python hook a client of `epr`, falling back to REFER —",
            "never permit — when the binary is absent.",
            f"Searched: {', '.join(p.relative_to(REPO).as_posix() for p in gate_files if p.is_file())}",
        ]
    ),
)

print()
if _failures:
    print(f"  {_passed} passed, {len(_failures)} FAILED ❌")
    print()
    print("  This test is the runnable red behind habits.yaml "
          "`governance-plane-single-evaluator`.")
    sys.exit(1)
print(f"  {_passed} validator-accounting assertions passed ✅")
