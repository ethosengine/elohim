"""Best-effort JSON store helpers for accumulators/state files.

Hooks and audit tools both need a small JSON keyed store with these properties:
  - Tolerant of missing files (returns sensible default)
  - Tolerant of malformed JSON (returns default rather than crash)
  - Atomic-ish write (write to tmp + rename)
  - Never raises on filesystem errors — accumulators must not fail tool calls

If a caller needs stronger guarantees (concurrency, versioning, fsync),
they should use a real DB. This is for small-state operational JSON only.
"""
from __future__ import annotations

import json
import os
import time
from pathlib import Path
from typing import Any, Callable

try:
    import fcntl  # POSIX advisory file locking (Linux/macOS); absent on Windows
except ImportError:  # pragma: no cover — the harness is Linux-only
    fcntl = None  # type: ignore[assignment]


def load_json(path: Path | str, default: Any = None) -> Any:
    """Load JSON from disk; return `default` (or {}) on any failure."""
    p = Path(path)
    if default is None:
        default = {}
    if not p.is_file():
        return default
    try:
        return json.loads(p.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return default


def save_json(path: Path | str, data: Any, indent: int = 2) -> bool:
    """Write JSON to disk atomically. Returns True on success; False otherwise.

    Best-effort: never raises. Callers (especially hooks) should swallow
    failure rather than crash.
    """
    p = Path(path)
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        tmp = p.with_suffix(p.suffix + ".tmp")
        tmp.write_text(json.dumps(data, indent=indent), encoding="utf-8")
        os.replace(tmp, p)
        return True
    except OSError:
        return False


def locked_update(
    path: Path | str,
    fn: Callable[[Any], Any],
    default: Any = None,
    *,
    timeout: float = 0.5,
    retry: float = 0.02,
) -> bool:
    """Read-modify-write `path`'s JSON under an fcntl advisory lock, so concurrent accumulator
    hooks don't lose each other's updates (the classic load-modify-save race → last-writer-wins).

    `fn(data) -> new_data`: receives the loaded JSON (or `default`) and returns the value to save.
    The load, the transform, and the save all happen while the lock is held.

    Non-blocking with short retry; gives up SILENTLY after ~`timeout` seconds (a hook must NEVER
    block a tool call waiting on a lock). Returns True iff the update was written. Best-effort:
    never raises. If fcntl is unavailable, degrades to an unlocked read-modify-write (still safe on
    a single-writer host; the lock only adds cross-session serialization where it matters).
    """
    p = Path(path)

    def _rmw() -> bool:
        data = load_json(p, default)
        try:
            new = fn(data)
        except Exception:
            return False
        return save_json(p, new)

    if fcntl is None:  # pragma: no cover — non-POSIX fallback
        return _rmw()

    lock_path = p.with_suffix(p.suffix + ".lock")
    fh = None
    try:
        p.parent.mkdir(parents=True, exist_ok=True)
        fh = open(lock_path, "w")  # noqa: SIM115 — closed in finally
    except OSError:
        return False
    try:
        deadline = time.monotonic() + timeout
        while True:
            try:
                fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except OSError:
                if time.monotonic() >= deadline:
                    return False  # contended — give up silently rather than block the tool call
                time.sleep(retry)
        return _rmw()
    finally:
        try:
            fcntl.flock(fh.fileno(), fcntl.LOCK_UN)
        except OSError:
            pass
        try:
            fh.close()
        except OSError:
            pass
