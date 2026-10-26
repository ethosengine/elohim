from concurrent.futures import ThreadPoolExecutor
import fcntl
import json
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys
import tempfile
import threading
import time
import unittest


REPO = Path(__file__).resolve().parents[3]
HOOK = REPO / '.claude/hooks/lsp-edit-event.py'
MODULE = runpy.run_path(str(HOOK))


class EditEventTest(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name) / 'main'
        # Inside a git hook (pre-push runs this file) git exports GIT_DIR and friends; inherited,
        # they would point `git init` and the hook's rev-parse at the real repository.
        clean = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
        subprocess.run(['git', 'init', '-q', str(self.root)], check=True, env=clean)
        self.log = self.root / '.eprfs/status/lsp/edits.jsonl'
        self.stub = self.root / 'epr-stub'
        self.stub.write_text('#!/usr/bin/env python3\nraise SystemExit(1)\n')
        self.stub.chmod(0o755)
        self.env = {**clean, 'CLAUDE_PROJECT_DIR': str(self.root),
                    'EPR_BIN': str(self.stub), 'ELOHIM_SESSION_ID': 'env-session',
                    'ELOHIM_SESSION_OWNER_PID': str(os.getpid())}

    def hook(self, payload, *, env=None):
        result = subprocess.run([sys.executable, str(HOOK)], cwd=self.root,
                                input=json.dumps(payload), text=True, capture_output=True,
                                env=env or self.env, timeout=2)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, '')
        self.assertEqual(result.stderr, '')

    def payload(self, tool='Edit', **extra):
        return {'tool_name': tool, 'session_id': 'payload-session',
                'tool_input': {'file_path': 'src/example.rs'}, **extra}

    def events(self):
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def test_all_edit_tools_append_c2_records(self):
        for tool in ('Edit', 'Write', 'MultiEdit', 'NotebookEdit'):
            data = self.payload(tool)
            if tool == 'NotebookEdit':
                data['tool_input'] = {'notebook_path': 'notebook.ipynb'}
            self.hook(data)
        events = self.events()
        self.assertEqual(len(events), 4)
        for row in events:
            self.assertEqual(set(row), {'v', 'at', 'session', 'ownerPid', 'ownerStartTime', 'path', 'tool'})
            self.assertEqual(row['v'], 1)
            self.assertEqual(row['session'], 'payload-session')
            self.assertEqual(row['ownerPid'], os.getpid())
            self.assertEqual(row['ownerStartTime'], MODULE['process_start'](os.getpid()))
            self.assertRegex(row['at'], r'^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$')
            self.assertTrue(Path(row['path']).is_absolute())
        self.assertEqual(events[-1]['path'], str(self.root / 'notebook.ipynb'))

    def test_native_session_wins(self):
        identity = {'runtime': 'claude', 'session': 'native', 'ownerPid': 123,
                    'ownerStartTime': '456', 'source': 'declared'}
        self.stub.write_text('#!/usr/bin/env python3\nimport sys\n'
                             'assert sys.argv[1:] == ["session", "id", "--json"]\n'
                             f'print({json.dumps(identity)!r})\n')
        self.hook(self.payload())
        self.assertEqual(self.events()[0]['session'], 'native')
        self.assertEqual(self.events()[0]['ownerPid'], 123)
        self.assertEqual(self.events()[0]['ownerStartTime'], '456')

    def test_timeout_falls_back_within_hook_budget(self):
        self.stub.write_text('#!/usr/bin/env python3\nimport time\ntime.sleep(10)\n')
        start = time.monotonic()
        self.hook(self.payload())
        self.assertLess(time.monotonic() - start, 2)
        self.assertEqual(self.events()[0]['session'], 'payload-session')

    def test_environment_then_anonymous_fallback(self):
        data = self.payload()
        del data['session_id']
        self.hook(data)
        self.assertEqual(self.events()[0]['session'], 'env-session')
        env = {k: v for k, v in self.env.items()
               if k not in ('ELOHIM_SESSION_ID', 'ELOHIM_SESSION_OWNER_PID')}
        self.hook(data, env=env)
        row = self.events()[1]
        self.assertEqual(row['session'], f'anon-{row["ownerPid"]}')

    def test_worktree_uses_main_checkout_log_and_worktree_file_path(self):
        subprocess.run(['git', '-C', str(self.root), '-c', 'user.name=Fixture',
                        '-c', 'user.email=fixture@example.test', 'commit', '--allow-empty', '-qm', 'fixture'], check=True, env=self.env)
        worktree = self.root.parent / 'worktree'
        subprocess.run(['git', '-C', str(self.root), 'worktree', 'add', '-q', '-b', 'fixture', str(worktree)], check=True, env=self.env)
        self.hook(self.payload(cwd=str(worktree)))
        self.assertEqual(self.events()[0]['path'], str(worktree / 'src/example.rs'))
        self.assertFalse((worktree / '.eprfs').exists())

    def test_prune_retains_last_ten_thousand_lines(self):
        self.log.parent.mkdir(parents=True)
        with self.log.open('w') as stream:
            for i in range(11_000):
                stream.write(json.dumps({'index': i, 'padding': 'x' * 200}) + '\n')
        self.assertGreater(self.log.stat().st_size, 2 * 1024 * 1024)
        self.hook(self.payload())
        rows = self.events()
        self.assertEqual(len(rows), 10_000)
        self.assertEqual(rows[0]['index'], 1001)
        self.assertEqual(rows[-1]['session'], 'payload-session')

    def test_simultaneous_writers_preserve_each_line(self):
        barrier = threading.Barrier(16)

        def append(index):
            barrier.wait(timeout=3)
            MODULE['append_event'](self.log, {'index': index})

        with ThreadPoolExecutor(max_workers=16) as writers:
            list(writers.map(append, range(16)))
        self.assertEqual(sorted(row['index'] for row in self.events()), list(range(16)))

    def test_invalid_payloads_and_locked_log_fail_open(self):
        for payload in ({}, [], self.payload(tool_input={}), self.payload(tool='Bash')):
            self.hook(payload)
        self.assertFalse(self.log.exists())
        self.log.parent.mkdir(parents=True)
        with self.log.open('a') as stream:
            fcntl.flock(stream, fcntl.LOCK_EX)
            self.hook(self.payload())
        self.assertEqual(self.log.read_text(), '')
        result = subprocess.run([sys.executable, str(HOOK)], input='{', text=True,
                                capture_output=True, env=self.env, timeout=2)
        self.assertEqual((result.returncode, result.stdout, result.stderr), (0, '', ''))

    def test_settings_register_every_edit_tool_with_two_second_timeout(self):
        settings = json.loads((REPO / '.claude/settings.json').read_text())
        for tool in ('Edit', 'Write', 'MultiEdit', 'NotebookEdit'):
            hooks = [hook for group in settings['hooks']['PostToolUse']
                     if re.fullmatch(group.get('matcher', ''), tool)
                     for hook in group['hooks'] if 'lsp-edit-event.py' in hook.get('command', '')]
            self.assertEqual(len(hooks), 1)
            self.assertEqual(hooks[0]['timeout'], 2)


if __name__ == '__main__':
    unittest.main()
