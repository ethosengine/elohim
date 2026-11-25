"""held-tool-ensure: every session's hook keeps this device's held-tool runner alive, and does nothing else.

Run: python3 -m unittest discover -s .claude/hooks/tests -p test_held_tool_ensure.py
"""
import fcntl
import os
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / '.claude/hooks/held-tool-ensure.py'
WATCHER = REPO / 'genesis/agentic/bin/held-tool-watch'
H = runpy.run_path(str(HOOK))


class EnsureTest(unittest.TestCase):
    """The hook's `ensure` against a scratch checkout; nothing is started in the real one."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.code = Path(tmp.name) / 'checkout'
        (self.code / 'genesis/agentic/bin').mkdir(parents=True)
        (self.code / 'genesis/agentic/bin/held-tool-watch').write_text('#!/usr/bin/env python3\n')
        self.started = []
        env = mock.patch.dict(os.environ, {'HELD_TOOL_WATCH': ''})
        env.start()
        self.addCleanup(env.stop)

    def start(self, watcher, code):
        self.started.append((watcher, code))

    def test_the_lock_is_the_one_the_runner_takes(self):
        self.assertEqual(H['LOCK_NAME'], runpy.run_path(str(WATCHER))['LOOP_LOCK'])
        self.assertEqual(H['lock_path'](self.code), self.code / '.eprfs/status' / H['LOCK_NAME'])

    def test_no_runner_starts_one_and_a_live_runner_starts_none(self):
        self.assertEqual(H['ensure'](self.code, self.start), 'started')
        self.assertEqual(self.started, [(self.code / 'genesis/agentic/bin/held-tool-watch', self.code)])
        fd = os.open(H['lock_path'](self.code), os.O_RDWR)
        self.addCleanup(os.close, fd)
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)  # a runner holds it
        for _ in range(3):
            self.assertEqual(H['ensure'](self.code, self.start), 'running')
        self.assertEqual(len(self.started), 1)

    def test_a_checkout_without_the_watcher_or_with_the_switch_off_starts_nothing(self):
        (self.code / 'genesis/agentic/bin/held-tool-watch').unlink()
        self.assertEqual(H['ensure'](self.code, self.start), 'absent')
        os.environ['HELD_TOOL_WATCH'] = 'off'
        self.assertEqual(H['ensure'](self.code, self.start), 'off')
        self.assertEqual(self.started, [])

    def test_its_own_error_is_one_stderr_line_and_exit_0(self):
        # A copy of the hook in a checkout whose status dir cannot be made: ensure raises inside.
        hooks = self.code / '.claude/hooks'
        hooks.mkdir(parents=True)
        (hooks / 'held-tool-ensure.py').write_bytes(HOOK.read_bytes())
        (self.code / '.eprfs').write_text('a file where a directory belongs\n')
        started = time.monotonic()
        out = subprocess.run([sys.executable, str(hooks / 'held-tool-ensure.py')], input='{}',
                             capture_output=True, text=True, timeout=10,
                             env={**os.environ, 'HELD_TOOL_WATCH': ''})
        self.assertLess(time.monotonic() - started, 2)
        self.assertEqual(out.returncode, 0)
        self.assertEqual(out.stdout, '')
        lines = out.stderr.splitlines()
        self.assertEqual(len(lines), 1, out.stderr)
        self.assertTrue(lines[0].startswith('held-tool-ensure: '), lines[0])


if __name__ == '__main__':
    unittest.main()
