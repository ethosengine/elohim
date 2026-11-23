#!/usr/bin/env python3
"""Regression tests for the package-projected device-arrive shell hook."""
from __future__ import annotations

import os
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path


HOOK = Path(__file__).resolve().parent.parent / "device-arrive.sh"
NODE_STUB = """#!/bin/sh
printf '%s|%s|%s\\n' "${GATE_LIVE_RUN-<unset>}" "${EPR_BIN-<unset>}" "$*" >> "$NODE_LOG"
exit "${PREFLIGHT_EXIT:-0}"
"""
EPR_STUB = """#!/bin/sh
printf '%s|%s\\n' "${EPR_BIN-<unset>}" "$*" >> "$EPR_LOG"
exit 0
"""


class DeviceArriveTest(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp(prefix="device-arrive-"))
        self.addCleanup(shutil.rmtree, self.tmp, True)
        self.project = self.tmp / "project"
        hook_dir = self.project / ".claude" / "hooks"
        hook_dir.mkdir(parents=True)
        shutil.copy(HOOK, hook_dir / "device-arrive.sh")
        preflight = self.project / "genesis" / "orchestrator" / "gate-preflight.mjs"
        preflight.parent.mkdir(parents=True)
        preflight.write_text("// handled by the node stub\n")

        self.bin = self.tmp / "bin"
        self.bin.mkdir()
        self.node = self.bin / "node"
        self.epr = self.bin / "epr"
        for path, body in ((self.node, NODE_STUB), (self.epr, EPR_STUB)):
            path.write_text(body)
            path.chmod(0o755)
        self.node_log = self.tmp / "node.log"
        self.epr_log = self.tmp / "epr.log"

    def run_hook(self, *, preflight_exit: int, epr_bin: str, env_file: Path | None):
        env = dict(os.environ)
        env.update({
            "PATH": f"{self.bin}:{env['PATH']}",
            "EPR_BIN": epr_bin,
            "GATE_LIVE_RUN": "must-not-reach-preflight",
            "PREFLIGHT_EXIT": str(preflight_exit),
            "NODE_LOG": str(self.node_log),
            "EPR_LOG": str(self.epr_log),
        })
        if env_file is None:
            env.pop("CLAUDE_ENV_FILE", None)
        else:
            env["CLAUDE_ENV_FILE"] = str(env_file)
        return subprocess.run(
            ["sh", str(self.project / ".claude" / "hooks" / "device-arrive.sh")],
            capture_output=True,
            text=True,
            env=env,
            timeout=30,
        )

    def test_refused_unsets_for_hook_and_claude_session(self):
        env_file = self.tmp / "claude.env"
        env_file.write_text("KEEP=1\n")
        old = "/fixture/old/epr"

        result = self.run_hook(preflight_exit=1, epr_bin=old, env_file=env_file)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(env_file.read_text(), "KEEP=1\nunset EPR_BIN\n")
        self.assertEqual(
            result.stdout,
            f'[device] EPR_BIN={old} is older than this device\'s head epr; unset for this session. '
            'The terminal that started this session still exports it: run "unset EPR_BIN" there, '
            'or open a new terminal.\n',
        )
        calls = self.epr_log.read_text().splitlines()
        self.assertEqual([line.split("|", 1)[0] for line in calls], ["<unset>"] * 3)
        self.assertEqual(
            [line.split("|", 1)[1] for line in calls],
            ["bin status --tool epr --json", "device arrive", "session open"],
        )
        self.assertTrue(self.node_log.read_text().startswith("<unset>|/fixture/old/epr|"))

    def test_ok_leaves_environment_and_output_unchanged(self):
        env_file = self.tmp / "claude.env"
        env_file.write_text("KEEP=1\n")

        result = self.run_hook(preflight_exit=0, epr_bin=str(self.epr), env_file=env_file)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(env_file.read_text(), "KEEP=1\n")
        calls = self.epr_log.read_text().splitlines()
        self.assertEqual([line.split("|", 1)[0] for line in calls], [str(self.epr)] * 3)
        self.assertTrue(self.node_log.read_text().startswith(f"<unset>|{self.epr}|"))

    def test_refused_without_claude_env_file_still_reports_and_succeeds(self):
        old = "/fixture/old/epr"

        result = self.run_hook(preflight_exit=1, epr_bin=old, env_file=None)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(f"[device] EPR_BIN={old} is older than this device's head epr", result.stdout)
        self.assertEqual(len(result.stdout.splitlines()), 1)
        self.assertEqual(
            [line.split("|", 1)[0] for line in self.epr_log.read_text().splitlines()],
            ["<unset>"] * 3,
        )

    def test_check_that_cannot_run_leaves_epr_bin_alone(self):
        # Exit 1 is REFUSED; any other failure (node missing is 127, a crash) proves nothing.
        env_file = self.tmp / "claude.env"
        env_file.write_text("KEEP=1\n")

        result = self.run_hook(preflight_exit=127, epr_bin=str(self.epr), env_file=env_file)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertEqual(env_file.read_text(), "KEEP=1\n")
        self.assertEqual(
            [line.split("|", 1)[0] for line in self.epr_log.read_text().splitlines()],
            [str(self.epr)] * 3,
        )


if __name__ == "__main__":
    unittest.main()
