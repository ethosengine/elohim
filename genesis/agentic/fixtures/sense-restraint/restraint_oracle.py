"""Restraint oracle for a sense-serving shift: detection AND restraint, on the real probe.

A shift that drains one `limit-models@1` candidate must leave a test a fresh reader can use to
recognize both recurrence and legitimate variation (autonomous-loop notes §9). Removing a
duplicate can preserve behaviour, so a behavioural test alone will not catch its return; this
oracle is the structural half. In a throwaway committed repo it:

  1. plants a BYPASS (a level judged against a hard watermark by hand) and an ADAPTER with the
     same textual shape, then rules the adapter `intentional` with its contract difference;
  2. senses with the REAL `limit_models.py` probe through `epr flow sense` and requires the bypass
     open and the adapter excepted and named — never silently skipped;
  3. migrates the bypass (asks a native verdict instead) and requires it healed;
  4. reintroduces the bypass and requires it re-opened, while the adapter stays excepted.

Exit 0 only when all four hold. Needs an `epr` with `flow sense` ($EPR_BIN, else PATH).
Nothing touches the shared household or the host store: the repo is a temporary directory.
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[3]
PROBES = REPO / ".claude/scripts/probes"
MEASURE = "limit-models@1"
sys.path.insert(0, str(REPO / ".claude/scripts"))
from _lib import epr_client  # noqa: E402

REGISTRY = """measures-version: 1
measures:
  - id: limit-models
    version: 1
    family: substrate-bypass
    unit: files
    probe:
      run: "python3 -I .claude/scripts/probes/limit_models.py"
      emits: candidates
    default-authority: observation
    status: active
  - id: limit-models-reset
    version: 1
    family: substrate-bypass
    unit: resets
    default-authority: observation
    status: active
lenses:
  - id: limit-models-open
    version: 1
    class: measure
    derive: distinct-subjects-since-reset
    reset: limit-models-reset@1
    consumes: [limit-models@1]
    compare: at-or-above
    hard: 1
    status: active
"""

BYPASS = "def judge(seconds, ceiling):\n    return 'hard' if seconds >= ceiling.hard else 'ok'\n"
MIGRATED = "def judge(verdict):\n    return verdict  # read from `epr flow report --breaches`\n"
ADAPTER = (
    "# Actuator, not a reading: decides in-process before a tool runs, with no epr binary.\n"
    "def deny(pct, hard):\n    return pct >= hard\n"
)


def epr() -> str:
    found = epr_client.resolve_binary()
    if not found:
        sys.exit("restraint oracle: no `epr` on PATH and EPR_BIN unset")
    return found


def run(root: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(args, cwd=root, capture_output=True, text=True, check=False)


def git(root: Path, *args: str) -> None:
    env = {
        **os.environ,
        "GIT_AUTHOR_NAME": "Oracle",
        "GIT_AUTHOR_EMAIL": "oracle@example.test",
        "GIT_COMMITTER_NAME": "Oracle",
        "GIT_COMMITTER_EMAIL": "oracle@example.test",
    }
    subprocess.run(["git", *args], cwd=root, env=env, check=True, capture_output=True)


def sense(root: Path) -> dict:
    out = run(root, epr(), "flow", "sense", "--measure", MEASURE, "--json", "--root", str(root))
    if out.returncode != 0:
        sys.exit(f"restraint oracle: sense failed: {out.stderr.strip() or out.stdout.strip()}")
    [outcome] = json.loads(out.stdout)
    return outcome


def check(label: str, ok: bool, outcome: dict) -> None:
    print(f"{'ok' if ok else 'FAILED'}  {label}")
    if not ok:
        print(json.dumps(outcome, indent=2))
        sys.exit(1)


def main() -> None:
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / ".epr-meta").mkdir()
        (root / ".epr-meta/repository.yaml").write_text("version: 1\nagent: repo:ethosengine/elohim\n")
        (root / ".claude/epr-meta").mkdir(parents=True)
        (root / ".claude/epr-meta/measures.yaml").write_text(REGISTRY)
        (root / ".claude/epr-meta/policies.yaml").write_text("policies: []\n")
        shutil.copytree(PROBES, root / ".claude/scripts/probes")
        bypass = root / ".claude/scripts/gate_cycle_copy.py"
        adapter = root / ".claude/hooks/disk_guard_copy.py"
        adapter.parent.mkdir(parents=True)
        bypass.write_text(BYPASS)
        adapter.write_text(ADAPTER)
        git(root, "init", "-q")
        git(root, "add", "-A")
        git(root, "commit", "-qm", "fixture")

        ruled = run(
            root, epr(), "flow", "sense", "rule", "--measure", MEASURE,
            "--subject", ".claude/hooks/disk_guard_copy.py", "--class", "intentional",
            "--why", "actuator in a PreToolUse hook: must decide in-process without the epr binary",
            "--root", str(root),
        )
        if ruled.returncode != 0:
            sys.exit(f"restraint oracle: rule failed: {ruled.stderr.strip()}")

        first = sense(root)
        excepted = {e["subject"] for e in first["excepted"]}
        check("the bypass is detected", ".claude/scripts/gate_cycle_copy.py" in first["open"], first)
        check(
            "the ruled adapter is excepted and named, not skipped",
            ".claude/hooks/disk_guard_copy.py" in excepted
            and ".claude/hooks/disk_guard_copy.py" not in first["open"],
            first,
        )

        bypass.write_text(MIGRATED)
        git(root, "commit", "-qam", "migrate the bypass")
        healed = sense(root)
        check("the migrated bypass is healed", ".claude/scripts/gate_cycle_copy.py" in healed["closed"], healed)

        bypass.write_text(BYPASS)
        git(root, "commit", "-qam", "reintroduce the bypass")
        again = sense(root)
        check("the reintroduced bypass re-opens", ".claude/scripts/gate_cycle_copy.py" in again["opened"], again)
        check(
            "the adapter stays excepted through the regression",
            ".claude/hooks/disk_guard_copy.py" in {e["subject"] for e in again["excepted"]},
            again,
        )
    print("restraint oracle: detection and restraint both hold")


if __name__ == "__main__":
    main()
