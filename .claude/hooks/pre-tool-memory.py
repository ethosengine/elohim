#!/usr/bin/env python3
"""
PreToolUse Memory Hook

Injects the project's auto-memory MEMORY.md into context before the first tool
call of a process tree that has NOT already received it. The harness injects
MEMORY.md natively for the MAIN session (the claudeMd system-reminder), so this
hook's real job is the gap: subagent process trees, which never see SessionStart
output or the native injection. load-project-context.py (SessionStart) pre-seeds
this hook's flag for the main tree so the main session is never double-injected.

Flag files are keyed on (session_id, ppid) — session_id (from the hook stdin
payload) kills cross-session PID-reuse collisions; ppid distinguishes subagent
trees within a session. Flags carry a TTL so a stale flag can never permanently
suppress injection, and old flags are opportunistically cleaned.

Emission is hookSpecificOutput JSON (PreToolUse additionalContext) — plain
stdout on PreToolUse never reaches model context (2026-07-02 review finding:
the prior print()-based version delivered nothing, silently).

Hook Type: PreToolUse (matcher "*")
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

MEMORY_BUDGET_LINES = 200
FLAG_TTL_SECONDS = 24 * 3600      # a flag older than this no longer suppresses
FLAG_SWEEP_SECONDS = 48 * 3600    # flags older than this are deleted on sight
FLAG_PREFIX = "claude-memory-loaded-"


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


def flag_path(session_id: str) -> Path:
    return Path(f"/tmp/{FLAG_PREFIX}{session_id}-{os.getppid()}")


def flag_fresh(flag: Path) -> bool:
    try:
        return (time.time() - flag.stat().st_mtime) < FLAG_TTL_SECONDS
    except OSError:
        return False


def sweep_stale_flags() -> None:
    """Best-effort cleanup of long-dead flags (runs only on the rare inject path)."""
    now = time.time()
    try:
        for p in Path("/tmp").glob(f"{FLAG_PREFIX}*"):
            try:
                if now - p.stat().st_mtime > FLAG_SWEEP_SECONDS:
                    p.unlink()
            except OSError:
                continue
    except OSError:
        pass


def session_id_from_stdin() -> str:
    try:
        payload = json.load(sys.stdin)
        return str(payload.get("session_id") or "nosess")[:64]
    except (json.JSONDecodeError, ValueError):
        return "nosess"


def main() -> int:
    sid = session_id_from_stdin()
    flag = flag_path(sid)
    if flag_fresh(flag):
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

    sweep_stale_flags()
    body = "\n".join(
        ["PROJECT MEMORY INDEX (auto-memory complement, first {} lines of MEMORY.md):".format(MEMORY_BUDGET_LINES), ""]
        + lines
        + ["", "(End of MEMORY.md preview. Topic files live alongside; read them when relevant.)"]
    )
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "additionalContext": body,
        }
    }))

    try:
        flag.touch()
    except OSError:
        pass
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception:  # noqa: BLE001 — fail-open by contract; never block a tool call
        sys.exit(0)
