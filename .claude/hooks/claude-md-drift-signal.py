#!/usr/bin/env python3
"""
CLAUDE.md Drift-Signal Accumulator

PostToolUse hook (matcher: Edit|Write). Fast accumulator — runs after every
edit, increments a drift counter for each enclosing CLAUDE.md scope. Costs
single-digit ms in the cheap path; offloads all judgment to `claude-md-audit.py`
which only runs when the operator invokes the ceremony.

Layered compute (trust-compute gradient):
  - Cheap path (every edit): walk up dirs, increment scope_edits counter
  - Medium path (every N edits, configurable): re-compute drift score
  - Expensive path: deferred entirely to claude-md-audit.py

When a CLAUDE.md's drift_score crosses threshold, the next session's
SessionStart hook surfaces it; until then, gospel stands.

Storage: .claude/memory-kit/claude-md-drift.json (schema_version: 1)

Hook Type: PostToolUse
Matcher: Edit|Write
"""
from __future__ import annotations

import json
import math
import os
import sys
import time
from pathlib import Path

# Bootstrap: locate .claude/scripts/_lib by walking up
_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import store as _store  # noqa: E402
from _lib import drift_score as _drift_score  # noqa: E402

# Tunables specific to this hook (the score formula itself lives in _lib.drift_score)
RESCORE_EVERY_N_EDITS = 5  # medium-path trigger
DEFAULT_THRESHOLD = 3.0
MAX_WALK_DEPTH = 12        # stop walking up after this many dirs


def repo_root_from_env() -> Path | None:
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(pd).resolve() if pd else None


def find_enclosing_claude_md_files(edited_file: Path, repo_root: Path) -> list[Path]:
    """Walk up from edited_file's dir, collect every CLAUDE.md until repo_root.

    A file deep in the tree counts against every enclosing CLAUDE.md scope.
    """
    results: list[Path] = []
    try:
        cur = edited_file.resolve().parent
    except OSError:
        return results
    for _ in range(MAX_WALK_DEPTH):
        candidate = cur / "CLAUDE.md"
        if candidate.is_file():
            results.append(candidate)
        if cur == repo_root or cur.parent == cur:
            break
        cur = cur.parent
    return results


def drift_store_path(repo_root: Path) -> Path:
    return repo_root / ".claude" / "memory-kit" / "claude-md-drift.json"


def load_store(path: Path) -> dict:
    default = {"schema_version": 1, "threshold": DEFAULT_THRESHOLD, "files": {}}
    data = _store.load_json(path, default=default)
    if not isinstance(data, dict):
        return default
    data.setdefault("files", {})
    data.setdefault("threshold", DEFAULT_THRESHOLD)
    return data


def save_store(path: Path, data: dict) -> None:
    _store.save_json(path, data)  # best-effort; never raises


def get_or_init_file_entry(store: dict, claude_md_rel: str, mtime_iso: str) -> dict:
    entry = store["files"].get(claude_md_rel)
    if not entry:
        entry = {
            "last_audited": None,       # ISO date or None
            "claude_md_mtime": mtime_iso,
            "direct_edits": 0,          # edits to the CLAUDE.md itself
            "scope_edits": 0,           # edits to files within its scope
            "lines_changed_in_scope": 0,  # reserved for medium-path
            "drift_score": 0.0,
            "last_signal_at": None,     # ISO timestamp of last increment
            "rescore_counter": 0,       # increments to RESCORE_EVERY_N_EDITS
        }
        store["files"][claude_md_rel] = entry
    return entry


# compute_score moved to _lib.drift_score (shared with audit + structural hook)


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    tool_input = data.get("tool_input", {}) or {}
    edited = tool_input.get("file_path") or ""
    if not edited:
        return 0

    repo = repo_root_from_env()
    if not repo:
        return 0

    edited_path = Path(edited)
    if not edited_path.is_absolute():
        edited_path = repo / edited_path

    # Find enclosing CLAUDE.md files (could be 0..N)
    enclosing = find_enclosing_claude_md_files(edited_path, repo)
    if not enclosing:
        return 0

    store_path = drift_store_path(repo)
    store = load_store(store_path)
    now_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())

    is_direct_edit_target = edited_path.name == "CLAUDE.md"

    for claude_md in enclosing:
        try:
            rel = str(claude_md.relative_to(repo))
        except ValueError:
            rel = str(claude_md)
        try:
            mtime_iso = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(claude_md.stat().st_mtime))
        except OSError:
            mtime_iso = now_iso

        entry = get_or_init_file_entry(store, rel, mtime_iso)
        entry["claude_md_mtime"] = mtime_iso
        entry["last_signal_at"] = now_iso

        if is_direct_edit_target and edited_path == claude_md:
            entry["direct_edits"] = entry.get("direct_edits", 0) + 1
        else:
            entry["scope_edits"] = entry.get("scope_edits", 0) + 1

        entry["rescore_counter"] = entry.get("rescore_counter", 0) + 1
        if entry["rescore_counter"] >= RESCORE_EVERY_N_EDITS:
            entry["drift_score"] = _drift_score.compute_score(entry)
            entry["rescore_counter"] = 0

    save_store(store_path, store)
    return 0  # hooks are best-effort; never block the tool call


if __name__ == "__main__":
    sys.exit(main())
