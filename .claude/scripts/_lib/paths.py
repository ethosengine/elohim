"""Path/date helpers used by every .claude/scripts/* audit and report.

Eliminates the `REPO_ROOT = Path(__file__).resolve().parents[N]` duplication
(N varies between 2 and 3 depending on script depth — error-prone). Use
`repo_root_from_file(__file__)` instead, which walks up looking for a
`.claude` directory.
"""
from __future__ import annotations

import os
from datetime import date
from pathlib import Path


def repo_root_from_file(file: str | Path, max_depth: int = 8) -> Path:
    """Walk up from a script file to the repo root.

    Repo root is the directory that contains BOTH `.claude/` and `.git`
    (file in worktrees, dir in the main repo). Requiring both anchors is
    important: a `.claude/.claude/` artifact from a buggy run would
    otherwise satisfy a `.claude/`-only check and cause the helper to
    return `.claude/` itself as repo root — recursively reinforcing the
    doubled-path bug. The `.git` anchor breaks that cycle.

    Falls back to a `.claude/`-only match (legacy behavior) if no `.git`
    marker is found within `max_depth` levels — preserves usefulness in
    exotic layouts (e.g. monorepo subtrees vendored without `.git`).
    """
    start = Path(file).resolve()

    # First pass: strict — require both .claude and .git
    p = start
    for _ in range(max_depth):
        if (p / ".claude").is_dir() and (p / ".git").exists():
            return p
        if p.parent == p:
            break
        p = p.parent

    # Fallback to env if available
    env = os.environ.get("CLAUDE_PROJECT_DIR")
    if env:
        return Path(env).resolve()

    # Last-resort: legacy `.claude/`-only walk
    p = start
    for _ in range(max_depth):
        if (p / ".claude").is_dir():
            return p
        if p.parent == p:
            break
        p = p.parent
    return start.parent


def reports_root(repo_root: Path) -> Path:
    """The shared dated-reports directory under .claude/memory-kit/.

    Legacy name retained for compatibility with /converge and dev-dashboard
    scripts that also write here. Output dir naming is follow-up scope.
    """
    return repo_root / ".claude" / "memory-kit"


def reports_dir_for_today(repo_root: Path, today: date | None = None) -> Path:
    """Today's reports directory (e.g. .claude/memory-kit/2026-05-13/)."""
    return reports_root(repo_root) / (today or date.today()).isoformat()


def memory_dir(repo_root: Path) -> Path:
    """Primary project-memory location (git-tracked, team-shareable).

    Per `project_memory_in_repo_two_tier.md`: the personal harness slot at
    `.claude-config/projects/<slug>/memory/` is a symlink to this directory.
    """
    return repo_root / ".claude" / "memory"
