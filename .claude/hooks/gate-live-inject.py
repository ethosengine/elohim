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
import time

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


SIBLING_KINDS = ("gate-red@1", "gate-final@1")
SIBLING_POLL_S = 20
SIBLING_FRESH_S = 600


def berth_bin():
    return os.environ.get("GATE_LIVE_BERTH_BIN") or str(CODE / "genesis" / "agentic" / "bin" / "berth")


def addressed_to(to, session):
    """berth's own rule: the full id, or a prefix of at least eight characters."""
    return bool(to and session and (to == session or (len(to) >= 8 and session.startswith(to))))


def sibling_line(body, note):
    """One line per sibling note; the text is another device's data, clipped and never executed."""
    def clip(value, n=200):
        return " ".join(str(value if value is not None else "").split())[:n]
    where = clip(str(body.get("device") or note.get("from_device") or "device").split(":")[-1][:10]
                 + (" " + str(body["ref"]) if body.get("ref") else ""))
    if body.get("kind") == "gate-red@1":
        cont = body.get("continuing") or {}
        more = ""
        if cont:
            more = f"; {len(cont.get('remaining') or [])} gate(s) still to run, ~{max(1, round((cont.get('total_s') or 0) / 60))} min"
        stop = f" (to stop it, on that device: {clip(body['stop'])})" if body.get("stop") else ""
        return f"[gate-live] SIBLING RED {where} {clip(body.get('gate'))} +{body.get('atS', '?')}s — {clip(body.get('line'))}{more}{stop}"
    reds = body.get("reds") or []
    detail = ": " + " | ".join(f"{clip(r.get('gate'))}: {clip(r.get('line'))}" for r in reds[:3]) if reds else ""
    return (f"[gate-live] SIBLING FINAL {where} {clip(body.get('verdict'))} in {body.get('wallS', '?')}s — "
            f"{len(reds)} red(s){detail}; ran {len(body.get('ran') or [])}, reused {len(body.get('reused') or [])}")


def sibling_unseen(live, session, reader, now=None):
    """Sibling gate-red@1 / gate-final@1 notes addressed to this session that this reader has not
    been shown, read with `berth inbox --peek` (nothing consumed) and tracked by note cid in this
    reader's own cursor. Polled at most every SIBLING_POLL_S; any error is silence."""
    now = time.time() if now is None else now
    path = live / "cursors" / f"{filename(session)}--{filename(reader)}--sibling.json"
    try:
        state = json.loads(path.read_text())
    except (OSError, ValueError):
        state = None
    poll = float(os.environ.get("GATE_LIVE_SIBLING_POLL_S", SIBLING_POLL_S))
    if state and now - state.get("checked_at", 0) < poll:
        return []
    result = subprocess.run(
        [berth_bin(), "inbox", "--session", session, "--peek", "--all", "--json"],
        capture_output=True, text=True, timeout=3,
    )
    if result.returncode != 0:
        return []
    notes = json.loads(result.stdout)
    seen = set(state.get("cids", [])) if state else set()
    lines = []
    for note in notes:
        cid = note.get("cid")
        if not cid or cid in seen or not addressed_to(note.get("to"), session):
            continue
        try:
            body = json.loads(note.get("text") or "")
        except ValueError:
            continue
        if not isinstance(body, dict) or body.get("kind") not in SIBLING_KINDS:
            continue
        seen.add(cid)
        # First contact: a red older than ten minutes is history, not news.
        if state is None and now - float(note.get("at") or 0) > SIBLING_FRESH_S:
            continue
        lines.append(sibling_line(body, note))
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"checked_at": now, "cids": sorted(seen)[-500:]}))
    return lines


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

    try:
        lines.extend(sibling_unseen(live, session, str(reader)))
    except Exception:  # noqa: BLE001 — berth missing, slow or broken: surface nothing
        pass

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
