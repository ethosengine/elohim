"""Live-set ledgers (ci/deprecations/architecture/runtime findings) are read last-row-per-fp.
A recovered older row appended after dev's triaged/closed row for the same fp reopens it.
Asserts no fp's last row changed versus the merge-base with origin/dev (the integration target).
Run: python3 .claude/scripts/__tests__/live_ledger_last_row_test.py (exit 0 = pass)."""
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
LEDGERS = ["ci-findings", "deprecations", "architecture-findings", "runtime-findings"]


def git(*a):
    return subprocess.run(["git", "-C", str(ROOT), *a], capture_output=True, text=True)


def last_rows(text):
    out = {}
    for l in text.splitlines():
        if l.strip():
            try:
                r = json.loads(l)
            except ValueError:
                continue
            out[r.get("fp")] = r
    return out


base = git("merge-base", "HEAD", "origin/dev")
if base.returncode != 0:
    print("skip: no origin/dev")
    sys.exit(0)
base = base.stdout.strip()
for name in LEDGERS:
    rel = f".claude/data/{name}.jsonl"
    old = git("show", f"{base}:{rel}")
    if old.returncode != 0:
        continue
    before, after = last_rows(old.stdout), last_rows((ROOT / rel).read_text())
    changed = [fp for fp, r in before.items() if after.get(fp) != r]
    assert not changed, f"FAIL: {rel} last row changed for fps {changed[:5]}"
    print(f"  ok {rel}")
