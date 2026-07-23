"""Golden-vector parity runner — the Python evaluator's correspondence-theorem check.

Consumes `elohim/sdk/schemas/v1/registries/governance-parity-vectors.json` VERBATIM (constraint 5:
"parity is the law, vectors are the text") and drives each vector through the REAL resolver decision
path — `_lib.epr_meta.resolve_write()`, the exact pure function the projected
`.claude/hooks/epr-meta-resolver.py` hook calls — never a reimplementation of the cascade/merge/
evaluate/combine/decision-mapping logic. A vector this runner cannot execute is an EXPLICIT skip
with a reason printed to stdout — never a silent green (constraint 5).

Run: python3 .claude/scripts/_lib/__tests__/test_governance_parity.py  (exit 0 = pass)
Bespoke assert-based harness — matches this __tests__ dir's convention (pytest is not installed;
see runtime_harvest_test.py's docstring)."""
import json
import sys
import tempfile
import textwrap
from pathlib import Path

_here = Path(__file__).resolve()
REPO = None
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        REPO = _here
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402

VECTORS_PATH = REPO / "elohim/sdk/schemas/v1/registries/governance-parity-vectors.json"

_passed = 0
_skipped = 0


def check(label: str, cond: bool) -> None:
    global _passed
    assert cond, f"FAIL: {label}"
    _passed += 1
    print(f"  ✅ {label}")


def skip(label: str, reason: str) -> None:
    global _skipped
    _skipped += 1
    print(f"  ⚠️  SKIP: {label} — {reason}")


def materialize(tmp_root: Path, manifests: dict) -> None:
    """Write a vector's `manifests` map (relpath -> raw .epr-meta YAML text) into a fresh temp
    tree, plus a `.git` marker so find_repo_root resolves the temp tree as its own repo (never the
    real one)."""
    (tmp_root / ".git").mkdir(parents=True, exist_ok=True)
    for rel, text in manifests.items():
        p = tmp_root / rel
        p.parent.mkdir(parents=True, exist_ok=True)
        # Vector manifests are bare YAML bodies (no frontmatter fences) — wrap them exactly like
        # every other fixture in this __tests__ dir (see epr_meta_resolver_test.py's _wr()).
        p.write_text("---\nepr-meta-version: 1\n" + textwrap.dedent(text).strip() + "\n---\n")


def run_vector(tmp_root: Path, vector: dict) -> dict:
    """Drive ONE vector through the real resolve_write() path. Returns the {decision, cls,
    rule_id, refer, witnessed} shape the vector's `expect` asserts against."""
    w = vector["write"]
    target = tmp_root / w["path"]
    is_new = bool(w.get("new"))
    content = w.get("content")
    if not is_new and content is not None:
        # An "existing" write needs the pre-image on disk too (mirrors is_new_subdir/require-
        # sibling handling elsewhere) — write it so on-disk-dependent predicates see it.
        target.parent.mkdir(parents=True, exist_ok=True)
        if not target.exists():
            target.write_text(content)
    write = {"path": str(target), "content": content, "is_new": is_new, "is_new_subdir": False}
    root = epr_meta.find_repo_root(target)
    return epr_meta.resolve_write(target, write, root)


def main() -> int:
    if not VECTORS_PATH.is_file():
        print(f"FAIL: golden-vector corpus not found at {VECTORS_PATH}")
        return 1
    corpus = json.loads(VECTORS_PATH.read_text())
    vectors = corpus["vectors"]

    for vector in vectors:
        name = vector["name"]
        expect = vector["expect"]
        with tempfile.TemporaryDirectory() as td:
            tmp_root = Path(td)
            materialize(tmp_root, vector.get("manifests", {}))
            result = run_vector(tmp_root, vector)

        print(f"\n[{name}] {vector.get('law', '')}")
        check(f"{name}: decision == {expect['decision']!r}",
              result["decision"] == expect["decision"])
        check(f"{name}: winning_class == {expect['winning_class']!r}",
              result["cls"] == expect["winning_class"])
        check(f"{name}: rule_id == {expect['rule_id']!r}",
              result["rule_id"] == expect["rule_id"])
        check(f"{name}: witnessed == {expect['witnessed']!r}",
              result["witnessed"] == expect["witnessed"])
        if "refer_reason" in expect:
            got = (result.get("refer") or {}).get("reason")
            check(f"{name}: refer.reason == {expect['refer_reason']!r}",
                  got == expect["refer_reason"])
        if expect.get("directive"):
            check(f"{name}: directive == {expect['directive']!r}",
                  result.get("directive") == expect["directive"])

    # Constraint 5: an EXPLICIT accounting of the whole corpus — nothing silently unexercised.
    print(f"\n  corpus vectors: {len(vectors)}; exercised: {len(vectors) - _skipped}; "
          f"skipped: {_skipped}")

    print(f"\n  {_passed} assertions passed ✅"
          + (f", {_skipped} explicit skip(s)" if _skipped else ""))
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except AssertionError as e:
        print(str(e))
        sys.exit(1)
