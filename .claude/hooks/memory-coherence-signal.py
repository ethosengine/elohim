#!/usr/bin/env python3
"""
Memory-coherence Signal Accumulator

PostToolUse hook (matcher: Edit|Write). Fast accumulator — when an edited file
matches a memory entry's declared `cites:` glob, bump that entry's
coherence-drift counter. The librarian surfaces accumulated counts during
`/hygiene-sweep` ("N memory entries cite code that changed since last
verified") and resets them. Judgment is deferred to the sweep; this hot path
only counts.

Design (trust-compute gradient, mirrors claude-md-drift-signal.py):
  - Loads the cached cites-index (.eprfs/status/lenses/cites-index.json) built by
    the relocated memory-coherence-audit lens
    (.epr-meta/elohim/lenses/memory/memory-coherence-audit.py). Absent index → no-op
    (graceful degradation until the first audit runs).
  - fnmatch the one edited path against the index globs; bump matched entries.
  - Best-effort, never blocks. Cost: load one small JSON + fnmatch one path.

Storage: ONE thing — a fold via `epr flow note --kind observation --measure
memory-coherence-drift@1`. The private JSON accumulator under `.claude/memory-kit/` was deleted
with the kit at station six round (b) (2026-09-11); the accumulated count is DERIVED from the
folds by cleanup-pressure-ceiling@1's `derive: distinct-subjects-since-reset`.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

from __future__ import annotations

# The intervenor's removal condition (Meadows' shifting-the-burden trap;
# counted by _lib/intervenor_census.py). A condition, never a date.
RETIRE_WHEN = (
    "when a memory entry's coherence with the code it cites is verified at read time (recall "
    "checks its own citations) rather than accumulated as drift between sweeps — the signal is "
    "a proxy for a check that happens too late."
)

import fnmatch
import json
import os
import sys
from pathlib import Path

# Bootstrap _lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import store as _store  # noqa: E402
from _lib import paths as _paths  # noqa: E402
sys.path.insert(0, str(Path(__file__).resolve().parent))
import _observation as _obs  # noqa: E402  (structured-observation emitter; JSON fallback while absent)


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def index_path(repo_root: Path) -> Path:
    """Where the memory-coherence-audit lens writes its cites index.

    ONE authority: `_lib.paths.reports_root` — the same function the lens resolves its own
    output through, so the reader cannot drift from the writer.
    """
    return _paths.reports_root(repo_root) / "cites-index.json"


def changed_matches_cite(changed: str, pattern: str) -> bool:
    changed = changed.strip().lstrip("./")
    pattern = pattern.strip().lstrip("./")
    if not changed or not pattern:
        return False
    if any(c in pattern for c in "*?["):
        return fnmatch.fnmatch(changed, pattern)
    if changed == pattern.rstrip("/"):
        return True
    return changed.startswith(pattern.rstrip("/") + "/")


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    edited = (data.get("tool_input", {}) or {}).get("file_path") or ""
    if not edited:
        return 0

    repo = repo_root_from_env()
    if not repo:
        return 0

    idx = _store.load_json(index_path(repo), default=None)
    if not isinstance(idx, dict):
        return 0  # no index yet → dormant until memory-coherence-audit.py runs
    cites = idx.get("cites") or {}
    if not cites:
        return 0

    # Normalize the edited path to repo-relative
    try:
        ep = Path(edited)
        rel = str(ep.relative_to(repo)) if ep.is_absolute() else str(ep)
    except (ValueError, OSError):
        rel = edited

    matched: set[str] = set()
    for pattern, slugs in cites.items():
        if changed_matches_cite(rel, pattern):
            for s in slugs if isinstance(slugs, list) else []:
                matched.add(s)
    if not matched:
        return 0

    # The bound lives in .claude/epr-meta (memory-coherence-drift@1). One fold per memory
    # entry whose declared cites: glob the edited path matched.
    if _obs.available():
        for slug in sorted(matched):
            _obs.emit(
                "memory-coherence-drift@1", f".claude/memory/{slug}.md", 1,
                reason="memory coherence: cited code changed since this entry was verified",
                env={"changed": rel}, root=str(repo))

    return 0


if __name__ == "__main__":
    sys.exit(main())
