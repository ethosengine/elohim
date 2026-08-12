"""Validator-registry parity — is the governance plane ONE evaluator, or two that disagree?

WHY THIS EXISTS. The repo runs two implementations of the same governance evaluation:

  * Python — `_lib.epr_meta.REFERENCE_VALIDATORS`, the LIVE one. It backs the PreToolUse hook
    (`.claude/hooks/epr-meta-resolver.py`, wired at `.claude/settings.json`) and the pre-push
    git gate (`.claude/scripts/epr-meta-git-gate.py`).
  * Rust — `ElohimRepositoryValidators::evaluate`'s match arm in
    `elohim/eprfs/epr-cli/src/repository_validators.rs`, reached via `epr check`.

`governance-parity-vectors.json` already pins that the two agree on nine golden vectors, and
`test_governance_parity.py` runs them. What nothing checked is whether the two registries contain
the SAME VALIDATORS — and they do not. A vector suite cannot catch a divergence in the set of
things it does not have vectors for; the correspondence theorem is asserted over the intersection
and silent on the difference.

The divergence is not cosmetic. A validator present in one registry and absent in the other
returns `Unavailable`/no-op there, so the SAME manifest yields different decision classes
depending on which evaluator ran — in BOTH severity directions at once (Python enforces three
rules Rust cannot see; Rust enforces one Python cannot see). Meanwhile `epr check` is invoked by
no gate at all, so the parity-tested twin never actually runs and the divergence has no way to
surface in normal use.

THIS TEST IS EXPECTED TO FAIL until the governance plane is collapsed to one evaluator. It is the
runnable check behind the `governance-plane-single-evaluator` habit in `genesis/manifests/
habits.yaml` — filed `red` per covenant rule 2 (a red needs a check that EXISTS and fails; an
unwired habit is one with no check at all). Its failure message is the work item.

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
VECTORS = REPO / "elohim/sdk/schemas/v1/registries/governance-parity-vectors.json"

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


# ── the two registries ────────────────────────────────────────────────────────
python_refs = set(epr_meta.REFERENCE_VALIDATORS)

rust_text = RUST_PROVIDER.read_text()
# Match-arm literals of the form:  "epr:validator-foo" => foo(request),
rust_refs = set(re.findall(r'"(epr:validator-[a-z0-9-]+)"\s*=>', rust_text))

print("Validator registries")
print(f"  python : {len(python_refs)}  ({RUST_PROVIDER.name}'s sibling, _lib/epr_meta.py)")
print(f"  rust   : {len(rust_refs)}  ({RUST_PROVIDER.relative_to(REPO)})")
print()

check(
    "both registries were parsed (a zero here means the extractor broke, not that they agree)",
    bool(python_refs) and bool(rust_refs),
    f"python={len(python_refs)} rust={len(rust_refs)}",
)

python_only = sorted(python_refs - rust_refs)
rust_only = sorted(rust_refs - python_refs)

check(
    "ONE evaluator: the Python and Rust validator registries hold the same set",
    python_refs == rust_refs,
    "\n".join(
        [
            "The governance plane is forked. The same manifest yields different decision",
            "classes depending on which evaluator ran, in both severity directions:",
            "",
            f"  python-only ({len(python_only)}) — Rust returns Unavailable, so these go UNENFORCED",
            f"  under `epr check`:",
            *[f"    - {r}" for r in python_only],
            "",
            f"  rust-only ({len(rust_only)}) — the LIVE Python hook cannot see these at all:",
            *[f"    - {r}" for r in rust_only],
            "",
            "Fix = collapse to one evaluator (Rust via `epr check --json`), with the Python hook",
            "as a client that falls back to REFER — never permit — when the binary is absent.",
        ]
    ),
)

# ── do the golden vectors even cover the divergence? ──────────────────────────
covered: set[str] = set()
if VECTORS.is_file():
    raw = json.loads(VECTORS.read_text())
    blob = json.dumps(raw)
    for ref in python_refs | rust_refs:
        if ref in blob:
            covered.add(ref)

divergent = set(python_only) | set(rust_only)
uncovered_divergent = sorted(divergent - covered)

check(
    "the golden parity vectors cover every DIVERGENT validator "
    "(a vector suite is silent on what it has no vector for)",
    not uncovered_divergent,
    "\n".join(
        [
            f"{len(divergent)} validators differ between the registries; "
            f"{len(uncovered_divergent)} of them appear in NO golden vector:",
            *[f"    - {r}" for r in uncovered_divergent],
            "",
            "This is why governance-parity-vectors.json reads green over a forked plane:",
            "the correspondence theorem is asserted over the intersection only.",
        ]
    ),
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
    if p.is_file() and re.search(r"\bepr\s+check\b", p.read_text())
]
check(
    "`epr check` (the Rust evaluator) is invoked by at least one gate",
    bool(invokers),
    "\n".join(
        [
            "No gate invokes it. The Rust evaluator is parity-tested but never RUN, so a",
            "divergence from the live Python evaluator cannot surface in normal use.",
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
print(f"  {_passed} validator-registry parity assertions passed ✅")
