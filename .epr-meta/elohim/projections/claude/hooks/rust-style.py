#!/usr/bin/env python3
"""Expose native Rust style findings before edits; reach gates cover every writer."""
import json
from pathlib import Path
import subprocess
import sys

RETIRE_WHEN = "all supported runtimes expose the native style gate as a trusted built-in edit check"


def inspect(root, event):
    tool = event.get("tool_name", "")
    data = event.get("tool_input", {})
    if isinstance(data, str):
        # Codex freeform apply_patch and exec events carry a string, not a dict.
        data = {"command": data}
    if not isinstance(data, dict):
        return None
    path = data.get("file_path") or data.get("path")
    if not isinstance(path, str) or not path.endswith(".rs"):
        # Shell and opaque orchestration calls cannot be reconstructed safely.
        # Their committed output is checked independently by pre-push and CI.
        command = data.get("command", data.get("cmd", data.get("code", "")))
        if isinstance(command, str) and ("cargo " in command or ".rs" in command):
            return {"context": "Rust style RS-GOV-04 is active: no new unexplained suppressions; touched crates with debt must reduce it. Run just gate rust-style before reach. This tool's prospective source is unavailable; it has not been certified clean."}
        return None
    target = Path(path)
    if not target.is_absolute():
        target = root / target
    try:
        relative = target.resolve().relative_to(root.resolve()).as_posix()
    except ValueError:
        return None
    if tool in ("Write", "write_file"):
        content = data.get("content")
    elif tool in ("Edit", "edit_file"):
        before = target.read_text()
        old, new = data.get("old_string"), data.get("new_string")
        if not isinstance(old, str) or not old or not isinstance(new, str):
            return {"context": "Rust style: prospective edit is unmeasured; run just gate rust-style."}
        count = before.count(old)
        if count == 0 or (count > 1 and not data.get("replace_all")):
            return {"context": "Rust style: edit context is ambiguous; native tool and reach gate will validate it."}
        content = before.replace(old, new, -1 if data.get("replace_all") else 1)
    else:
        return {"context": "Rust style RS-GOV-04 applies to this crate. Run just gate rust-style; this tool's prospective source has not been measured."}
    if not isinstance(content, str):
        return {"context": "Rust style: prospective content is unmeasured."}
    result = subprocess.run(
        ["bash", str(root / "genesis/orchestrator/rust-style.sh"), "--hook", "--path", relative, "--stdin"],
        input=content, text=True, capture_output=True, timeout=4,
    )
    if result.returncode not in (0, 1):
        return {"context": "Rust style UNMEASURED: " + result.stderr.strip()[:700]}
    payload = json.loads(result.stdout)
    failures = payload.get("failures")
    if not isinstance(failures, list):
        return {"context": "Rust style UNMEASURED: checker returned no verdict."}
    if failures:
        return {"deny": "\n".join(failures[:8]) + "\nSee genesis/docs/rust-style-guide.md#rs-gov-04."}
    return {"context": "Rust style: proposed attributes pass RS-GOV-04 (all cfg branches; cached checker binary passed freshness checks). Crate debt/paydown and compiler checks remain required at reach; just gate rust-style."}


def main():
    root = Path(__file__).resolve().parents[2]
    try:
        event = json.loads(sys.stdin.buffer.read(1572864))
        result = inspect(root, event)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        result = {"context": f"Rust style UNMEASURED ({type(error).__name__}); run just gate rust-style before reach."}
    if result:
        output = {"hookEventName": "PreToolUse"}
        if "deny" in result:
            output.update(permissionDecision="deny", permissionDecisionReason=result["deny"])
        else:
            output["additionalContext"] = result["context"]
        print(json.dumps({"hookSpecificOutput": output}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
