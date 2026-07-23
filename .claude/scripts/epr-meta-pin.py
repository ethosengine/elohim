#!/usr/bin/env python3
"""contentHash pin tool for the .epr-meta policy registry (.claude/epr-meta/policies.yaml).

Each registry row can carry a `contentHash: sha256:<hex>` pin over its own canonical JSON body
(sorted keys, compact separators) MINUS the lifecycle/volatile fields (contentHash itself, status,
superseded_by) — the SAME canonicalization `_lib.epr_meta.policy_content_hash()` uses to verify a
loaded row at resolve time (a mismatch routes any bound rule to review: policy-pin-mismatch, never
silently enforces or silently drops possibly-altered semantics).

Usage:
  epr-meta-pin.py --verify   # exit 0 if every pinned row's stored hash matches; else exit 1 and
                              # list the mismatches (unpinned rows are reported, not failed —
                              # pinning is opt-in per row, not yet mandatory for all policy classes)
  epr-meta-pin.py --write    # recompute + write/refresh contentHash on every row IN PLACE,
                              # preserving key order and the file's surrounding YAML/comments
                              # (line-level surgical edit — never a full re-dump, which would
                              # destroy the registry's hand-authored comments and formatting)

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
_ROW_START_RE = re.compile(r"^  - id: (\S+)\s*$")
_CONTENT_HASH_RE = re.compile(r"^    contentHash: (\S+)\s*$")
_VERSION_RE = re.compile(r"^    version: (\d+)\s*$")


def _repo_root() -> Path:
    return epr_meta.find_repo_root(Path.cwd())


def _load_rows(root: Path) -> list[dict]:
    """The registry's policy list, parsed via PyYAML — the same shape load_policies() sees, minus
    the load-time class/actionable-predicate validation (this tool cares only about hashing, so an
    otherwise-invalid row is still hashable/pinnable)."""
    if yaml is None:
        print("PyYAML unavailable — cannot load the policy registry", file=sys.stderr)
        sys.exit(1)
    p = root / REGISTRY_REL
    if not p.is_file():
        print(f"no registry at {REGISTRY_REL}", file=sys.stderr)
        sys.exit(1)
    data = yaml.safe_load(p.read_text()) or {}
    if not isinstance(data, dict) or data.get("epr-meta-policies-version") != 1:
        print(f"{REGISTRY_REL} missing/invalid `epr-meta-policies-version`", file=sys.stderr)
        sys.exit(1)
    return data.get("policies", []) or []


def verify(root: Path) -> int:
    rows = _load_rows(root)
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


def write(root: Path) -> int:
    """Surgical line-level rewrite: for each `  - id: <x>` row block, insert/update a
    `    contentHash: sha256:<hex>` line immediately after its `    version: <n>` line (matching
    the registry's existing indentation convention). Never a full YAML re-dump — this file carries
    hand-authored prose (`why:` blocks, header comments) that a round-trip dump would mangle."""
    rows = _load_rows(root)
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


def main(argv: list[str]) -> int:
    root = _repo_root()
    if "--verify" in argv:
        return verify(root)
    if "--write" in argv:
        return write(root)
    print("usage: epr-meta-pin.py [--verify | --write]", file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
