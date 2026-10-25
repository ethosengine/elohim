"""Dependency-free adapter to the ephemeral native session identity primitive."""
from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
from typing import Mapping


def load_registry(env: Mapping[str, str], cwd: Path | None = None) -> dict:
    explicit = env.get("EPR_AGENT_RUNTIMES", "").strip()
    cwd = (cwd or Path.cwd()).resolve()
    path = Path(explicit) if explicit else next(
        (p / "genesis/agentic/agent-runtimes.json" for p in (cwd, *cwd.parents)
         if (p / "genesis/agentic").is_dir()), None)
    try:
        registry = json.loads(path.read_text()) if path else {}
        rows = registry["runtimes"]
        if not isinstance(rows, list) or any(
            not isinstance(r, dict) or not isinstance(r.get("id"), str)
            or any(not isinstance(r.get(k), list) or any(not isinstance(v, str) for v in r[k])
                   for k in ("comm", "sessionEnv")) for r in rows
        ):
            return {"runtimes": []}
        return registry
    except (OSError, ValueError, KeyError, TypeError):
        return {"runtimes": []}


def _process(root: Path, pid: int) -> dict | None:
    if pid <= 0 or pid > 0xffffffff:
        return None
    try:
        stat = (root / str(pid) / "stat").read_text()
        first, last = stat.index("("), stat.rindex(")")
        if first >= last:
            return None
        fields = stat[last + 1:].split()
        parent, leader, tty, start = (int(fields[i]) for i in (1, 3, 4, 19))
        if not (0 <= parent <= 0xffffffff and 0 <= leader <= 0xffffffff
                and -(2**63) <= tty < 2**63 and 0 <= start < 2**64):
            return None
        return dict(comm=stat[first + 1:last], parent=parent, leader=leader, tty=tty, start=start)
    except (OSError, ValueError, IndexError):
        return None


def _identity(runtime="none", session=None, owner=None, source="none") -> dict:
    return dict(runtime=runtime, session=session, ownerPid=owner[0] if owner else None,
                ownerStartTime=str(owner[1]) if owner else None, source=source)


def resolve_fallback(*, environ: Mapping[str, str] | None = None, proc_root: Path | None = None,
                     start_pid: int | None = None, registry: dict | None = None) -> dict:
    env = os.environ if environ is None else environ
    root = proc_root if proc_root is not None else Path(env.get("EPR_PROC_ROOT", "").strip() or "/proc")
    start_pid = os.getpid() if start_pid is None else start_pid
    registry = load_registry(env) if registry is None else registry
    declared = env.get("ELOHIM_SESSION_ID", "").strip()
    owner_raw = env.get("ELOHIM_SESSION_OWNER_PID", "").strip()
    try:
        owner_pid = int(owner_raw)
    except ValueError:
        owner_pid = None
    current = _process(root, start_pid)
    if declared and owner_pid == start_pid and current:
        return _identity("declared", declared, (start_pid, current["start"]), "declared")
    pid = current["parent"] if current else 0
    seen = {start_pid}
    for _ in range(256):
        if pid <= 0 or pid in seen:
            break
        seen.add(pid)
        process = _process(root, pid)
        if process is None:
            break
        if declared and owner_pid == pid:
            return _identity("declared", declared, (pid, process["start"]), "declared")
        for runtime in registry["runtimes"]:
            if process["comm"] in runtime["comm"]:
                try:
                    inherited = (root / str(pid) / "environ").read_bytes().split(b"\0")
                except OSError:
                    inherited = None
                session = next((env[k].strip() for k in runtime["sessionEnv"]
                                if env.get(k, "").strip() and inherited is not None
                                and f"{k}={env[k]}".encode() not in inherited), None)
                return _identity(runtime["id"], session or f'{runtime["id"]}-{pid}-{process["start"]}',
                                 (pid, process["start"]), "runtimeAncestor")
        pid = process["parent"]
    if declared and not owner_raw:
        return _identity("declared", declared, source="declared")
    if current and current["tty"]:
        leader = _process(root, current["leader"])
        if leader:
            pid = current["leader"]
            return _identity("tty", f'tty-{pid}-{leader["start"]}', (pid, leader["start"]), "ttyLeader")
    return _identity()


def resolve(*, environ: Mapping[str, str] | None = None, proc_root: Path | None = None,
            start_pid: int | None = None) -> dict:
    env = dict(os.environ if environ is None else environ)
    # An injected process root/pid is a fixture. Native resolution always uses its actual pid.
    # The guard prevents bin-launch -> shim -> bin-launch recursion on a PATH launcher.
    if proc_root is None and start_pid is None and not env.get("EPR_SESSION_RESOLVING"):
        try:
            result = subprocess.run([env.get("EPR_BIN") or "epr", "session", "id", "--json"],
                                    env={**env, "EPR_SESSION_RESOLVING": "1"},
                                    capture_output=True, text=True, timeout=2, check=True)
            identity = json.loads(result.stdout)
            if (isinstance(identity, dict) and set(identity) == set(_identity())
                    and identity.get("source") in ("declared", "runtimeAncestor", "ttyLeader", "none")):
                return identity
        except (OSError, ValueError, subprocess.SubprocessError):
            pass
    return resolve_fallback(environ=env, proc_root=proc_root, start_pid=start_pid)


def envelope(*, environ: Mapping[str, str] | None = None) -> dict | None:
    """Read the native facet view. Absence never grants a capability tier."""
    env = dict(os.environ if environ is None else environ)
    try:
        result = subprocess.run([env.get("EPR_BIN") or "epr", "session", "show", "--json"],
                                env=env, capture_output=True, text=True, timeout=5, check=True)
        view = json.loads(result.stdout)
        return view if isinstance(view, dict) else None
    except (OSError, ValueError, subprocess.SubprocessError):
        return None
