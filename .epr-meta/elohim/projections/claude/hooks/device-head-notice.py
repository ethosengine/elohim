#!/usr/bin/env python3
"""Prompt adapter: the payload or environment names this session directly."""
import importlib.util
import json
import os
from pathlib import Path
import sys

root = Path(__file__).resolve().parents[2]
try:
    payload = json.load(sys.stdin)
except (ValueError, OSError):
    payload = {}
# The installed command names the nearest runtime; its payload names that session.
# An ownerless declaration is only a fallback when the runtime supplies no session.
def nonblank(value):
    return value.strip() if isinstance(value, str) else ""


declared = nonblank(os.environ.get("ELOHIM_SESSION_ID"))
owner = nonblank(os.environ.get("ELOHIM_SESSION_OWNER_PID"))
parent = nonblank(os.environ.get("ELOHIM_HOOK_RUNTIME_PID", str(os.getppid())))
owner_matches = (
    owner.isascii() and owner.isdecimal() and 0 < int(owner) <= 4294967295
    and parent.isascii() and parent.isdecimal() and int(owner) == int(parent)
)
explicit = declared if owner_matches else ""
ownerless = declared if not owner else ""
runtime_keys = ("CLAUDE_SESSION_ID", "CLAUDE_CODE_SESSION_ID", "CODEX_THREAD_ID", "CODEX_SESSION_ID")
session = explicit or nonblank(payload.get("session_id")) or ownerless or next(
    (value for key in runtime_keys if (value := nonblank(os.environ.get(key)))), ""
)

spec = importlib.util.spec_from_file_location("head_notice", root / "genesis/agentic/bin-head-notice.py")
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
for line in module.notices(os.environ.get("BIN_STORE", "/projects/.claude-config/tools/bin-store"), session):
    print(line)
