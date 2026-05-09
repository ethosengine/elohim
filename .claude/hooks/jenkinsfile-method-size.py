#!/usr/bin/env python3
"""
Post-edit hook: catch MethodTooLargeException risk on Jenkinsfile edits.

Jenkins compiles each `pipeline { }` block into one CPS method capped at
64KB JVM bytecode. Source bytes correlate with compiled bytecode but
aren't 1:1. Empirical thresholds (calibrated from real builds):

  • elohim/holochain/Jenkinsfile @ 62812 source — compiles fine (#1210)
  • genesis/orchestrator/Jenkinsfile @ 74732 source — failed #922 with
    MethodTooLargeException

This hook runs after Edit/Write on any path matching *Jenkinsfile*.
Stays silent on pass; only outputs feedback when the file crosses the
WARN or FAIL threshold so context isn't cluttered.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import re
import sys
from pathlib import Path

WARN = 55000
HARD = 65000


def extract_pipeline_block_bytes(text: str) -> int:
    """Count source bytes inside the top-level `pipeline { ... }` block.

    Tracks brace depth: enter at `pipeline {`, exit when depth returns to 0.
    Returns 0 if no `pipeline {` block is present (file is a library /
    shared module).
    """
    in_pipe = False
    depth = 0
    bytes_count = 0
    for line in text.splitlines(keepends=True):
        if not in_pipe:
            if re.match(r"^pipeline\s*\{", line):
                in_pipe = True
            else:
                continue
        bytes_count += len(line)
        depth += line.count("{")
        depth -= line.count("}")
        if in_pipe and depth == 0:
            return bytes_count
    return 0 if not in_pipe else bytes_count


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0  # silently no-op on malformed input

    tool_input = payload.get("tool_input") or {}
    file_path = tool_input.get("file_path") or ""

    # Match any *Jenkinsfile* path. Skip everything else silently.
    if "Jenkinsfile" not in Path(file_path).name:
        return 0

    try:
        text = Path(file_path).read_text(encoding="utf-8")
    except (FileNotFoundError, PermissionError, UnicodeDecodeError):
        return 0  # file moved / unreadable — don't surface noise

    size = extract_pipeline_block_bytes(text)
    if size == 0:
        return 0  # not a pipeline-bearing Jenkinsfile (library/shared)

    if size >= HARD:
        # Use stderr so Claude Code surfaces the message prominently.
        print(
            f"\n❌ {file_path}: pipeline {{}} block is {size} bytes "
            f"(HARD limit {HARD}, JVM CPS method limit 64KB).\n"
            f"   This will fail at Jenkins compile time with "
            f"MethodTooLargeException.\n"
            f"   Refactor: extract `def helperFunc()` blocks ABOVE "
            f"`pipeline {{`, call them from script {{}} bodies.\n"
            f"   See CLAUDE.md: 'Never add inline logic to stages.'",
            file=sys.stderr,
        )
        # Exit code 2 → "blocking error" in Claude Code hooks. The user
        # asked specifically to surface this — don't let it pass quietly.
        return 2

    if size >= WARN:
        print(
            f"\n⚠️  {file_path}: pipeline {{}} block is {size} bytes "
            f"(WARN threshold {WARN}). Consider extracting helpers above "
            f"`pipeline {{` before this hits the {HARD} HARD limit.",
            file=sys.stderr,
        )
        return 0  # warning only — don't block

    # Silent pass — no output.
    return 0


if __name__ == "__main__":
    sys.exit(main())
