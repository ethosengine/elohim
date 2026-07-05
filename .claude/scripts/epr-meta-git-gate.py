#!/usr/bin/env python3
"""Thin CLI for the harness-agnostic .epr-meta compose-gate — sibling to ci-ignore-projector.py.
All logic lives in _lib.epr_meta_git (tested under _lib/__tests__). Invoked by
.husky/{pre-commit,pre-push} and (later) a CI stage, so the SAME pure rule engine governs every
author's commit — Codex, Gemini, a human, or Claude driving git — not just Claude's tool edits.

Usage: epr-meta-git-gate.py [--staged | --range A..B]
Exit 0 = allow (advisories to stderr); 1 = block (deny, or ask without EPR_META_ACK=1).
Bypass: EPR_META_ACK=1 downgrades ask->allow for THIS gate; git --no-verify skips ALL hooks.
Fail-open by design (callers guard on `command -v python3`); absence never blocks a commit."""
import os
import sys
from pathlib import Path

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent

from _lib import epr_meta_git  # noqa: E402


def main(argv: list[str]) -> int:
    ack = os.environ.get("EPR_META_ACK") == "1"
    if "--staged" in argv:
        code, msgs = epr_meta_git.run("staged", None, ack=ack)
    elif "--range" in argv:
        i = argv.index("--range")
        rng = argv[i + 1] if i + 1 < len(argv) else "HEAD~1..HEAD"
        code, msgs = epr_meta_git.run("range", rng, ack=ack)
    else:
        print("usage: epr-meta-git-gate.py [--staged | --range A..B]", file=sys.stderr)
        return 0  # unknown invocation -> fail-open
    for m in msgs:
        print(m, file=sys.stderr)
    return code


if __name__ == "__main__":
    try:
        sys.exit(main(sys.argv[1:]))
    except SystemExit:
        raise
    except Exception as e:  # fail-open: an internal error is NOT a block
        print(f"epr-meta-git-gate internal error: {e}", file=sys.stderr)
        sys.exit(0)
