#!/usr/bin/env python3
"""
CLAUDE.md Drift-Signal Accumulator

PostToolUse hook (matcher: Edit|Write). Fast accumulator — runs after every
edit, increments a drift counter for each enclosing CLAUDE.md scope. Costs
single-digit ms in the cheap path; offloads all judgment to `claude-md-audit.py`
which only runs when the operator invokes the ceremony.

Layered compute (trust-compute gradient):
  - Cheap path (every edit): walk up dirs, increment scope_edits counter
  - Medium path (every N edits, configurable): re-compute drift score
  - Expensive path: deferred entirely to claude-md-audit.py

When a CLAUDE.md's drift_score crosses threshold, the next session's
SessionStart hook surfaces it; until then, gospel stands.

Storage: ONE thing — a fold via `epr flow note --kind observation --measure
claude-md-edit-signal@1`. The private JSON accumulator under `.claude/memory-kit/` was deleted
with the kit at station six round (b) (2026-09-11); per-scope edit counts and the drift score
derived from them are read from the fold plane by `epr flow report`, never carried here.
The JSON is not a fallback: cleanup-pressure.py counts its `files` collection, so the kit
still produces the accumulated number the SessionStart bridge folds. Station six deletes it.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

from __future__ import annotations

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when the gospel-tier surfaces are audited on a cadence driven by substrate landings rather "
    "than by edit-count pressure — the counter approximates 'enough has changed to re-read "
    "this', and a real trigger retires the approximation."
)

import json
import math
import os
import sys
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))
import _observation as _obs  # noqa: E402  (structured-observation emitter; fail-open, never blocks)

# Tunable specific to this hook. The score formula and its threshold are NOT here any more:
# `_lib.drift_score.compute_score` is fed by the fold plane through `epr flow report`, and the
# threshold is DECLARED as claude-md-drift-score@1's ceiling in .claude/epr-meta/measures.yaml.
# A hook that folds one event per edit does not need a rescore cadence or a private score.
MAX_WALK_DEPTH = 12        # stop walking up after this many dirs


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def find_enclosing_claude_md_files(edited_file: Path, repo_root: Path) -> list[Path]:
    """Walk up from edited_file's dir, collect every CLAUDE.md until repo_root.

    A file deep in the tree counts against every enclosing CLAUDE.md scope.
    """
    results: list[Path] = []
    try:
        cur = edited_file.resolve().parent
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




# compute_score moved to _lib.drift_score (shared with audit + structural hook)


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    tool_input = data.get("tool_input", {}) or {}
    edited = tool_input.get("file_path") or ""
    if not edited:
        return 0

    repo = repo_root_from_env()
    if not repo:
        return 0

    edited_path = Path(edited)
    if not edited_path.is_absolute():
        edited_path = repo / edited_path

    # Find enclosing CLAUDE.md files (could be 0..N)
    enclosing = find_enclosing_claude_md_files(edited_path, repo)
    if not enclosing:
        return 0

    is_direct_edit_target = edited_path.name == "CLAUDE.md"

    # The bound lives in .claude/epr-meta (claude-md-edit-signal@1 feeding the derived
    # claude-md-drift-score@1 ceiling). One observation per enclosing scope; the drift score
    # is derived from the folds by the native report, not carried in a private counter here.
    if _obs.available():
        for claude_md in enclosing:
            try:
                rel = str(claude_md.relative_to(repo))
            except ValueError:
                rel = str(claude_md)
            kind = "direct" if (is_direct_edit_target and edited_path == claude_md) else "scope"
            _obs.emit("claude-md-edit-signal@1", rel, 1,
                      reason=f"gospel drift: {kind} edit inside this CLAUDE.md scope",
                      env={"kind": kind}, root=str(repo))

    return 0  # hooks are best-effort; never block the tool call


if __name__ == "__main__":
    sys.exit(main())
