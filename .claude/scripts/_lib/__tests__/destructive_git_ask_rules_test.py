#!/usr/bin/env python3
"""Every form in the destructive-git table makes Claude Code ask first.

Operator ruling 2026-10-25: until our own embedded classifier exists, destructive git is guarded
by Claude Code's permission system — `.claude/settings.json` `permissions.ask`, which applies to
every agent and subagent and still prompts under auto mode — not by the tier gate hook, which
denied every session because none resolved a tier. The table itself stays in one place, the
`destructive-git-requires-tier@1` row of `.claude/epr-meta/policies.yaml`; this test derives one
example command per row rule (plain and under `git -C <dir>`) and fails when no ask rule catches
it, so the settings list cannot silently fall behind the table.
"""

import json
import re
import sys
from pathlib import Path

import yaml

REPO = Path(__file__).resolve().parents[4]
POLICY = "destructive-git-requires-tier"


def table() -> dict:
    rows = yaml.safe_load((REPO / ".claude/epr-meta/policies.yaml").read_text(encoding="utf-8"))
    rows = rows.get("policies", rows) if isinstance(rows, dict) else rows
    return next(r for r in rows if isinstance(r, dict) and r.get("id") == POLICY)["parameters"]


def examples(rule: dict) -> list[str]:
    sub = rule["sub"]
    if "min_positional_args" in rule:
        return [f"{sub} HEAD refs/heads/x"]
    words: list[str] = []
    for flag in rule.get("any_flags", []):
        words.append(flag)
    for letter in rule.get("flag_letters", []):
        words.append(f"-{letter}d")
    for prefix in rule.get("any_arg_prefix", []):
        words.append(f"origin {prefix}HEAD:dev")
    args = [a for a in rule.get("any_args", []) if a != "*"]
    if words and args:  # several keys on one rule AND together
        return [f"{sub} {args[0]} {w} x" if sub == "worktree" else f"{sub} {w} {args[0]}"
                for w in words]
    if words:
        return [f"{sub} {w} x" for w in words]
    if args:
        return [f"{sub} {a}" for a in args]
    return [f"{sub} x"]


def matcher(rules: list[str]):
    pats = [re.compile("^" + ".*".join(map(re.escape, r[len("Bash("):-1].split("*"))) + "$")
            for r in rules if r.startswith("Bash(")]
    return lambda command: any(p.match(command) for p in pats)


def main() -> int:
    asks = json.loads((REPO / ".claude/settings.json").read_text(encoding="utf-8"))
    asks = asks.get("permissions", {}).get("ask", [])
    asked = matcher(asks)
    params = table()
    failures = []
    for rule in params["rules"]:
        for tail in examples(rule):
            for command in (f"git {tail}", f"git -C /projects/elohim {tail}"):
                if not asked(command):
                    failures.append(command)
    # A literal `*` target cannot be written as a permission pattern without asking for every
    # `rm -r`; auto mode's classifier still reads it.
    for target in params["rm-force-recursive-targets"]:
        if target != "*" and not asked(f"rm -rf {target}"):
            failures.append(f"rm -rf {target}")
    for command in ("git status --porcelain", "git push origin dev", "git branch -d feat/x",
                    "git restore src/lib.rs", "git worktree remove .worktrees/x"):
        if asked(command):
            failures.append(f"over-asks: {command}")
    for failure in failures:
        print(f"FAIL {failure}")
    print(f"{len(params['rules'])} table rules against {len(asks)} ask rules: "
          f"{'ok' if not failures else f'{len(failures)} failing'}")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
