"""held-tool-ensure: every session's hook keeps this device's held-tool runner alive, and does nothing else.

Run: python3 -m unittest discover -s .claude/hooks/tests -p test_held_tool_ensure.py
"""
import fcntl
import json
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

    def test_rule_9_the_next_hook_restarts_a_runner_after_its_lock_dies(self):
        lock = H['lock_path'](self.code)
        lock.parent.mkdir(parents=True)
        fd = os.open(lock, os.O_RDWR | os.O_CREAT, 0o644)
        fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        self.assertEqual(H['ensure'](self.code, self.start), 'running')
        os.close(fd)  # the runner died: its process-owned lock is released
        self.assertEqual(H['ensure'](self.code, self.start), 'started')
        self.assertEqual(len(self.started), 1)

    def test_a_checkout_without_the_watcher_starts_nothing(self):
        (self.code / 'genesis/agentic/bin/held-tool-watch').unlink()
        self.assertEqual(H['ensure'](self.code, self.start), 'absent')
        self.assertEqual(self.started, [])

    def test_the_off_switch_is_device_wide_until_a_session_turns_it_on(self):
        stop = H['lock_path'](self.code).with_name(H['STOP_NAME'])
        self.assertEqual(H['STOP_NAME'], runpy.run_path(str(WATCHER))['STOP_FILE'])
        os.environ['HELD_TOOL_WATCH'] = 'off'
        self.assertEqual(H['ensure'](self.code, self.start), 'off')
        self.assertTrue(stop.exists())  # the running loop reads this at its next pass and exits
        os.environ['HELD_TOOL_WATCH'] = ''
        self.assertEqual(H['ensure'](self.code, self.start), 'off')
        self.assertEqual(self.started, [])
        os.environ['HELD_TOOL_WATCH'] = 'on'
        self.assertEqual(H['ensure'](self.code, self.start), 'started')
        self.assertFalse(stop.exists())

    def test_a_standing_stop_file_is_named_at_every_session_start(self):
        hooks = self.code / '.claude/hooks'
        hooks.mkdir(parents=True)
        (hooks / 'held-tool-ensure.py').write_bytes(HOOK.read_bytes())
        stop = self.code / '.eprfs/status' / H['STOP_NAME']
        stop.parent.mkdir(parents=True)
        stop.touch()

        def run(event):
            return subprocess.run([sys.executable, str(hooks / 'held-tool-ensure.py')],
                                  input=json.dumps({'hook_event_name': event}), capture_output=True,
                                  text=True, timeout=10, env={**os.environ, 'HELD_TOOL_WATCH': ''})
        start = run('SessionStart')
        self.assertEqual((start.returncode, start.stdout), (0, ''))
        self.assertEqual(start.stderr.splitlines(),
                         [f'held-tool watch is off on this device: remove {stop} or set HELD_TOOL_WATCH=on'])
        self.assertEqual(run('PostToolUse').stderr, '')  # once per session, not per tool call
        stop.unlink()
        self.assertEqual(run('SessionStart').stderr, '')

    def test_the_registered_commands_are_silent_and_exit_0_without_python_or_the_file(self):
        settings = json.loads((REPO / '.claude/settings.json').read_text())
        codex = json.loads((REPO / '.codex/hooks.json').read_text())
        commands = [h['command'] for config in (settings, codex) for groups in config['hooks'].values()
                    for group in groups for h in group['hooks'] if 'held-tool-ensure' in h['command']]
        self.assertEqual(len(commands), 4)
        tools = self.code / 'tools'
        tools.mkdir()
        (tools / 'python3').symlink_to(sys.executable)
        git = tools / 'git'
        git.write_text('#!/bin/sh\nexit 1\n')  # this scratch checkout deliberately has no repository
        git.chmod(0o755)
        for command in commands:
            for path in ('/nonexistent', str(tools)):  # no interpreter; interpreter but an absent hook file
                with self.subTest(command=command[:40], path=path[:12]):
                    out = subprocess.run(['/bin/sh', '-c', command], capture_output=True, text=True,
                                         cwd=str(self.code), timeout=10,
                                         env={'PATH': path, 'CLAUDE_PROJECT_DIR': str(self.code)})
                    self.assertEqual((out.returncode, out.stdout, out.stderr), (0, '', ''))

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
