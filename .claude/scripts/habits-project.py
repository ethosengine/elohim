#!/usr/bin/env python3
"""habits-project.py — project the habit census into `genesis/manifests/habits.yaml`.

THE REGISTER IS A PROJECTION, NEVER A HOME. A habit is declared in the `.epr-meta` governance
package of the directory whose behaviour it describes (`<dir>/.epr-meta/<id>.habit.md`), so its
scope is derived from where it lives — the same authority the compose-gate already resolves. This
script renders that census into the flat file every existing reader knows by path: the a2o
declared-concern denominator (`genesis/a2o/scripts/lib/declared-concerns.ts`), `saga-status.py`,
`ci-harvest.py`, `latency-scoreboard.py`, `_lib/seam_forecast.py`.

Two hand-written homes for one truth is a failure mode this repo has already paid for twice
(`cluster-state.yaml` vs `ELOHIM_REMOTE_COMPUTE_STATUS`; the `deployments.json` `suspended` flags
that drifted until they were made derived). So the generated file says so in its first line, and
`--check` fails when it drifts — the same freshness discipline the schema codegen and the memory
index already run under.

    habits-project.py            # write the projection
    habits-project.py --check    # exit 1 if the projection is stale or the census is invalid
    habits-project.py --census   # list what is declared, and where
"""
from __future__ import annotations

import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path

_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(_ROOT / ".claude" / "scripts"))

from _lib import epr_habits as eh  # noqa: E402

TARGET = _ROOT / "genesis" / "manifests" / "habits.yaml"

BANNER = (
    "# GENERATED — DO NOT EDIT. Projected from the habit atoms in the .epr-meta governance\n"
    "# packages by .claude/scripts/habits-project.py. Edit a habit where it is DECLARED:\n"
    "#   <dir>/.epr-meta/<id>.habit.md   (frontmatter = declaration, body = evidence ledger)\n"
    "# The covenant, the vision and the priority order live in .epr-meta/habits-covenant.md.\n"
    "# `habits-project.py --check` fails when this file drifts from the tree.\n"
    "#"
)


def render(root: Path) -> tuple[str, list[str]]:
    habits, errs = eh.census(root)
    covenant, prose, raw = eh.load_covenant(root)
    header = BANNER + "\n" + "\n".join(
        (f"# {ln}" if ln.strip() else "#") for ln in prose.strip("\n").split("\n"))
    return eh.project(habits, header, raw), errs


def _timestamp(value: str) -> datetime | None:
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00"))
    except (AttributeError, TypeError, ValueError):
        return None


def _committed_statuses(root: Path) -> dict[str, str]:
    result = subprocess.run(
        ["git", "show", "HEAD:genesis/manifests/habits.yaml"],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0 or eh.yaml is None:
        return {}
    try:
        document = eh.yaml.safe_load(result.stdout) or {}
    except Exception:  # noqa: BLE001 — a malformed committed projection is handled as no baseline
        return {}
    rows = document.get("habits", []) if isinstance(document, dict) else []
    return {
        row["id"]: row["status"]
        for row in rows
        if isinstance(row, dict)
        and isinstance(row.get("id"), str)
        and isinstance(row.get("status"), str)
    }


def _last_commit_at(root: Path, path: Path) -> datetime | None:
    try:
        label = path.resolve().relative_to(root.resolve()).as_posix()
    except (OSError, ValueError):
        return None
    result = subprocess.run(
        ["git", "log", "-1", "--format=%cI", "--", label],
        cwd=root,
        capture_output=True,
        text=True,
        check=False,
    )
    return _timestamp(result.stdout.strip()) if result.returncode == 0 else None


def _latest_rulings(root: Path) -> dict[str, datetime]:
    ledger = root / ".eprfs" / "status" / "flows.jsonl"
    if not ledger.is_file():
        return {}
    latest: dict[str, datetime] = {}
    for line in ledger.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            record = json.loads(line).get("record", {})
        except (json.JSONDecodeError, AttributeError):
            continue
        labels = record.get("classifiedAs")
        occurred_at = _timestamp(record.get("occurredAt"))
        if (
            record.get("kind") != "event"
            or record.get("action") != "cite"
            or not isinstance(labels, list)
            or len(labels) < 2
            or labels[0] != "run:ruling"
            or not isinstance(labels[1], str)
            or occurred_at is None
        ):
            continue
        prior = latest.get(labels[1])
        if prior is None or occurred_at > prior:
            latest[labels[1]] = occurred_at
    return latest


def status_flip_errors(root: Path, habits: list[dict]) -> list[str]:
    """Require a post-commit ruling for every status change against HEAD's projection."""
    committed = _committed_statuses(root)
    rulings = _latest_rulings(root)
    errors: list[str] = []
    for habit in habits:
        habit_id = habit.get("id")
        new_status = habit.get("status")
        old_status = committed.get(habit_id)
        source = habit.get("_source")
        if (
            not isinstance(habit_id, str)
            or not isinstance(new_status, str)
            or old_status is None
            or old_status == new_status
            or not isinstance(source, Path)
        ):
            continue
        try:
            label = source.resolve().relative_to(root.resolve()).as_posix()
        except (OSError, ValueError):
            continue
        last_commit = _last_commit_at(root, source)
        ruling = rulings.get(label)
        if last_commit is None or ruling is None or ruling <= last_commit:
            errors.append(
                f"FLIP-WITHOUT-RULING {habit_id} {old_status}->{new_status}: record it with "
                f"epr flow note --on {label} --kind ruling --reason '<evidence>'"
            )
    return errors


def main(root: Path = _ROOT, argv: list[str] | None = None) -> int:
    args = set(sys.argv[1:] if argv is None else argv)
    target = root / "genesis" / "manifests" / "habits.yaml"
    text, errs = render(root)

    if "--census" in args:
        habits, _ = eh.census(root)
        for h in habits:
            src = Path(h["_source"]).relative_to(root).as_posix()
            flag = " ACTIVE" if h.get("active") else ""
            print(f"{str(h.get('status')):<8}{flag:<8}{str(h.get('id')):<36} {src}")
        for e in errs:
            print(f"  ! {e}", file=sys.stderr)
        return 1 if errs else 0

    if errs:
        print("habit census is INVALID — refusing to project:", file=sys.stderr)
        for e in errs:
            print(f"  - {e}", file=sys.stderr)
        return 1

    if "--check" in args:
        habits, _ = eh.census(root)
        flip_errors = status_flip_errors(root, habits)
        if flip_errors:
            for error in flip_errors:
                print(error, file=sys.stderr)
            return 1
        current = target.read_text(encoding="utf-8") if target.is_file() else ""
        if current != text:
            print(f"{target.relative_to(root)} is STALE — run "
                  f".claude/scripts/habits-project.py", file=sys.stderr)
            return 1
        print(f"{target.relative_to(root)} is current ({text.count(chr(10)) + 1} lines)")
        return 0

    target.write_text(text, encoding="utf-8")
    print(f"projected {len(eh.census(root)[0])} habits -> {target.relative_to(root)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
