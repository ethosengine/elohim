#!/usr/bin/env python3
"""Evaluate the memory-frontmatter-at-birth policy as the PreToolUse hook does: the same
`_lib.epr_meta.resolve_write` the epr-meta resolver drives, on a simulated NEW-file write.

usage: memory-gate-probe.py <repository-root> <content-file>
The target path is a not-yet-existing file under the repository's `.claude/memory/`, so the
repository's own policy files (not the live checkout's) decide. Prints the verdict as JSON.
"""
import json
import sys
from pathlib import Path

root = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(root / ".claude" / "scripts"))
from _lib import epr_meta  # noqa: E402

target = root / ".claude" / "memory" / "feedback_newcomer_correction.md"
assert not target.exists(), "the probe simulates a NEW file; the target must not exist yet"
write = {
    "path": str(target),
    "content": Path(sys.argv[2]).read_text(),
    "is_new": True,
    "is_new_subdir": False,
}
r = epr_meta.resolve_write(target, write, epr_meta.find_repo_root(target))
print(json.dumps({k: r.get(k) for k in ("decision", "cls", "rule_id", "reason", "advisories")}, default=str))
