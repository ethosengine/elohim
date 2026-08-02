#!/usr/bin/env python3
"""contentHash pin tool for the .epr-meta policy registry (.claude/epr-meta/policies.yaml).

Each registry row can carry a `contentHash: sha256:<hex>` pin over its own canonical JSON body
(sorted keys, compact separators) MINUS the lifecycle/volatile fields (contentHash itself, status,
superseded_by) — the SAME canonicalization `_lib.epr_meta.policy_content_hash()` uses to verify a
loaded row at resolve time (a mismatch routes any bound rule to review: policy-pin-mismatch, never
silently enforces or silently drops possibly-altered semantics).

Covers BOTH sibling registries under `.claude/epr-meta/` (same Precedent row shape, same
canonicalization, different consumers): `policies.yaml` (the edit-gate enforcement registry read by
the resolver hook) and `concerns.yaml` (the concern canon read by the decision-point census + the
SDK canon projection). Pins are written by this tool, never by hand.

Usage:
  epr-meta-pin.py --verify   # exit 0 if every pinned row's stored hash matches; else exit 1 and
                              # list the mismatches (unpinned rows are reported, not failed —
                              # pinning is opt-in per row, not yet mandatory for all policy classes)
  epr-meta-pin.py --write    # recompute + write/refresh contentHash on every row IN PLACE,
                              # preserving key order and the file's surrounding YAML/comments
                              # (line-level surgical edit — never a full re-dump, which would
                              # destroy the registry's hand-authored comments and formatting)
  … --registry policies|concerns|all      # default: all (both siblings)

Fail-closed by design (unlike the resolver): this is an operator/CI tool a human runs deliberately,
not a PreToolUse gate — a bug here should be LOUD, not swallowed.
"""
import re
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta  # noqa: E402

try:
    import yaml
except Exception:  # pragma: no cover
    yaml = None

REGISTRY_REL = epr_meta.POLICY_REGISTRY_REL
CONCERNS_REL = ".claude/epr-meta/concerns.yaml"

# name -> (relative path, top-level version key, top-level rows key). Both siblings carry
# Precedent-shaped rows hashed by the SAME `policy_content_hash` canonicalization; they differ only
# in their consumer (resolver hook vs census/canon projection), which is why they are separate files.
REGISTRIES: dict[str, tuple[str, str, str]] = {
    "policies": (REGISTRY_REL, "epr-meta-policies-version", "policies"),
    "concerns": (CONCERNS_REL, "concern-canon-version", "concerns"),
}

_ROW_START_RE = re.compile(r"^  - id: (\S+)\s*$")
_CONTENT_HASH_RE = re.compile(r"^    contentHash: (\S+)\s*$")
_VERSION_RE = re.compile(r"^    version: (\d+)\s*$")


def _repo_root() -> Path:
    return epr_meta.find_repo_root(Path.cwd())


def _load_rows(root: Path, reg: tuple[str, str, str]) -> list[dict]:
    """One registry's row list, parsed via PyYAML — the same shape load_policies() sees, minus
    the load-time class/actionable-predicate validation (this tool cares only about hashing, so an
    otherwise-invalid row is still hashable/pinnable)."""
    rel, version_key, rows_key = reg
    if yaml is None:
        print("PyYAML unavailable — cannot load the registry", file=sys.stderr)
        sys.exit(1)
    p = root / rel
    if not p.is_file():
        print(f"no registry at {rel}", file=sys.stderr)
        sys.exit(1)
    data = yaml.safe_load(p.read_text()) or {}
    if not isinstance(data, dict) or data.get(version_key) != 1:
        print(f"{rel} missing/invalid `{version_key}`", file=sys.stderr)
        sys.exit(1)
    return data.get(rows_key, []) or []


