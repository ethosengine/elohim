#!/usr/bin/env python3
"""
PreToolUse Memory Hook

Injects the project's auto-memory MEMORY.md (first 200 lines) into context
before the first tool call of a session / subagent process tree. Per the
Pawel Huryn pattern: survives context compaction and reaches subagents that
otherwise wouldn't see the SessionStart hook's output.

Fires at most once per parent process tree, gated by a flag file under /tmp.
Exits silently when the flag is already present so the per-call overhead
stays in the single-digit-ms range.

Hook Type: PreToolUse (matcher "*")
"""
from __future__ import annotations

import os
import sys
from pathlib import Path

MEMORY_BUDGET_LINES = 200


def memory_dir() -> Path:
    """Resolve project memory location.

    Two-tier model (2026-05-13):
      Primary: <CLAUDE_PROJECT_DIR>/.claude/memory/ (team-shareable, git-tracked).
      Fallback: /projects/.claude-config/projects/<slug>/memory/ (personal slot).
    The personal slot is typically a symlink to the primary; both resolve
    to the same files. We check the primary first to be explicit about intent.
    """
    env_dir = os.environ.get("CLAUDE_MEMORY_DIR")
    if env_dir:
        return Path(env_dir).expanduser()
    project_dir = os.environ.get("CLAUDE_PROJECT_DIR")
    if not project_dir:
        return Path("/dev/null/missing-project-dir")
    primary = Path(project_dir) / ".claude" / "memory"
    if primary.is_dir():
        return primary
    slug = "-" + "-".join(Path(project_dir).resolve().parts[1:])
    return Path(f"/projects/.claude-config/projects/{slug}/memory")


def flag_path() -> Path:
    return Path(f"/tmp/claude-memory-loaded-{os.getppid()}")


def main() -> int:
    flag = flag_path()
    if flag.exists():
        return 0

    index = memory_dir() / "MEMORY.md"
    if not index.is_file():
        try:
            flag.touch()
        except OSError:
            pass
        return 0

    try:
        lines = index.read_text(encoding="utf-8", errors="replace").splitlines()[:MEMORY_BUDGET_LINES]
    except OSError:
        return 0

    print("PROJECT MEMORY INDEX (auto-memory complement, first {} lines of MEMORY.md):".format(
        MEMORY_BUDGET_LINES
    ))
    print()
    for line in lines:
        print(line)
    print()
    print("(End of MEMORY.md preview. Topic files live alongside; read them when relevant.)")

    try:
        flag.touch()
    except OSError:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
