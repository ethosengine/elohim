#!/usr/bin/env python3
"""PostToolUse, every tool: a gate red reaches the agent at its next tool call, unasked.

Reads only this session's gate-live pointer and this reader's cursor (one per agent, so a
subagent never consumes its parent's news) and spawns node only when the run log has grown
since this reader last looked; gate-live.mjs `unseen` formats the lines and moves the cursor.
A background push or gate also gets the one Monitor command that wakes an idle agent on the
next red, since no hook fires while an agent waits. Fail-open: any error is silence.
Habit gate-failures-arrive-live (genesis/orchestrator/.epr-meta/).
"""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys

CODE = Path(__file__).resolve().parents[2]
ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or CODE)
GATE_LIVE = CODE / "genesis" / "orchestrator" / "gate-live.mjs"
SEGMENT = re.compile(r"&&|\|\||[;|&\n()]")


def filename(value):
    """gate-live.mjs sessionFilename: the same sanitizing, so both name one file."""
    return re.sub(r"[^A-Za-z0-9._-]", "_", str(value))


def live_root(root=ROOT):
    """The main checkout's run log, from a linked worktree too (gate-live.mjs liveRoot)."""
    main = root
    dot_git = root / ".git"
    try:
        if dot_git.is_file():
            gitdir = Path(dot_git.read_text().split("gitdir:", 1)[1].strip())
            gitdir = gitdir if gitdir.is_absolute() else (root / gitdir)
            common = (gitdir / "commondir").read_text().strip()
            main = (gitdir / common).resolve().parent
    except (OSError, IndexError):
        pass
    return main / ".eprfs" / "status" / "gate-live"


def has_news(live, session, reader):
    """True when this reader's cursor no longer describes the session's run file."""
    try:
        run = (live / "by-session" / filename(session)).read_text().strip()
        size = os.stat(run).st_size
    except OSError:
        return False
    try:
        cursor = json.loads((live / "cursors" / f"{filename(session)}--{filename(reader)}.json").read_text())
    except (OSError, ValueError):
        return True
    return cursor.get("run") != run or cursor.get("size") != size


def unseen(session, reader):
    result = subprocess.run(
        ["node", str(GATE_LIVE), "unseen", "--session", session, "--reader", reader],
        cwd=str(ROOT), capture_output=True, text=True, timeout=4,
        # A hook inherits nothing from git normally, but never let a pinned GIT_DIR move the log.
        env={k: v for k, v in os.environ.items() if not k.startswith("GIT_")},
    )
    return [line for line in result.stdout.splitlines() if line.strip()]


def starts_gate_run(command):
    """A `git push` (any global options before the verb) or a `just gate` / gate-runner call."""
    for segment in SEGMENT.split(command or ""):
        try:
            words = shlex.split(segment)
        except ValueError:
            words = segment.split()
        words = [w for w in words if not re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", w)] if words else []
        if not words:
            continue
        if words[0] == "git":
            rest = words[1:]
            while rest and rest[0].startswith("-"):
                rest = rest[2:] if rest[0] in ("-C", "-c") else rest[1:]
            if rest and rest[0] == "push":
                return True
        if words[:2] == ["just", "gate"] or any(w.endswith("gate-runner.mjs") for w in words):
            return True
    return False


def arm_line(session):
    return (
        "[gate-live] This gate run is in the background. Its reds arrive here at your next tool call; "
        "if you are about to wait idle, arm Monitor now so the first red wakes you: "
        f"`node {GATE_LIVE} follow --session {session}` (load it with ToolSearch select:Monitor). "
        "Investigate read-only until the push exits."
    )


def main():
    try:
        payload = json.load(sys.stdin)
    except (ValueError, OSError):
        return
    session = payload.get("session_id") or os.environ.get("ELOHIM_SESSION_ID") or ""
    if not isinstance(session, str) or not session.strip():
        return
    session = session.strip()
    reader = payload.get("agent_id") or "main"
    lines = []

    tool_input = payload.get("tool_input") or {}
    if (payload.get("tool_name") == "Bash" and tool_input.get("run_in_background")
            and starts_gate_run(tool_input.get("command", ""))):
        lines.append(arm_line(session))

    live = live_root()
    if has_news(live, session, reader):
        lines.extend(unseen(session, str(reader)))

    if lines:
        print(json.dumps({"hookSpecificOutput": {
            "hookEventName": "PostToolUse",
            "additionalContext": "\n".join(lines),
        }}))


if __name__ == "__main__":
    try:
        main()
    except Exception:  # noqa: BLE001 — a hook on every tool call must never break the call
        pass
