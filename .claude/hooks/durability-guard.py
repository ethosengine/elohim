#!/usr/bin/env python3
"""SessionStart durability guard — Claude session state must land on the PVC.

Born from the 2026-08-03 unclean node reboot: a research session vanished when
the workspace pod restarted. The container's $HOME is overlay storage and is
wiped on every restart; only /projects survives. Claude writes transcripts,
history, and credentials to CLAUDE_CONFIG_DIR (falling back to ~/.claude), so
both must resolve onto /projects or the session is silently disposable.

Checks (repairs where safe, warns loudly otherwise; always exits 0 — a guard
that can abort session start is worse than the failure it guards against):
  1. CLAUDE_CONFIG_DIR is set and lives under /projects
  2. ~/.claude is a symlink resolving into CLAUDE_CONFIG_DIR
     - missing or wrong symlink  -> repaired in place
     - REAL directory (a session already wrote ephemeral state) -> contents
       rescued to $CLAUDE_CONFIG_DIR/rescued-ephemeral/<utc-ts>/ first
  3. the config dir exists and is writable

Silent when everything is green; on any finding it prints a banner that
SessionStart injects into context so the session (and operator) see it
immediately instead of discovering the loss after the next pod restart.
"""

import os
import shutil
import sys
from datetime import datetime, timezone
from pathlib import Path

PVC_ROOT = "/projects"
EXPECTED_CONFIG = "/projects/.claude-config"


def main() -> int:
    findings: list[str] = []
    repairs: list[str] = []

    cfg = os.environ.get("CLAUDE_CONFIG_DIR", "")
    if not cfg:
        findings.append(
            "CLAUDE_CONFIG_DIR is UNSET — this session is writing to ~/.claude; "
            f"if that is not on {PVC_ROOT} the transcript will not survive a pod "
            f"restart. Expected: {EXPECTED_CONFIG} (set in devfile.yaml tools env)."
        )
        cfg = EXPECTED_CONFIG
    elif not cfg.startswith(PVC_ROOT + "/"):
        findings.append(
            f"CLAUDE_CONFIG_DIR={cfg} is OFF the persistent volume — session "
            f"state will be lost on pod restart. Expected {EXPECTED_CONFIG}."
        )

    cfg_path = Path(cfg)
    if not cfg_path.is_dir():
        findings.append(f"config dir {cfg} does not exist — transcripts have nowhere durable to land.")
    elif not os.access(cfg_path, os.W_OK):
        findings.append(f"config dir {cfg} is not writable — session writes will fail or divert.")

    home_claude = Path(os.environ.get("HOME", "/home/user")) / ".claude"
    try:
        if home_claude.is_symlink():
            target = home_claude.resolve()
            if cfg_path.is_dir() and target != cfg_path.resolve():
                home_claude.unlink()
                home_claude.symlink_to(cfg_path)
                repairs.append(f"~/.claude pointed at {target}; relinked to {cfg}.")
        elif home_claude.is_dir():
            # A session already wrote real state to ephemeral storage. Rescue it
            # onto the PVC before replacing the directory with the symlink —
            # this is the data the 2026-08-03 restart destroyed.
            stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
            rescue = cfg_path / "rescued-ephemeral" / stamp
            rescue.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(home_claude, rescue, symlinks=True, dirs_exist_ok=True)
            aside = home_claude.with_name(f".claude.pre-link.{stamp}")
            home_claude.rename(aside)
            home_claude.symlink_to(cfg_path)
            findings.append(
                f"~/.claude was a REAL directory on ephemeral storage — a session "
                f"wrote state outside the PVC. Contents rescued to {rescue}; "
                f"~/.claude now links to {cfg}. Inspect the rescue dir for lost "
                f"transcripts (projects/*/*.jsonl) before the next restart discards "
                f"the {aside.name} copy."
            )
        else:
            if cfg_path.is_dir():
                home_claude.symlink_to(cfg_path)
                repairs.append(f"~/.claude was missing; linked to {cfg}.")
        (home_claude / "ide").mkdir(parents=True, exist_ok=True)
    except OSError as exc:
        findings.append(f"could not verify/repair ~/.claude symlink: {exc}")

    if findings or repairs:
        print("[durability-guard] SESSION DURABILITY CHECK")
        for line in findings:
            print(f"[durability-guard] ⚠ {line}")
        for line in repairs:
            print(f"[durability-guard] repaired: {line}")
        if findings:
            print(
                "[durability-guard] Session state outside /projects does NOT survive "
                "pod restarts. Fix the wiring (devfile.yaml tools env + postStart) "
                "before doing substantive work in this session."
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())
