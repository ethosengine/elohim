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

import sys
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


def main() -> int:
    args = set(sys.argv[1:])
    text, errs = render(_ROOT)

    if "--census" in args:
        habits, _ = eh.census(_ROOT)
        for h in habits:
            src = Path(h["_source"]).relative_to(_ROOT).as_posix()
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
        current = TARGET.read_text(encoding="utf-8") if TARGET.is_file() else ""
        if current != text:
            print(f"{TARGET.relative_to(_ROOT)} is STALE — run "
                  f".claude/scripts/habits-project.py", file=sys.stderr)
            return 1
        print(f"{TARGET.relative_to(_ROOT)} is current ({text.count(chr(10)) + 1} lines)")
        return 0

    TARGET.write_text(text, encoding="utf-8")
    print(f"projected {len(eh.census(_ROOT)[0])} habits -> {TARGET.relative_to(_ROOT)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