def verify(root: Path, reg: tuple[str, str, str]) -> int:
    REGISTRY_REL = reg[0]
    rows = _load_rows(root, reg)
    mismatches = []
    unpinned = []
    for pol in rows:
        key = f"{pol.get('id')}@{pol.get('version')}"
        stored = pol.get("contentHash")
        if not stored:
            unpinned.append(key)
            continue
        computed = epr_meta.policy_content_hash(pol)
        if computed != stored:
            mismatches.append((key, stored, computed))
    if unpinned:
        print(f"[epr-meta-pin] {len(unpinned)} unpinned row(s): {', '.join(unpinned)}")
    if mismatches:
        print(f"[epr-meta-pin] {len(mismatches)} MISMATCH(ES):")
        for key, stored, computed in mismatches:
            print(f"  · {key}: pinned {stored}, computed {computed}")
        return 1
    print(f"[epr-meta-pin] {len(rows) - len(unpinned)} pinned row(s) verified clean "
          f"({len(rows)} total).")
    return 0


def write(root: Path, reg: tuple[str, str, str]) -> int:
    """Surgical line-level rewrite: for each `  - id: <x>` row block, insert/update a
    `    contentHash: sha256:<hex>` line immediately after its `    version: <n>` line (matching
    the registry's existing indentation convention). Never a full YAML re-dump — this file carries
    hand-authored prose (`why:` blocks, header comments) that a round-trip dump would mangle."""
    REGISTRY_REL = reg[0]
    rows = _load_rows(root, reg)
    by_key = {f"{pol.get('id')}@{pol.get('version')}": pol for pol in rows}

    p = root / REGISTRY_REL
    lines = p.read_text().splitlines(keepends=True)
    out: list[str] = []
    current_id = None
    changed = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        m = _ROW_START_RE.match(line)
        if m:
            current_id = m.group(1)
        cm = _CONTENT_HASH_RE.match(line)
        if cm and current_id:
            # Replace an existing contentHash line in place (drop it here; re-inserted below the
            # version line on the pass that already emitted it — simplest: strip old, re-add).
            i += 1
            continue
        out.append(line)
        vm = _VERSION_RE.match(line)
        if vm and current_id:
            version = int(vm.group(1))
            key = f"{current_id}@{version}"
            pol = by_key.get(key)
            if pol is not None:
                new_hash = epr_meta.policy_content_hash(pol)
                indent = line[:len(line) - len(line.lstrip(" "))]
                out.append(f"{indent}contentHash: {new_hash}\n")
                changed += 1
        i += 1
    new_text = "".join(out)
    # Normalize to exactly one trailing newline regardless of any pre-existing drift in the
    # source file — a surgical line-edit tool must never accumulate blank lines across runs.
    p.write_text(new_text.rstrip("\n") + "\n")
    print(f"[epr-meta-pin] wrote {changed} contentHash pin(s) to {REGISTRY_REL}.")
    return 0


def _selected(argv: list[str]) -> list[str]:
    """`--registry policies|concerns|all` (default: all). An unknown name is LOUD — this tool is
    fail-closed by design, and a silently-skipped registry is an unpinned registry."""
    if "--registry" not in argv:
        return list(REGISTRIES)
    i = argv.index("--registry")
    name = argv[i + 1] if i + 1 < len(argv) else ""
    if name == "all":
        return list(REGISTRIES)
    if name not in REGISTRIES:
        print(f"unknown --registry `{name}` (expected: {'|'.join(REGISTRIES)}|all)", file=sys.stderr)
        sys.exit(2)
    return [name]


def main(argv: list[str]) -> int:
    root = _repo_root()
    names = _selected(argv)
    # A registry that does not exist yet is skipped with a note, never a hard failure: the
    # concern canon is authored after the policy registry, and `--verify` must stay runnable in a
    # tree where only one sibling has landed. A registry that exists but is unreadable still
    # exits LOUD via _load_rows.
    live = [n for n in names if (root / REGISTRIES[n][0]).is_file()]
    for n in names:
        if n not in live:
            print(f"[epr-meta-pin] {REGISTRIES[n][0]} absent — skipped.")
    if "--verify" in argv:
        return max((verify(root, REGISTRIES[n]) for n in live), default=0)
    if "--write" in argv:
        return max((write(root, REGISTRIES[n]) for n in live), default=0)
    print("usage: epr-meta-pin.py [--verify | --write] [--registry policies|concerns|all]",
          file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
