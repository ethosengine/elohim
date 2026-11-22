#!/usr/bin/env python3
"""Silently carry successful Edit/Write/NotebookEdit paths into the parent session's berth."""
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import sys
import time

CODE = Path(__file__).resolve().parents[2]
ROOT = Path(os.environ.get("CLAUDE_PROJECT_DIR") or CODE)
ID = re.compile(r"[A-Za-z0-9][A-Za-z0-9._:-]{0,127}")
POLL_S = 10
DEADLINE_S = 2


def state_root(root=None):
    """The main checkout's status dir, from a linked worktree too (gate-live-inject live_root)."""
    root = ROOT if root is None else root
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
    return main / ".eprfs" / "status" / "berth-writes"


def state_path(session, reader):
    """One file per (session, reader): a hash of both joined by NUL, so no two pairs share it."""
    key = hashlib.sha256(session.encode() + b"\0" + reader.encode()).hexdigest()[:40]
    return state_root() / f"{key}.json"


def read_state(path):
    try:
        state = json.loads(path.read_text())
        return state if isinstance(state, dict) else {}
    except (OSError, ValueError):
        return {}


def write_state(path, state):
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        tmp = path.with_name(f"{path.name}.{os.getpid()}.tmp")
        tmp.write_text(json.dumps(state))
        os.replace(tmp, path)
    except OSError:
        pass


def observe(payload):
    if not isinstance(payload, dict):
        return
    session = payload.get('session_id')
    inputs = payload.get('tool_input')
    if not isinstance(session, str) or not ID.fullmatch(session):
        return
    if payload.get('tool_name') not in ('Edit', 'Write', 'NotebookEdit'):
        return
    if not isinstance(inputs, dict):
        return
    path = inputs.get('file_path') or inputs.get('notebook_path')
    if not isinstance(path, str) or not path or '\0' in path:
        return
    path = os.path.abspath(os.path.join(str(ROOT), path))
    # agent_id deliberately does not select a different owner or throttle state.
    state = state_path(session, 'main')
    state.parent.mkdir(parents=True, exist_ok=True)
    with state.with_suffix('.lock').open('a') as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        saved = read_state(state)
        now = time.time()
        key = hashlib.sha256(path.encode()).hexdigest()[:40]
        seen = saved.get('seen', [])
        if key in seen and 0 <= now - saved.get('checked_at', 0) < POLL_S:
            return
        # Stamp attempts too: missing/slow berth must not cause a per-edit retry storm.
        write_state(state, {'checked_at': now, 'seen': list(dict.fromkeys([*seen, key]))[-200:]})
        binary = os.environ.get('BERTH_WRITES_BERTH_BIN') or str(CODE / 'genesis/agentic/bin/berth')
        proc = subprocess.Popen([binary, 'observed', '--session', session, path],
                                stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                                stderr=subprocess.DEVNULL, start_new_session=True)
        try:
            proc.wait(timeout=1.6)
        finally:
            # berth may itself be awaiting epr; leave no descendant behind on timeout.
            if proc.poll() is None:
                os.killpg(proc.pid, signal.SIGKILL)
                proc.wait(timeout=0.1)


def deadline(signum, frame):
    raise TimeoutError('hook deadline')


def main():
    signal.signal(signal.SIGALRM, deadline)
    signal.setitimer(signal.ITIMER_REAL, DEADLINE_S)
    try:
        observe(json.load(sys.stdin))
    except Exception:
        pass
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)


if __name__ == '__main__':
    main()
    os._exit(0)
