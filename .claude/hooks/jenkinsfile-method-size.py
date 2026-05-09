#!/usr/bin/env python3
"""
Post-edit hook: catch MethodTooLargeException risk on Jenkinsfile edits.

Jenkins compiles each `pipeline { }` block into a graph of CPS methods,
NOT a single method. Each `script { }`, `parallel { }` branch, closure
literal, and stage body becomes its own method capped at 64KB JVM
bytecode. Total pipeline-block bytes is the WRONG metric — a 56KB
pipeline can still have one dense closure that compiles to >64KB.

Empirical evidence (from build #923):
  pipeline { } total: 56825 source bytes — passed earlier "total" check
  failure: WorkflowScript.___cps___74 too large (a sub-method)

This hook now does TWO checks:
  1. Total pipeline {} block size — catches gross-overweight files
  2. Max INNER block size — catches dense individual segments
     (the actual CPS-method-size proxy)

Stays silent on pass; only outputs feedback when either threshold
is crossed. Exits 2 (blocking) on HARD; exits 0 with stderr advisory
on WARN.

Hook Type: PostToolUse
Matcher: Edit|Write
"""

import json
import re
import sys
from pathlib import Path

# Total-pipeline thresholds (gross sanity check)
TOTAL_WARN = 55000
TOTAL_HARD = 65000

# Per-script-block thresholds — the actual CPS method size proxy.
# Each `script { ... }` body in a Jenkinsfile compiles to one CPS method.
# Calibration from real builds (2026-05-09):
#   • elohim/holochain/Jenkinsfile @ 10656 source bytes — compiles (#1210)
#   • genesis/orchestrator/Jenkinsfile @ 13724 source bytes — FAILED #923
#     with WorkflowScript.___cps___74 too large
# So the danger zone for a single script body starts somewhere between
# 11KB and 13KB source. We set HARD at 11000 (above largest known-good)
# and WARN at 9000 (give margin to refactor before push).
SCRIPT_WARN = 9000
SCRIPT_HARD = 11000


def find_pipeline_block(text: str) -> tuple[int, int] | None:
    """Return (start_pos, end_pos) of the top-level `pipeline { ... }` block.
    start_pos points at `{`; end_pos at the matching `}`. None if no block.
    """
    m = re.search(r"^pipeline\s*\{", text, re.MULTILINE)
    if not m:
        return None
    open_pos = m.end() - 1
    depth = 0
    for i in range(open_pos, len(text)):
        ch = text[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return (open_pos, i)
    return None


def script_block_sizes(text: str, start: int, end: int) -> list[dict]:
    """Find every `script { ... }` block body inside the pipeline range.
    Each compiles to a single CPS method, so each is independently
    capped at 64KB JVM bytecode. Returns size descending.
    """
    sizes: list[dict] = []
    for m in re.finditer(r"\bscript\s*\{", text):
        if m.start() < start or m.end() > end:
            continue
        open_pos = m.end() - 1
        depth = 0
        for i in range(open_pos, end + 1):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    body_size = i - open_pos + 1
                    line = text[:open_pos].count("\n") + 1
                    sizes.append({
                        "size": body_size,
                        "line": line,
                    })
                    break
    sizes.sort(key=lambda d: d["size"], reverse=True)
    return sizes


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0

    tool_input = payload.get("tool_input") or {}
    file_path = tool_input.get("file_path") or ""

    if "Jenkinsfile" not in Path(file_path).name:
        return 0

    try:
        text = Path(file_path).read_text(encoding="utf-8")
    except (FileNotFoundError, PermissionError, UnicodeDecodeError):
        return 0

    block = find_pipeline_block(text)
    if block is None:
        return 0  # library/shared file, no pipeline {}

    start, end = block
    total_size = end - start + 1
    scripts = script_block_sizes(text, start, end)
    largest_script = scripts[0] if scripts else None

    # Decide verdict — failure on either total or per-script threshold.
    # Per-script is the dominant signal (each is its own CPS method);
    # total catches edge cases like maps of inline closures.
    total_fail = total_size >= TOTAL_HARD
    total_warn = total_size >= TOTAL_WARN
    script_fail = largest_script is not None and largest_script["size"] >= SCRIPT_HARD
    script_warn = largest_script is not None and largest_script["size"] >= SCRIPT_WARN

    if total_fail or script_fail:
        msg = [f"\n❌ {file_path}: Jenkinsfile method-size HARD limit exceeded"]
        if total_fail:
            msg.append(
                f"   • pipeline {{}} block: {total_size} bytes "
                f"(HARD {TOTAL_HARD})"
            )
        if script_fail:
            ls = largest_script
            msg.append(
                f"   • largest script {{}} body: {ls['size']} bytes "
                f"at line {ls['line']} (HARD {SCRIPT_HARD})"
            )
        msg.append(
            "   Each `script { }` body compiles to its own CPS method "
            "capped at 64KB JVM bytecode."
        )
        msg.append(
            "   Refactor: extract logic into `def helperFn()` ABOVE `pipeline {`. "
            "Call from `script {}` with one-line invocations."
        )
        print("\n".join(msg), file=sys.stderr)
        return 2

    if total_warn or script_warn:
        msg = [f"\n⚠️  {file_path}: Jenkinsfile method-size WARN"]
        if total_warn:
            msg.append(
                f"   • pipeline {{}} block: {total_size} bytes "
                f"(WARN {TOTAL_WARN}, HARD {TOTAL_HARD})"
            )
        if script_warn:
            ls = largest_script
            msg.append(
                f"   • largest script {{}} body: {ls['size']} bytes "
                f"at line {ls['line']} (WARN {SCRIPT_WARN}, HARD {SCRIPT_HARD})"
            )
        msg.append("   Consider extracting helpers above `pipeline {` soon.")
        print("\n".join(msg), file=sys.stderr)
        return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
