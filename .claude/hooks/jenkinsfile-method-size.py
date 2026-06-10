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
# Each `script { ... }` body compiles to one CPS method, but BYTECODE DENSITY
# VARIES PER FILE (interpolation/expression density), so thresholds are
# PER-FILE empirical brackets, keyed by path suffix. Evidence:
#   • root Jenkinsfile deploy block: 9,034B compiled (#1518) / 9,898B broke
#     (#1519-#1521, ___cps___7636 — the 2026-06-10 breach, mis-attributed to
#     helper heredocs in cut 1; cut 2's block-split fixed it)
#   • genesis/orchestrator: 9,524B + 10,656B compile / 13,724B broke (#923)
#   • elohim/holochain: 10,656B compiles (#1210)
# Unknown files get the looser bracket (false HARD-blocks are worse than
# warns where density is unmeasured).
SCRIPT_THRESHOLDS = {
    # path-suffix match → (WARN, HARD)
    "/projects/elohim/Jenkinsfile": (8500, 9500),   # root app pipeline (dense blocks)
}
SCRIPT_DEFAULT = (10000, 11000)

# Per-DEF thresholds (largest single top-level `def` body above `pipeline {`,
# comment-stripped). 2026-06-10 lesson, both directions: (a) helpers are
# CPS-compiled too — the hook's old advice sent code into an unmeasured
# region; (b) the AGGREGATE region size is NOT the unit (the orchestrator's
# helper region totals ~46KB across many small defs and compiles fine —
# an aggregate check false-blocked a real 3-line bugfix there). The unit is
# the single def. Largest known-good def: 6,564B (root stageSpaBlobs heredoc
# era). No def has been observed breaking; HARD sits conservatively above
# all known-good.
DEF_WARN = 6000
DEF_HARD = 8000


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


def largest_def_body(text: str, pipeline_start: int) -> dict | None:
    """Largest single top-level `def name(...) { ... }` body above
    `pipeline {`, comment-stripped (groovy // lines cost no bytecode and are
    stripped; heredoc contents are GString constants that DO cost and are
    kept). Each def is its own CPS method — the per-method 64KB cap applies
    to the def, never to the region aggregate."""
    pre = text[:pipeline_start]
    best = None
    for dm in re.finditer(r"^def\s+(\w+)\s*\(", pre, re.MULTILINE):
        op = pre.find("{", dm.end())
        if op < 0:
            continue
        depth = 0
        for i in range(op, len(pre)):
            if pre[i] == "{":
                depth += 1
            elif pre[i] == "}":
                depth -= 1
                if depth == 0:
                    body = re.sub(r"^\s*//.*$", "", pre[op:i + 1], flags=re.MULTILINE)
                    size = len(body.encode("utf-8"))
                    line = pre[:dm.start()].count("\n") + 1
                    if best is None or size > best["size"]:
                        best = {"size": size, "name": dm.group(1), "line": line}
                    break
    return best


def script_thresholds(file_path: str) -> tuple[int, int]:
    for suffix, pair in SCRIPT_THRESHOLDS.items():
        if file_path.endswith(suffix) or suffix.endswith(file_path):
            return pair
    return SCRIPT_DEFAULT


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
    largest_def = largest_def_body(text, start)
    s_warn, s_hard = script_thresholds(file_path)

    # Decide verdict — failure on any of the three checks.
    # Per-script is the dominant in-pipeline signal (each is its own CPS
    # method, per-file calibrated); total catches maps of inline closures;
    # per-def catches the top-level helper bodies the first two structurally
    # cannot see (the unit is the single def — NEVER the region aggregate).
    total_fail = total_size >= TOTAL_HARD
    total_warn = total_size >= TOTAL_WARN
    script_fail = largest_script is not None and largest_script["size"] >= s_hard
    script_warn = largest_script is not None and largest_script["size"] >= s_warn
    def_fail = largest_def is not None and largest_def["size"] >= DEF_HARD
    def_warn = largest_def is not None and largest_def["size"] >= DEF_WARN

    if total_fail or script_fail or def_fail:
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
                f"at line {ls['line']} (HARD {s_hard} for this file; the root "
                f"deploy block broke at 9,898B — builds #1519-#1521)"
            )
        if def_fail:
            ld = largest_def
            msg.append(
                f"   • largest top-level def `{ld['name']}` body: {ld['size']} "
                f"comment-stripped bytes at line {ld['line']} (HARD {DEF_HARD})"
            )
        msg.append(
            "   Each `script { }` body AND each top-level `def` compile to "
            "their own CPS method capped at 64KB JVM bytecode."
        )
        msg.append(
            "   Refactor: split the oversized method — bash bodies move to "
            "scripts/ci/*.sh called via one-line "
            "`sh \"bash '${env.WORKSPACE}/scripts/ci/<name>.sh' …\"` (secrets "
            "via withEnv, never argv); dense groovy splits into several small "
            "defs (each is its own CPS method — that's the relief valve)."
        )
        print("\n".join(msg), file=sys.stderr)
        return 2

    if total_warn or script_warn or def_warn:
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
                f"at line {ls['line']} (WARN {s_warn}, HARD {s_hard} for this file)"
            )
        if def_warn:
            ld = largest_def
            msg.append(
                f"   • largest top-level def `{ld['name']}`: {ld['size']} "
                f"comment-stripped bytes at line {ld['line']} "
                f"(WARN {DEF_WARN}, HARD {DEF_HARD})"
            )
        msg.append(
            "   Split early: small script blocks + small defs; bash bodies "
            "to scripts/ci/*.sh."
        )
        print("\n".join(msg), file=sys.stderr)
        return 0

    return 0


if __name__ == "__main__":
    sys.exit(main())
