#!/usr/bin/env python3
"""PostToolUse observer that turns task brief/report frontmatter into valueflow records."""
from __future__ import annotations

import fnmatch
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import yaml

_here = Path(__file__).resolve()
for _ in range(8):
    if (_here / ".claude" / "scripts" / "_lib").is_dir():
        sys.path.insert(0, str(_here / ".claude" / "scripts"))
        break
    _here = _here.parent
from _lib import frontmatter as fm  # noqa: E402

CONVENTION = (
    "[valueflow-observer] convention: task-*-brief.md frontmatter requires gap and actor; "
    "task-*-report.md requires gap, actor, status, and optional commits: [sha, ...]"
)
DISCHARGING = {"DONE", "DONE_WITH_CONCERNS"}
OBSERVATIONS = {"NEEDS_CONTEXT", "BLOCKED", "HOLD"}


def _repo_root() -> Path:
    configured = os.environ.get("CLAUDE_PROJECT_DIR")
    return Path(configured).resolve() if configured else _here


def _label(path: Path, root: Path) -> str:
    try:
        return path.resolve().relative_to(root).as_posix()
    except (OSError, ValueError):
        return str(path)


def _read_document(path: Path) -> tuple[dict, str] | None:
    try:
        parsed = fm.parse_file(path)
    except OSError:
        return None
    if not parsed.raw_block:
        return None
    try:
        fields = yaml.safe_load(parsed.raw_block) or {}
    except yaml.YAMLError:
        return None
    return (fields, parsed.body) if isinstance(fields, dict) else None


def _epr(root: Path, args: list[str]) -> None:
    executable = shutil.which("epr")
    if executable is None:
        print("epr: command not found")
        return
    try:
        result = subprocess.run(
            [executable, *args],
            cwd=root,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            timeout=8,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        print(str(error))
        return
    if result.stdout:
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n")


def _first_body_line(body: str) -> str:
    return next((line.strip() for line in body.splitlines() if line.strip()), "(empty report body)")


def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except (json.JSONDecodeError, ValueError):
        return 0
    edited = (payload.get("tool_input") or {}).get("file_path")
    if not isinstance(edited, str) or not edited:
        return 0

    basename = Path(edited).name
    is_brief = fnmatch.fnmatchcase(basename, "task-*-brief.md")
    is_report = fnmatch.fnmatchcase(basename, "task-*-report.md")
    if not is_brief and not is_report:
        return 0

    root = _repo_root()
    path = Path(edited)
    if not path.is_absolute():
        path = root / path
    document = _read_document(path)
    if document is None:
        print(CONVENTION)
        return 0
    fields, body = document
    required = ("gap", "actor") if is_brief else ("gap", "actor", "status")
    if any(not isinstance(fields.get(key), str) or not fields[key].strip() for key in required):
        print(CONVENTION)
        return 0

    gap = fields["gap"].strip()
    actor = fields["actor"].strip()
    label = _label(path, root)
    if is_brief:
        _epr(root, ["flow", "claim", "--on", gap, "--as", actor, "--brief", label, "--json"])
        return 0

    status = fields["status"].strip().upper()
    if status not in DISCHARGING | OBSERVATIONS:
        print(CONVENTION)
        return 0
    if status in OBSERVATIONS:
        _epr(
            root,
            [
                "flow",
                "note",
                "--on",
                gap,
                "--kind",
                "observation",
                "--reason",
                f"{status}: {_first_body_line(body)}",
                "--as",
                actor,
                "--json",
            ],
        )
        return 0

    commits = fields.get("commits", [])
    if commits is None:
        commits = []
    if not isinstance(commits, list) or not all(isinstance(commit, str) for commit in commits):
        print(CONVENTION)
        return 0
    args = ["flow", "fulfill", "--on", gap, "--report", label, "--status", status]
    for commit in commits:
        args.extend(("--commit", commit))
    args.extend(("--as", actor, "--json"))
    _epr(root, args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
