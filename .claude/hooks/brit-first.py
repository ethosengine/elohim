#!/usr/bin/env python3
"""Brit habit friction for visible shell commands; not a security boundary."""
import importlib.util
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("command_parser", ROOT / ".claude/hooks/capability-tier-gate.py")
parser = importlib.util.module_from_spec(spec)
spec.loader.exec_module(parser)


def uses_git(command, depth=0):
    if depth > 4:
        return False
    for chain in parser._chains(command):
        for stage in chain:
            argv = parser._strip_wrappers(stage)
            if not argv:
                continue
            head = Path(argv[0]).name
            if head == "git":
                return True
            if head in ("bash", "sh", "zsh", "dash"):
                index = parser._find_dash_c(argv[1:])
                if index is not None and index + 2 < len(argv):
                    if uses_git(argv[index + 2], depth + 1):
                        return True
    return False


def evaluate(event):
    config = json.loads((ROOT / ".epr-meta/brit-first.json").read_text())
    if config.get("enabled") is not True:
        return None
    name = event.get("tool_name", "")
    if name not in ("Bash", "exec_command", "functions.exec_command", "shell_command"):
        return None
    args = event.get("tool_input", {})
    command = args.get("command", args.get("cmd", ""))
    if not isinstance(command, str) or not uses_git(command):
        return None
    # A leading comment belongs only to this invocation, never ambient environment.
    first = command.splitlines()[0].strip()
    prefix = "# Brit fallback: "
    if first.startswith(prefix) and len(first[len(prefix):].strip()) >= 12:
        return {"additionalContext": "Brit fallback recorded in this tool call: " + first[len(prefix):]}
    return {"permissionDecision": "deny", "permissionDecisionReason":
        "Use Brit for this repository operation. Check brit <command> --help first. "
        "If a specific capability is missing or failed, explain it in commentary and retry "
        "with a leading '# Brit fallback: <specific reason>' comment on this command. "
        "Inspect state before retrying a failed mutation. No human approval is needed."}


def main():
    try:
        raw = sys.stdin.buffer.read(1572865)
        if len(raw) > 1572864:
            raise ValueError("event too large")
        result = evaluate(json.loads(raw))
    except (OSError, ValueError, TypeError, AttributeError):
        result = {"additionalContext": "Brit reminder coverage gap: shell event could not be decoded."}
    if result:
        print(json.dumps({"hookSpecificOutput": {"hookEventName": "PreToolUse", **result}}))
    return 0


if __name__ == "__main__":
    sys.exit(main())
