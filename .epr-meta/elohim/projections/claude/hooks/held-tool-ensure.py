#!/usr/bin/env python3
"""PostToolUse and SessionStart, every tool: keep this device's held-tool runner alive, and nothing else.

A session held on a tool call (very likely a permission prompt nobody saw) runs no hooks, so nothing
inside it can say it is held. The runner does that from outside: `genesis/agentic/bin/held-tool-watch
run`, one per device, an `announce` pass every minute (observe through the held session's own pinned
epr; then one berth note per incident to each live session moored here or on a sibling device, and
one more when the call resumes). This hook only makes sure that runner exists: it tries the runner's
lock without waiting and, when the lock is free, starts `held-tool-watch run` detached in its own
session. It reads no transcript, writes no record and spawns nothing while a runner holds the lock.

Covered: every device on which any Claude Code or Codex session has started or made a tool call since
the workspace last restarted, including a device whose only session is the one that later holds.
Not covered: a device where no session has started since its restart, and the runner is not
supervised by the device's ark (`epr device run` declares bin-store tools only).

Never in the way: a few milliseconds, exit 0 always; its own failure is one line on stderr.
HELD_TOOL_WATCH=off turns it off. Runs from Claude Code (`$CLAUDE_PROJECT_DIR/.claude/hooks/`) and
Codex (`.codex/hooks/`); the checkout is the one this file sits in.
"""
import fcntl
import os
from pathlib import Path
import subprocess
import sys

CODE = Path(__file__).resolve().parents[2]
LOCK_NAME = "held-tool-watch.run.lock"  # genesis/agentic/bin/held-tool-watch LOOP_LOCK
WATCHER = Path("genesis/agentic/bin/held-tool-watch")


def main_checkout(root):
    """A linked worktree's main checkout, else root (held-tool-watch main_checkout)."""
    try:
        marker = (root / ".git").read_text().strip()
        if not marker.startswith("gitdir: "):
            return root
        git_dir = (root / marker[8:]).resolve()
        common = (git_dir / (git_dir / "commondir").read_text().strip()).resolve()
        if common.name == ".git" and common.is_dir():
            return common.parent
    except OSError:
        pass
    return root


def lock_path(code=CODE):
    return main_checkout(code) / ".eprfs" / "status" / LOCK_NAME


def spawn(watcher, code):
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    subprocess.Popen([sys.executable, str(watcher), "run"], cwd=str(code), env=env,
                     stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                     start_new_session=True, close_fds=True)


def ensure(code=CODE, start=spawn):
    """'off', 'absent', 'running' or 'started'."""
    if os.environ.get("HELD_TOOL_WATCH") == "off":
        return "off"
    watcher = code / WATCHER
    if not watcher.is_file():
        return "absent"
    lock = lock_path(code)
    lock.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(lock, os.O_RDWR | os.O_CREAT, 0o644)
    try:
        try:
            fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            return "running"
        fcntl.flock(fd, fcntl.LOCK_UN)
    finally:
        os.close(fd)
    start(watcher, code)
    return "started"


def main():
    try:
        ensure()
    except Exception as error:  # noqa: BLE001 — a hook on every tool call must never break the call
        try:
            print(f"held-tool-ensure: {type(error).__name__}: {error}", file=sys.stderr)
        except Exception:  # noqa: BLE001
            pass


if __name__ == "__main__":
    main()
    # Exit without the interpreter's own stdout flush: a broken pipe there would turn into exit 120.
    os._exit(0)
