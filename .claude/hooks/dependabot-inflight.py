#!/usr/bin/env python3
"""Non-blocking local PostToolUse context for dependency edits (no network).

Retire when native epr flow sense owns the poll, reconciliation and paydown
judgement. This transport reads the JSONL only and never records closure.
"""
import json
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from _lib.dependabot_inflight import context


def main():
    try:
        root = Path(os.environ.get("CLAUDE_PROJECT_DIR") or Path(__file__).resolve().parents[2])
        message = context(root, json.loads(sys.stdin.buffer.read(1572864)))
        if message:
            print(json.dumps({"hookSpecificOutput": {"hookEventName": "PostToolUse",
                                                    "additionalContext": message}}))
    except Exception:
        pass  # Advisory transport is fail-open, including malformed tool input.
    return 0


if __name__ == "__main__":
    sys.exit(main())
