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
OPERATORS = "();&|\n"
ASSIGNMENT = re.compile(r"^[A-Za-z_][A-Za-z0-9_]*=")
RUNNER = "genesis/orchestrator/gate-runner.mjs"


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


def command_segments(command):
    """Best-effort shell boundaries; retain quotes so echo ';' is not a separator."""
    lexer = shlex.shlex(command.replace("\\\n", ""), posix=False,
                        punctuation_chars=OPERATORS)
    lexer.whitespace = " \t\r"
    lexer.whitespace_split = True
    segment = []
    for token in lexer:
        if all(char in OPERATORS for char in token):
            if segment:
                yield shlex.split(" ".join(segment))
            segment = []
        else:
            segment.append(token)
    if segment:
        yield shlex.split(" ".join(segment))


def starts_gate_run(command):
    """Recognize executed gates without mistaking quoted mentions or dry runs for one."""
    if not isinstance(command, str):
        return False
    try:
        segments = list(command_segments(command))
    except ValueError:
        return False
    for words in segments:
        while words and ASSIGNMENT.match(words[0]):
            words = words[1:]
        if not words:
            continue
        head, args = os.path.basename(words[0]), words[1:]
        if head == "git":
            # Global git options precede the subcommand; -C and -c consume a value.
            while args and args[0].startswith("-"):
                if args[0] in ("-C", "-c", "--git-dir", "--work-tree"):
                    args = args[2:]
                else:
                    args = args[1:]
            if args and args[0] == "push" and not {"--dry-run", "-n"}.intersection(args[1:]):
                return True
        elif head == "just" and args[:1] == ["gate"]:
            return True
        else:
            script = args[0] if head == "node" and args else words[0]
            if script == RUNNER or script.endswith("/" + RUNNER):
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
    if (payload.get("tool_name") == "Bash" and tool_input.get("run_in_background") is True
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
