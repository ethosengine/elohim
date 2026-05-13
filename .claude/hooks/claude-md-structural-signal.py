#!/usr/bin/env python3
"""
CLAUDE.md Structural-Signal Accumulator

PostToolUse hook (matcher: Bash). Detects directory-restructuring commands
(`mv`, `git mv`, `cp -r`, `rm -rf`, `mkdir`) and bumps a `structural_edits`
counter on the affected CLAUDE.md scopes.

Structural ops invalidate scope context far more than file edits — when a
directory moves or is copied, the CLAUDE.md describing the old shape may
now describe nothing real. So `structural_edits` weight more heavily in
the drift score than regular `scope_edits`.

Detection is best-effort regex; false-negatives are fine (we'll catch via
direct-edit signals as fallback), false-positives are fine (small bump,
no harm). Complex shell pipelines / scripts may slip past.

Hook Type: PostToolUse
Matcher: Bash
"""
from __future__ import annotations

import json
import math
import os
import re
import sys
import time
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import store as _store  # noqa: E402
from _lib import drift_score as _drift_score  # noqa: E402

# Tunables
# Structural ops rescore every time (they matter); no batching
MAX_WALK_DEPTH = 12
DEFAULT_THRESHOLD = 3.0

# Patterns that detect structural ops. Each captures the operation kind and
# the argument tail. We split on shell separators before matching so each
# sub-command stands alone.
SHELL_SEPARATORS = re.compile(r"&&|\|\||;")
STRUCTURAL_OP_RE = re.compile(
    r"""^\s*
        (?P<op>
            git\s+mv
          | mv
          | cp\s+-[Rr][a-zA-Z]*
          | cp\s+--recursive
          | rm\s+-[a-zA-Z]*[rR][a-zA-Z]*f?[a-zA-Z]*
          | rm\s+-[a-zA-Z]*f[a-zA-Z]*[rR][a-zA-Z]*
          | rmdir
          | mkdir(?:\s+-p)?
        )
        \s+(?P<rest>.+)$
    """,
    re.VERBOSE | re.IGNORECASE,
)

# Strip flags & quotes from path args; ignore obvious non-paths
PATH_ARG_RE = re.compile(r"^[^-][^\s'\"]*$|^['\"]([^'\"]+)['\"]$")


def extract_paths_from_arg_string(rest: str) -> list[str]:
    """Pull plausible filesystem path arguments out of a command's tail."""
    raw_tokens = rest.split()
    paths: list[str] = []
    for tok in raw_tokens:
        if tok.startswith("-"):
            continue  # flag
        # Strip surrounding quotes
        if (tok.startswith('"') and tok.endswith('"')) or (
            tok.startswith("'") and tok.endswith("'")
        ):
            tok = tok[1:-1]
        # Heuristic: contains a path-ish character or matches a name
        if tok and not tok.startswith("$") and not tok.startswith("`"):
            paths.append(tok)
    return paths


def detect_structural_ops(command: str) -> list[tuple[str, list[str]]]:
    """Return list of (op_kind, [paths]) for each structural sub-command."""
    results: list[tuple[str, list[str]]] = []
    subcommands = SHELL_SEPARATORS.split(command)
    for sub in subcommands:
        m = STRUCTURAL_OP_RE.match(sub.strip())
        if not m:
            continue
        op = re.sub(r"\s+", " ", m.group("op").strip())
        paths = extract_paths_from_arg_string(m.group("rest"))
        if not paths:
            continue
        results.append((op, paths))
    return results


def find_enclosing_claude_md_files(path: Path, repo_root: Path) -> list[Path]:
    """Walk up from path's dir (or path if it's a dir), collect CLAUDE.md."""
    results: list[Path] = []
    try:
        # If path doesn't exist, walk up from its parent
        if path.exists() and path.is_dir():
            cur = path.resolve()
        else:
            cur = path.resolve().parent
    except OSError:
        return results
    for _ in range(MAX_WALK_DEPTH):
        candidate = cur / "CLAUDE.md"
        if candidate.is_file():
            results.append(candidate)
        if cur == repo_root or cur.parent == cur:
            break
        cur = cur.parent
    return results


def drift_store_path(repo_root: Path) -> Path:
    return repo_root / ".claude" / "memory-kit" / "claude-md-drift.json"


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    tool_input = data.get("tool_input", {}) or {}
    command = tool_input.get("command") or ""
    if not command:
        return 0

    ops = detect_structural_ops(command)
    if not ops:
        return 0

    repo = repo_root_from_env()
    if not repo:
        return 0

    store_path = drift_store_path(repo)
    default = {"schema_version": 1, "threshold": DEFAULT_THRESHOLD, "files": {}}
    store = _store.load_json(store_path, default=default)
    if not isinstance(store, dict):
        store = default
    store.setdefault("files", {})

    now_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())
    affected_count = 0

    for op, paths in ops:
        for raw in paths:
            # Resolve relative to repo if not absolute
            p = Path(raw)
            if not p.is_absolute():
                p = repo / p
            enclosing = find_enclosing_claude_md_files(p, repo)
            for cm in enclosing:
                try:
                    rel = str(cm.relative_to(repo))
                except ValueError:
                    rel = str(cm)
                entry = store["files"].setdefault(rel, {
                    "last_audited": None,
                    "direct_edits": 0,
                    "scope_edits": 0,
                    "structural_edits": 0,
                    "lines_changed_in_scope": 0,
                    "drift_score": 0.0,
                    "rescore_counter": 0,
                })
                entry["structural_edits"] = entry.get("structural_edits", 0) + 1
                entry["last_signal_at"] = now_iso
                entry["last_structural_op"] = op
                # Structural ops are rare + high-impact → always rescore
                entry["drift_score"] = _drift_score.compute_score(entry)
                entry["rescore_counter"] = 0
                affected_count += 1

    _store.save_json(store_path, store)
    return 0  # best-effort; never block


if __name__ == "__main__":
    sys.exit(main())
