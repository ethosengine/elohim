"""Capture-surface regression tests; all ledger writes use pytest's temporary root."""
import importlib.util
import io
import json
from pathlib import Path

import pytest


HOOK = Path(__file__).resolve().parents[3] / "hooks" / "deprecation-sentinel.py"
spec = importlib.util.spec_from_file_location("sentinel", HOOK)
sentinel = importlib.util.module_from_spec(spec)
spec.loader.exec_module(sentinel)


@pytest.mark.parametrize("command,allowed", [
    ("pnpm run lint", True),
    ("cd app && pnpm lint", True),
    ('RUSTFLAGS="" cargo clippy', True),
    ('env RUSTFLAGS="" cd "app dir" && cd child && pnpm lint', True),
    ("python3 -m pytest tests", True),
    ("python3 -m pip install example", True),
    ("python3 -m unittest genesis.agentic.berth_test", True),
    ("python3 report.py", False),
    ("git push origin HEAD:dev", True),
    ("git commit -m msg", True),
    ("S=/tmp/x; tail -1 $S/log | grep -o EXIT", False),
    ("bash scripts/ci/build.sh", True),
    ("sh ./scripts/ci/build.sh", True),
    ("bash scripts/report.sh", False),
    ("bash scripts/ci/../report.sh", False),
    ("bash -c 'cat report.md'", False),
    ("cat report.md && pnpm lint", False),
    ("cat report.md | pnpm lint", False),
    ("cd app; pnpm lint", True),
    ('cd /projects/elohim\necho "=== eslint ==="\npnpm -C app/elohim-app exec eslint src/a.ts', True),
    ('cd app && echo "=== PRETTIER ===" && pnpm exec prettier --check src', True),
    ('cd app && timeout 300 pnpm run lint 2>&1 > /tmp/x/lint.txt; grep -c warn /tmp/x/lint.txt', False),
    ('grep -n "Tier" app/x/identity.model.ts | head -40\necho done', False),
    ("for f in a b; do pnpm -C $f lint; done", True),
    ("SP=/tmp/s; cargo clippy -j1 2>&1 | tail -20", True),
    ("time just gate elohim-sdk", True),
    ("git log --oneline -5", False),
    ("echo hello", False),
    ("pnpm lint 2>&1 | tail -3", True),
    ("pnpm lint | tail -3 saved.log", False),
    ("pnpm lint; cat saved.log", False),
    ("pnpm lint | awk '{print $1}'", True),
    ("cd", False),
    ("cd app\npnpm lint", True),
    ("env -u DEBUG exec command nice -n 5 timeout -k 2s --signal TERM 300 sudo -u builder pnpm lint", True),
    ("if true; then pnpm lint; else echo skipped; fi", True),
    ("while true; do pnpm lint; done", True),
    ("until true; do pnpm lint; done", True),
    ("case $x in a) pnpm lint;; esac", True),
    ("function build() { pnpm lint; }", True),
    ("(pnpm lint) & echo done", True),
    ("pnpm lint || echo failed", True),
    ("", False),
    ('cd "unfinished', False),
] + [(f"{reader} report.md", False) for reader in
     "cat sed grep rg find head tail less wc git-show git-diff git-log git-status ls tree jq yq awk".split()]
  + [(f"git {subcommand}", False) for subcommand in "show diff log status blame grep".split()])
def test_command_surface(command, allowed):
    assert sentinel._command_can_capture(command) is allowed


@pytest.mark.parametrize("command,line,captures", [
    ("cat report.md", "GHSA-xxxx-yyyy-zzzz", 0),
    ("pnpm run lint", "npm warn deprecated old-package@1: use the replacement", 1),
    ("cd app && pnpm lint", "npm warn deprecated old-package@1: use the replacement", 1),
    ('RUSTFLAGS="" cargo clippy', "warning: use of deprecated function old_api", 1),
])
def test_capture_through_main(tmp_path, monkeypatch, command, line, captures):
    report = tmp_path / "report.md"
    report.write_text(line)
    monkeypatch.setenv("CLAUDE_PROJECT_DIR", str(tmp_path))
    monkeypatch.setattr(sentinel.sys, "stdin", io.StringIO(json.dumps({
        "tool_name": "Bash", "tool_input": {"command": command},
        "tool_response": {"stdout": report.read_text()},
    })))
    sentinel.main()
    ledger = tmp_path / ".claude/data/deprecations.jsonl"
    rows = [json.loads(row) for row in ledger.read_text().splitlines()] if ledger.exists() else []
    assert len(rows) == captures
    if rows:
        assert rows[0]["cmd"] == command
        assert rows[0]["line"] == line
