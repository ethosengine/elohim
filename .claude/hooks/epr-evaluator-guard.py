#!/usr/bin/env python3
"""Fail-open SessionStart adapter; native bin verbs own custody and installation."""
import json
import os
from pathlib import Path
import subprocess

REPO = Path(__file__).resolve().parents[2]


def _say(tool, line):
    print(f"{tool.upper()} EVALUATOR: {line}")


def _run(tool, *args):
    return subprocess.run(
        [os.environ.get("EPR_BIN") or "epr", "bin", *args, "--tool", tool],
        cwd=REPO, capture_output=True, text=True, timeout=900, check=False,
    )


def _detail(result):
    lines = (result.stderr or result.stdout).strip().splitlines()
    return lines[-1] if lines else f"exit {result.returncode}"


def main():
    try:
        registry = json.loads((REPO / "genesis/agentic/bin-tools.json").read_text())
        skipped = set(os.environ.get("BIN_STORE_SKIP_TOOLS", "").split(","))
        for tool in registry["tools"]:
            if tool in skipped:
                continue
            try:
                boot = _run(tool, "bootstrap", "--repo", str(REPO))
                if boot.returncode:
                    _say(tool, f"no verified head; launcher left untouched: {_detail(boot)}")
                    continue
                follow = _run(tool, "follow", "--repo", str(REPO))
                if follow.returncode:
                    _say(tool, f"could not follow; landed head stays: {_detail(follow)}")
                installed = _run(tool, "install-launcher", "--repo", str(REPO))
                if installed.returncode:
                    _say(tool, _detail(installed))
                    continue
                status = _run(tool, "status", "--json", "--pin")
                if status.returncode:
                    _say(tool, _detail(status))
                else:
                    view = json.loads(status.stdout)
                    _say(tool, f"verified head {view['head']} · {view['source']}")
            except Exception as error:  # noqa: BLE001 — one tool cannot block another
                _say(tool, f"guard error (ignored): {error}")
    except Exception as error:  # noqa: BLE001 — SessionStart remains fail-open
        _say("bin-store", f"guard initialization error (ignored): {error}")


if __name__ == "__main__":
    main()
