#!/usr/bin/env python3
"""
Managed-surface edit-discipline injector (the PRE half of "you don't have to remember").

PreToolUse hook (matcher: Edit|Write). BEFORE an edit lands on a file that belongs to the
project-managed memory system (gospel CLAUDE.mds, specs/plans, doc-roots, memory entries, a2o
features, stories, skills/agents, axis manifests — see _lib.managed_surfaces, the single
edit-time registry), inject that surface's discipline + exact tooling into context. This is the
hook that prevents the 2026-06-05 episode shape: an agent hand-writing cite slugs into a
CLAUDE.md because nothing told it the file was cite-graph-managed until AFTER the edit.

Sibling of cite-seal-signal.py (the POST half — verifies debt after the write). Fires at most
once per (file, parent process tree): the discipline statement is context, not nagging.
Single-digit-ms cheap path (registry is pattern-match only); best-effort, fail-open, never blocks.

Hook Type: PreToolUse   Matcher: Edit|Write
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import managed_surfaces as ms  # noqa: E402


def _flag(rel: str) -> Path:
    h = hashlib.sha1(rel.encode("utf-8", "replace")).hexdigest()[:12]
    return Path(f"/tmp/claude-managed-surface-{os.getppid()}-{h}")


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    edited = (data.get("tool_input", {}) or {}).get("file_path") or ""
    pd = os.environ.get("CLAUDE_PROJECT_DIR")
    if not edited or not pd:
        return 0
    repo = Path(pd).resolve()
    try:
        entry = ms.classify(edited, repo)
    except Exception:
        return 0
    if not entry:
        return 0
    try:
        ep = Path(edited)
        rel = str(ep.resolve().relative_to(repo)) if ep.is_absolute() else str(ep)
    except (ValueError, OSError):
        rel = edited
    flag = _flag(rel)
    if flag.exists():
        return 0
    tools = "  ·  ".join(entry["tools"])
    msg = (f"[managed-surface] {rel} is a MANAGED-MEMORY SURFACE — {entry['label']}.\n"
           f"Discipline: {entry['discipline']}\n"
           f"Tooling: {tools}")
    print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", "additionalContext": msg}}))
    try:
        flag.touch()
    except OSError:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
