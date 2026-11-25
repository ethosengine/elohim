"""A hook-wide wall-clock deadline for subprocess calls.

A hook registered with a `timeout` of N seconds is cancelled by the harness at N seconds, and the
cancellation is a visible exception. A hook that spawns `epr` with its own larger timeouts can
overrun on a loaded device. A hook calls `set(seconds)` once at start; every subprocess it makes
through `cap(default, what)` then gets `min(default, time left)`, and when nothing is left the call
is skipped. What was skipped is recorded so the hook can say so in one stderr line.

With no deadline set (every other caller) `cap` returns `default` unchanged.
"""
from __future__ import annotations

import time

_end: float | None = None
skipped: list[str] = []


def set(seconds: float) -> None:  # noqa: A001 — the module is used as `deadline.set(...)`
    global _end
    _end = time.monotonic() + seconds
    skipped.clear()


def active() -> bool:
    return _end is not None


def remaining() -> float | None:
    return None if _end is None else _end - time.monotonic()


def cap(default: float, what: str) -> float:
    """The timeout for one subprocess; raises TimeoutError when the deadline has passed."""
    left = remaining()
    if left is None:
        return default
    if left <= 0.05:
        skipped.append(what)
        raise TimeoutError(what)
    return min(default, left)


def note_expired(what: str) -> None:
    """Record that a call ended on the (possibly capped) timeout, when a deadline is in force."""
    if _end is not None and what not in skipped:
        skipped.append(what)


def line(hook: str) -> str | None:
    if not skipped:
        return None
    return f"{hook}: skipped {', '.join(skipped)} — the hook's 2.5s internal deadline was reached (harness budget 3s)"
