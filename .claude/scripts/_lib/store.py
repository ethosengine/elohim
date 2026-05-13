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
from pathlib import Path
from typing import Any


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
