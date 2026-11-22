"""All hook state and child binaries belong to temporary directories."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock

HOOK = Path(__file__).resolve().parents[1] / 'berth-writes-observe.py'
spec = importlib.util.spec_from_file_location('writes_observe', HOOK)
hook = importlib.util.module_from_spec(spec)
spec.loader.exec_module(hook)


class ObserveTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.binary = self.root / 'berth'
        self.log = self.root / 'calls'
        self.binary.write_text('#!/usr/bin/env python3\nimport sys\nwith open(' + repr(str(self.log)) +
                               ', "a") as f: f.write(repr(sys.argv[1:]) + "\\n")\n')
        self.binary.chmod(0o755)
        self.env = {**os.environ, 'CLAUDE_PROJECT_DIR': str(self.root),
                    'BERTH_WRITES_BERTH_BIN': str(self.binary)}

    def call(self, path='src/a', agent=None, tool='Edit'):
        payload = {'session_id': 'parent', 'tool_name': tool, 'tool_input': {'file_path': path}}
        if agent:
            payload['agent_id'] = agent
        result = subprocess.run([sys.executable, str(HOOK)], input=json.dumps(payload),
                                text=True, capture_output=True, env=self.env, timeout=3)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))
        return result

    def test_throttle_and_new_path_exception_and_parent_attribution(self):
        self.call(agent='child')
        self.call(agent='other-child')
        self.call('src/b', tool='Write')
        lines = self.log.read_text().splitlines()
        self.assertEqual(len(lines), 2)
        self.assertIn("'observed', '--session', 'parent'", lines[0])
        self.assertNotIn('child', lines[0])
        self.assertIn(str(self.root / 'src/b'), lines[1])
        state = next((self.root / '.eprfs/status/berth-writes').glob('*.json'))
        data = json.loads(state.read_text())
        data['checked_at'] -= 11
        state.write_text(json.dumps(data))
        self.call('src/a')
        self.assertEqual(len(self.log.read_text().splitlines()), 3)

    def test_missing_berth_fail_open(self):
        self.binary.unlink()
        self.call()
        self.call()

    def test_slow_berth_fail_open_under_deadline(self):
        self.binary.write_text('#!/usr/bin/env python3\nimport time\ntime.sleep(20)\n')
        started = time.monotonic()
        self.call()
        self.assertLess(time.monotonic() - started, 2.2)

    def test_notebook_and_invalid_payload(self):
        with mock.patch.object(hook, 'ROOT', self.root), mock.patch.dict(os.environ, self.env):
            hook.observe({'session_id': 'parent', 'tool_name': 'NotebookEdit',
                          'tool_input': {'notebook_path': 'test.ipynb'}, 'agent_id': 'child'})
            hook.observe({'session_id': '../bad', 'tool_name': 'Edit', 'tool_input': {'file_path': 'x'}})
            hook.observe({'session_id': 'parent', 'tool_name': 'Read', 'tool_input': {'file_path': 'x'}})
            hook.observe(None)
        self.assertEqual(len(self.log.read_text().splitlines()), 1)
        self.assertIn('test.ipynb', self.log.read_text())

    def test_linked_checkout_state_is_main_checkout_state(self):
        worktree = self.root / '.worktrees/one'
        worktree.mkdir(parents=True)
        gitdir = self.root / '.git/worktrees/one'
        gitdir.mkdir(parents=True)
        (gitdir / 'commondir').write_text('../..')
        (worktree / '.git').write_text('gitdir: ' + str(gitdir))
        self.assertEqual(hook.state_root(worktree), self.root / '.eprfs/status/berth-writes')


if __name__ == '__main__':
    unittest.main()
