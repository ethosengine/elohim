"""Shared plumbing for `epr flow sense` probes.

A probe implements ONE measure row's prose `procedure:` and prints one JSON object on stdout:
`{"value": N}` for an `emits: count` probe, or `{"candidates": [{"subject", "symbol",
"signature"}]}` for an `emits: candidates` probe; `detail` lines are for people and are never
folded. The probe never writes to the fold plane and never judges a candidate: `epr flow sense`
folds what it prints, and a `ruling` note on the candidate's file classifies it.

Probes run as `python3 -I`, so this module is imported by path, never from the cwd.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import sys
from pathlib import Path

#: Directories that hold the development system's own measurement and governance code —
#: where a bypass of the native fold plane or limit model would live.
DEV_SYSTEM_ROOTS = (
    ".claude/scripts",
    ".claude/hooks",
    "elohim/eprfs",
    "genesis/orchestrator",
    "genesis/agentic",
    "genesis/a2o/scripts",
    "genesis/a2o/steps",
)

#: Source suffixes a probe reads.
SOURCE_SUFFIXES = (".py", ".rs", ".mjs", ".js", ".ts", ".sh")

#: Path fragments that mark test code or fixtures. A test that re-states a contract to check it is
#: not a second implementation of it.
TEST_MARKERS = ("/tests/", "/__tests__/", "/fixtures/", ".test.", "_test.", "/test_", ".spec.")


def root() -> Path:
    return Path(os.environ.get("EPR_SENSE_ROOT") or os.getcwd()).resolve()


def tracked(base: Path, roots=DEV_SYSTEM_ROOTS, suffixes=SOURCE_SUFFIXES) -> list[str]:
    """Tracked files under `roots` with one of `suffixes`, test code excluded, sorted."""
    out = subprocess.run(
        ["git", "-C", str(base), "ls-files", "-z", "--", *roots],
        capture_output=True,
        check=True,
    ).stdout.decode("utf-8", "replace")
    files = [p for p in out.split("\0") if p]
    return sorted(
        p
        for p in files
        if p.endswith(suffixes) and not any(m in f"/{p}" for m in TEST_MARKERS)
    )


def read(base: Path, rel: str) -> str:
    try:
        return (base / rel).read_text(encoding="utf-8", errors="replace")
    except OSError:
        return ""


def matching_lines(text: str, pattern: re.Pattern) -> list[int]:
    return [n for n, line in enumerate(text.splitlines(), 1) if pattern.search(line)]


def emit(payload: dict) -> None:
    json.dump(payload, sys.stdout, sort_keys=True)
    sys.stdout.write("\n")
