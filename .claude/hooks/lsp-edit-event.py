#!/usr/bin/env python3
"""PostToolUse edit attribution: private, bounded, fail-open C2 events for the mux."""
from collections import deque
from datetime import datetime, timezone
import fcntl
import json
import os
from pathlib import Path
import subprocess
import sys
import time


TOOLS = {"Edit", "Write", "MultiEdit", "NotebookEdit"}


def process_start(pid):
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]
    except (OSError, IndexError):
        return ""


def session_identity(payload, cwd):
    try:
        result = subprocess.run(
            [os.environ.get("EPR_BIN") or "epr", "session", "id", "--json"],
            cwd=cwd, capture_output=True, text=True, timeout=1, check=True,
        )
        identity = json.loads(result.stdout)
        if (isinstance(identity, dict) and isinstance(identity.get("session"), str)
                and identity["session"] and type(identity.get("ownerPid")) is int
                and identity["ownerPid"] > 0
                and isinstance(identity.get("ownerStartTime"), str)):
            return {key: identity[key] for key in ("session", "ownerPid", "ownerStartTime")}
    except (OSError, ValueError, subprocess.SubprocessError):
        pass
    session = payload.get("session_id") or os.environ.get("ELOHIM_SESSION_ID")
    try:
        pid = int(os.environ.get("ELOHIM_SESSION_OWNER_PID", ""))
        if pid <= 0:
            raise ValueError("invalid pid")
    except ValueError:
        pid = os.getppid()
    if not isinstance(session, str) or not session:
        pid = os.getpid()
        session = f"anon-{pid}"
    return {"session": session, "ownerPid": pid, "ownerStartTime": process_start(pid)}


def live_root(cwd):
    common = subprocess.check_output(
        ["git", "rev-parse", "--git-common-dir"], cwd=cwd, text=True,
        stderr=subprocess.DEVNULL, timeout=0.5,
    ).strip()
    git_dir = (cwd / common).resolve()
    return git_dir.parent


def append_event(path, event):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd = os.open(path, os.O_CREAT | os.O_RDWR | os.O_APPEND, 0o600)
    try:
        # Keep the inode stable across pruning so concurrent writers cannot append to an
        # unlinked file. Brief contention is normal when sessions edit together;
        # a stuck writer must still fit inside the hook's two-second budget.
        deadline = time.monotonic() + 0.15
        while True:
            try:
                fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.monotonic() >= deadline:
                    return
                time.sleep(0.005)
        os.write(fd, (json.dumps(event, separators=(",", ":")) + "\n").encode())
        if os.fstat(fd).st_size > 2 * 1024 * 1024:
            os.lseek(fd, 0, os.SEEK_SET)
            with os.fdopen(os.dup(fd), "rb") as stream:
                tail = b"".join(deque(stream, maxlen=10_000))
            os.ftruncate(fd, 0)
            os.write(fd, tail)
    finally:
        os.close(fd)


def main():
    try:
        payload = json.load(sys.stdin)
        tool = payload.get("tool_name")
        if tool not in TOOLS:
            return
        tool_input = payload.get("tool_input") or {}
        file_path = tool_input.get("file_path") or tool_input.get("notebook_path")
        if not isinstance(file_path, str) or not file_path:
            return
        cwd = Path(payload.get("cwd") or os.environ.get("CLAUDE_PROJECT_DIR") or os.getcwd()).resolve()
        root = live_root(cwd)
        event = {
            "v": 1,
            "at": datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z"),
            **session_identity(payload, cwd),
            "path": str((cwd / file_path).resolve()),
            "tool": tool,
        }
        append_event(root / ".eprfs/status/lsp/edits.jsonl", event)
    except Exception:
        # Diagnostics must never block an edit or emit protocol bytes to stdout.
        pass


if __name__ == "__main__":
    main()
